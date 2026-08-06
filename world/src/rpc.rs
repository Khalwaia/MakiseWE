use std::collections::BTreeMap;
use std::pin::Pin;

use makise_proto::v1::world_service_server::WorldService;
use makise_proto::v1::{
    ActivityView as ProtoActivityView, Affordance as ProtoAffordance,
    CommandEnvelope as ProtoCommandEnvelope, CommandResult as ProtoCommandResult,
    CommandStatus as ProtoCommandStatus, ErrorDetail, EventEnvelope as ProtoEventEnvelope,
    GetCommandResultRequest, GetPerceptionRequest, HandshakeRequest, HandshakeResponse,
    HealthResponse, HealthStatus as ProtoHealthStatus, ObservedObject as ProtoObservedObject,
    PerceptionWindow as ProtoPerceptionWindow, SubscribeEventsRequest, command_envelope,
};
use prost_types::{Duration as ProtoDuration, Timestamp, value::Kind};
use tokio::sync::mpsc;
use tokio_stream::{Stream, wrappers::ReceiverStream};
use tonic::{Request, Response, Status};

use crate::{
    ActorError, Affordance, CommandEnvelope, CommandPayload, CommandResult, CommandStatus,
    EventBatch, HealthSnapshot, PerceptionWindow, PersistedEvent, TimeStatus, WorldActorHandle,
};

const PROTOCOL_VERSION: u32 = 1;
const PRIMARY_AGENT_ID: &str = "makise";
const EVENT_STREAM_BUFFER: usize = 64;

#[derive(Clone)]
pub struct WorldRpc {
    actor: WorldActorHandle,
}

impl WorldRpc {
    pub fn new(actor: WorldActorHandle) -> Self {
        Self { actor }
    }
}

type EventStream =
    Pin<Box<dyn Stream<Item = std::result::Result<ProtoEventEnvelope, Status>> + Send + 'static>>;

#[tonic::async_trait]
impl WorldService for WorldRpc {
    type SubscribeEventsStream = EventStream;

    async fn handshake(
        &self,
        request: Request<HandshakeRequest>,
    ) -> std::result::Result<Response<HandshakeResponse>, Status> {
        let request = request.into_inner();
        if request.client_name.trim().is_empty() {
            return Err(Status::invalid_argument("client_name is required"));
        }
        if request.min_protocol_version > PROTOCOL_VERSION
            || request.max_protocol_version < PROTOCOL_VERSION
        {
            return Err(Status::failed_precondition(
                "client and server protocol ranges do not overlap",
            ));
        }

        let health = self.actor.health().await.map_err(actor_status)?;
        if !request.expected_identity_id.is_empty()
            && request.expected_identity_id != health.identity_id
        {
            return Err(Status::failed_precondition("identity_id mismatch"));
        }
        if !request.expected_world_definition_hash.is_empty()
            && request.expected_world_definition_hash != health.world_definition_hash
        {
            return Err(Status::failed_precondition(
                "world_definition_hash mismatch",
            ));
        }

        Ok(Response::new(HandshakeResponse {
            selected_protocol_version: PROTOCOL_VERSION,
            identity_id: health.identity_id,
            world_definition_hash: health.world_definition_hash,
            world_version: health.world_version,
            last_event_seq: health.last_event_seq,
            server_capabilities: vec![
                "command-deduplication".into(),
                "event-replay".into(),
                "bounded-backpressure".into(),
                "perform-parameters-v1".into(),
                "dynamic-object-placement".into(),
            ],
        }))
    }

    async fn execute_command(
        &self,
        request: Request<ProtoCommandEnvelope>,
    ) -> std::result::Result<Response<ProtoCommandResult>, Status> {
        let command = command_from_proto(request.into_inner())?;
        let result = self.actor.execute(command).await.map_err(actor_status)?;
        Ok(Response::new(command_result_to_proto(result)))
    }

    async fn get_command_result(
        &self,
        request: Request<GetCommandResultRequest>,
    ) -> std::result::Result<Response<ProtoCommandResult>, Status> {
        let command_id = request.into_inner().command_id;
        if command_id.is_empty() {
            return Err(Status::invalid_argument("command_id is required"));
        }
        let result = self
            .actor
            .command_result(command_id)
            .await
            .map_err(actor_status)?
            .ok_or_else(|| Status::not_found("command result was not found"))?;
        Ok(Response::new(command_result_to_proto(result)))
    }

    async fn get_perception(
        &self,
        request: Request<GetPerceptionRequest>,
    ) -> std::result::Result<Response<ProtoPerceptionWindow>, Status> {
        let request = request.into_inner();
        if request.agent_id != PRIMARY_AGENT_ID {
            return Err(Status::permission_denied("agent_id mismatch"));
        }
        let perception = self.actor.perception().await.map_err(actor_status)?;
        Ok(Response::new(perception_to_proto(perception)))
    }

    async fn subscribe_events(
        &self,
        request: Request<SubscribeEventsRequest>,
    ) -> std::result::Result<Response<Self::SubscribeEventsStream>, Status> {
        let after_seq = request.into_inner().after_seq;
        let live = self.actor.subscribe_events();
        let initial = self
            .actor
            .events_after(after_seq)
            .await
            .map_err(actor_status)?;
        if after_seq > initial.head_seq {
            return Err(Status::invalid_argument(
                "after_seq is ahead of the durable event log",
            ));
        }

        let (sender, receiver) = mpsc::channel(EVENT_STREAM_BUFFER);
        let actor = self.actor.clone();
        tokio::spawn(stream_events(actor, live, initial, after_seq, sender));
        Ok(Response::new(Box::pin(ReceiverStream::new(receiver))))
    }

    async fn health(
        &self,
        _request: Request<()>,
    ) -> std::result::Result<Response<HealthResponse>, Status> {
        let health = self.actor.health().await.map_err(actor_status)?;
        Ok(Response::new(health_to_proto(health)))
    }
}

async fn stream_events(
    actor: WorldActorHandle,
    mut live: tokio::sync::broadcast::Receiver<PersistedEvent>,
    initial: EventBatch,
    mut last_seq: u64,
    sender: mpsc::Sender<std::result::Result<ProtoEventEnvelope, Status>>,
) {
    if !forward_batch(&sender, initial.events, &mut last_seq).await {
        return;
    }

    loop {
        match live.recv().await {
            Ok(event) if event.event_seq <= last_seq => {}
            Ok(event) if event.event_seq == last_seq.saturating_add(1) => {
                if !forward_event(&sender, event, &mut last_seq).await {
                    return;
                }
            }
            Ok(_) | Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                let recovered = match actor.events_after(last_seq).await {
                    Ok(batch) => batch,
                    Err(error) => {
                        let _ = sender.send(Err(actor_status(error))).await;
                        return;
                    }
                };
                if !forward_batch(&sender, recovered.events, &mut last_seq).await {
                    return;
                }
            }
            Err(tokio::sync::broadcast::error::RecvError::Closed) => return,
        }
    }
}

async fn forward_batch(
    sender: &mpsc::Sender<std::result::Result<ProtoEventEnvelope, Status>>,
    events: Vec<PersistedEvent>,
    last_seq: &mut u64,
) -> bool {
    for event in events {
        if event.event_seq <= *last_seq {
            continue;
        }
        if event.event_seq != last_seq.saturating_add(1) {
            let _ = sender
                .send(Err(Status::data_loss("durable event sequence has a gap")))
                .await;
            return false;
        }
        if !forward_event(sender, event, last_seq).await {
            return false;
        }
    }
    true
}

async fn forward_event(
    sender: &mpsc::Sender<std::result::Result<ProtoEventEnvelope, Status>>,
    event: PersistedEvent,
    last_seq: &mut u64,
) -> bool {
    let event_seq = event.event_seq;
    let message = event_to_proto(event).map_err(|error| Status::internal(error.to_string()));
    if sender.send(message).await.is_err() {
        return false;
    }
    *last_seq = event_seq;
    true
}

fn command_from_proto(
    message: ProtoCommandEnvelope,
) -> std::result::Result<CommandEnvelope, Status> {
    let issued_at_ms = timestamp_to_ms(
        message
            .issued_at
            .as_ref()
            .ok_or_else(|| Status::invalid_argument("issued_at is required"))?,
    )?;
    let ttl_ms = duration_to_ms(
        message
            .ttl
            .as_ref()
            .ok_or_else(|| Status::invalid_argument("ttl is required"))?,
    )?;
    let payload = match message
        .payload
        .ok_or_else(|| Status::invalid_argument("command payload is required"))?
    {
        command_envelope::Payload::MoveTo(move_to) => CommandPayload::MoveTo {
            target_anchor_id: move_to.target_anchor_id,
        },
        command_envelope::Payload::Perform(perform) => CommandPayload::Perform {
            action_id: perform.action_id,
            target_id: perform.target_id,
            parameters: parse_string_parameters(perform.parameters)?,
        },
        command_envelope::Payload::Inspect(inspect) => CommandPayload::Inspect {
            target_id: inspect.target_id,
        },
        command_envelope::Payload::ManagePlan(_)
        | command_envelope::Payload::WaitUntil(_)
        | command_envelope::Payload::Phone(_) => {
            return Err(Status::invalid_argument(
                "command namespace is not implemented by makise-world",
            ));
        }
    };

    Ok(CommandEnvelope {
        command_id: message.command_id,
        identity_id: message.identity_id,
        agent_id: message.agent_id,
        expected_world_version: message.expected_world_version,
        schema_version: message.schema_version,
        decision_id: message.decision_id,
        issued_at_ms,
        ttl_ms,
        payload,
    })
}

fn parse_string_parameters(
    parameters: Option<prost_types::Struct>,
) -> std::result::Result<BTreeMap<String, String>, Status> {
    let Some(parameters) = parameters else {
        return Ok(BTreeMap::new());
    };
    if parameters.fields.len() > 16 {
        return Err(Status::invalid_argument(
            "world.perform accepts at most 16 parameters",
        ));
    }
    parameters
        .fields
        .into_iter()
        .map(|(key, value)| {
            if key.is_empty()
                || key.len() > 96
                || !key.bytes().all(|byte| {
                    byte.is_ascii_lowercase()
                        || byte.is_ascii_digit()
                        || matches!(byte, b'_' | b'.' | b'-')
                })
            {
                return Err(Status::invalid_argument("invalid parameter name"));
            }
            let Some(Kind::StringValue(value)) = value.kind else {
                return Err(Status::invalid_argument(
                    "world.perform parameter values must be strings",
                ));
            };
            if value.len() > 256 {
                return Err(Status::invalid_argument("parameter value is too long"));
            }
            Ok((key, value))
        })
        .collect()
}

fn command_result_to_proto(result: CommandResult) -> ProtoCommandResult {
    let error = result.error_code.map(|code| ErrorDetail {
        code,
        message: result.error_message.unwrap_or_default(),
        fields: Default::default(),
    });
    ProtoCommandResult {
        command_id: result.command_id,
        status: command_status_to_proto(result.status) as i32,
        committed_world_version: result.committed_world_version,
        first_event_seq: result.first_event_seq,
        last_event_seq: result.last_event_seq,
        error,
        suggested_recovery: result
            .suggested_recovery
            .into_iter()
            .map(affordance_to_proto)
            .collect(),
    }
}

fn command_status_to_proto(status: CommandStatus) -> ProtoCommandStatus {
    match status {
        CommandStatus::Committed => ProtoCommandStatus::Committed,
        CommandStatus::AlreadyCommitted => ProtoCommandStatus::AlreadyCommitted,
        CommandStatus::RejectedPrecondition => ProtoCommandStatus::RejectedPrecondition,
        CommandStatus::ResourceConflict => ProtoCommandStatus::ResourceConflict,
        CommandStatus::StaleWorld => ProtoCommandStatus::StaleWorld,
        CommandStatus::ExpiredDecision => ProtoCommandStatus::ExpiredDecision,
        CommandStatus::Unauthorized => ProtoCommandStatus::Unauthorized,
        CommandStatus::InvalidArgument => ProtoCommandStatus::InvalidArgument,
        CommandStatus::TemporarilyUnavailable => ProtoCommandStatus::TemporarilyUnavailable,
        CommandStatus::InternalError => ProtoCommandStatus::InternalError,
    }
}

fn event_to_proto(event: PersistedEvent) -> crate::Result<ProtoEventEnvelope> {
    let event_type = event.event_type().into();
    let payload_json = event.payload_json()?;
    Ok(ProtoEventEnvelope {
        event_id: event.event_id,
        event_seq: event.event_seq,
        world_version: event.world_version,
        event_schema_version: event.event_schema_version,
        occurred_at: Some(ms_to_timestamp(event.occurred_at_ms)),
        causation_command_id: event.causation_command_id,
        correlation_id: None,
        event_type,
        payload_json,
    })
}

fn perception_to_proto(perception: PerceptionWindow) -> ProtoPerceptionWindow {
    ProtoPerceptionWindow {
        perception_id: perception.perception_id,
        world_version: perception.world_version,
        location_id: perception.location_id,
        anchor_id: perception.anchor_id,
        qualitative_body_state: Vec::new(),
        observed_objects: perception
            .observed_objects
            .into_iter()
            .map(|object| ProtoObservedObject {
                object_id: object.object_id,
                name: object.name,
                observed_properties: object.observed_properties.into_iter().collect(),
                affordances: object
                    .affordances
                    .into_iter()
                    .map(affordance_to_proto)
                    .collect(),
            })
            .collect(),
        affordances: perception
            .affordances
            .into_iter()
            .map(affordance_to_proto)
            .collect(),
        activities: perception
            .activities
            .into_iter()
            .map(|activity| ProtoActivityView {
                activity_id: activity.activity_id,
                action_id: activity.action_id,
                completes_at: Some(ms_to_timestamp(activity.completes_at_ms)),
                reserved_resources: activity.reserved_resources,
            })
            .collect(),
        unread_notification_count: 0,
        significant_changes: perception.significant_changes,
        environment_cues: perception.environment_cues,
    }
}

fn affordance_to_proto(affordance: Affordance) -> ProtoAffordance {
    ProtoAffordance {
        action_id: affordance.action_id,
        target_id: affordance.target_id,
        description: affordance.description,
        duration: Some(ms_to_duration(affordance.duration_ms)),
        required_resources: affordance.required_resources,
        parameters_schema_json: affordance.parameters_schema_json,
    }
}

fn health_to_proto(health: HealthSnapshot) -> HealthResponse {
    let ready = health.last_error.is_none() && health.time_status == TimeStatus::Normal;
    let status = match health.time_status {
        TimeStatus::Normal if ready => ProtoHealthStatus::Healthy,
        TimeStatus::Normal => ProtoHealthStatus::Degraded,
        TimeStatus::TimeAnomaly => ProtoHealthStatus::TimeAnomaly,
        TimeStatus::SafeStop => ProtoHealthStatus::SafeStop,
    };
    HealthResponse {
        status: status as i32,
        live: true,
        ready,
        version: env!("CARGO_PKG_VERSION").into(),
        world_version: health.world_version,
        last_event_seq: health.last_event_seq,
    }
}

fn actor_status(error: ActorError) -> Status {
    match error {
        ActorError::Busy => Status::resource_exhausted(error.to_string()),
        ActorError::Stopped | ActorError::Clock(_) => Status::unavailable(error.to_string()),
        ActorError::World(_) => Status::internal(error.to_string()),
    }
}

fn timestamp_to_ms(timestamp: &Timestamp) -> std::result::Result<i64, Status> {
    if !(0..1_000_000_000).contains(&timestamp.nanos) || timestamp.nanos % 1_000_000 != 0 {
        return Err(Status::invalid_argument(
            "timestamp nanos must be an exact non-negative millisecond value",
        ));
    }
    timestamp
        .seconds
        .checked_mul(1_000)
        .and_then(|seconds| seconds.checked_add(i64::from(timestamp.nanos / 1_000_000)))
        .ok_or_else(|| Status::invalid_argument("timestamp is out of range"))
}

fn duration_to_ms(duration: &ProtoDuration) -> std::result::Result<i64, Status> {
    if duration.seconds < 0
        || !(0..1_000_000_000).contains(&duration.nanos)
        || duration.nanos % 1_000_000 != 0
    {
        return Err(Status::invalid_argument(
            "duration must be positive and use exact milliseconds",
        ));
    }
    duration
        .seconds
        .checked_mul(1_000)
        .and_then(|seconds| seconds.checked_add(i64::from(duration.nanos / 1_000_000)))
        .filter(|milliseconds| *milliseconds > 0)
        .ok_or_else(|| Status::invalid_argument("duration is zero or out of range"))
}

fn ms_to_timestamp(milliseconds: i64) -> Timestamp {
    Timestamp {
        seconds: milliseconds.div_euclid(1_000),
        nanos: i32::try_from(milliseconds.rem_euclid(1_000) * 1_000_000)
            .expect("millisecond remainder always fits in i32"),
    }
}

fn ms_to_duration(milliseconds: i64) -> ProtoDuration {
    ProtoDuration {
        seconds: milliseconds.div_euclid(1_000),
        nanos: i32::try_from(milliseconds.rem_euclid(1_000) * 1_000_000)
            .expect("millisecond remainder always fits in i32"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_sub_millisecond_timestamp() {
        let error = timestamp_to_ms(&Timestamp {
            seconds: 1,
            nanos: 1,
        })
        .unwrap_err();
        assert_eq!(error.code(), tonic::Code::InvalidArgument);
    }

    #[test]
    fn negative_timestamp_round_trips() {
        let timestamp = ms_to_timestamp(-1);
        assert_eq!(timestamp_to_ms(&timestamp).unwrap(), -1);
    }

    #[test]
    fn perform_parameters_accept_string_values() {
        let parameters = prost_types::Struct {
            fields: BTreeMap::from([
                (
                    "relation".into(),
                    prost_types::Value {
                        kind: Some(Kind::StringValue("container".into())),
                    },
                ),
                (
                    "slot_id".into(),
                    prost_types::Value {
                        kind: Some(Kind::StringValue("drawer".into())),
                    },
                ),
            ]),
        };
        let parsed = parse_string_parameters(Some(parameters)).unwrap();
        assert_eq!(parsed["relation"], "container");
        assert_eq!(parsed["slot_id"], "drawer");
    }

    #[test]
    fn perform_parameters_reject_non_string_values() {
        let parameters = prost_types::Struct {
            fields: BTreeMap::from([(
                "relation".into(),
                prost_types::Value {
                    kind: Some(Kind::BoolValue(true)),
                },
            )]),
        };
        let error = parse_string_parameters(Some(parameters)).unwrap_err();
        assert_eq!(error.code(), tonic::Code::InvalidArgument);
    }
}
