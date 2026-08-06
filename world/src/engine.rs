use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use crate::WorldDefinition;
use crate::domain::{
    Activity, ActivityCompletion, COMMAND_SCHEMA_VERSION, ClockSample, CommandEnvelope,
    CommandPayload, CommandResult, CommandStatus, DomainEvent, PerceptionWindow, PersistedEvent,
    Resource, TimeStatus, WorldState,
};
use crate::store::{EventStore, digest};
use crate::{ObjectPlacement, PathGuard, PlacementRelation, Result, WorldError};

const CLOCK_CHECKPOINT_INTERVAL_MS: i64 = 5_000;
const MIN_RECORDED_DOWNTIME_MS: i64 = 1_000;
const MAX_CLOCK_DRIFT_MS: i64 = 60_000;
const MAX_COMMAND_TTL_MS: i64 = 300_000;
const PRIMARY_AGENT_ID: &str = "makise";

pub struct WorldEngine {
    definition: WorldDefinition,
    state: WorldState,
    store: EventStore,
    last_clock_utc_ms: i64,
    last_clock_checkpoint_utc_ms: i64,
}

impl WorldEngine {
    pub fn open(
        database_path: impl AsRef<Path>,
        identity_id: &str,
        definition: WorldDefinition,
        initial_anchor_id: &str,
        now_ms: i64,
        guard: &PathGuard,
    ) -> Result<Self> {
        validate_external_id("identity_id", identity_id)?;
        if !definition.contains_anchor(initial_anchor_id) {
            return Err(WorldError::InvalidDefinition(format!(
                "initial anchor {initial_anchor_id} does not exist"
            )));
        }
        let mut store = EventStore::open(database_path, identity_id, definition.hash(), guard)?;
        let mut state = store.load_state(identity_id, definition.hash())?;
        if state.last_event_seq() == 0 {
            state = store.commit_system_events(
                &state,
                now_ms,
                vec![DomainEvent::AgentAwakened {
                    anchor_id: initial_anchor_id.into(),
                }],
            )?;
            store.force_snapshot(&state, now_ms)?;
        }
        let last_clock_utc_ms = store.clock_checkpoint_utc_ms()?.max(state.last_utc_ms());
        Ok(Self {
            definition,
            state,
            store,
            last_clock_utc_ms,
            last_clock_checkpoint_utc_ms: last_clock_utc_ms,
        })
    }

    pub fn state(&self) -> &WorldState {
        &self.state
    }

    pub fn definition(&self) -> &WorldDefinition {
        &self.definition
    }

    pub fn database_path(&self) -> &Path {
        self.store.database_path()
    }

    pub fn perception(&self) -> Result<PerceptionWindow> {
        PerceptionWindow::build(&self.state, &self.definition)
    }

    pub fn command_result(&self, command_id: &str) -> Result<Option<CommandResult>> {
        Ok(self
            .store
            .find_command(command_id)?
            .map(|(_, result)| result))
    }

    pub fn events_after(&self, after_seq: u64) -> Result<Vec<PersistedEvent>> {
        self.store.events_after(after_seq)
    }

    pub fn execute_command(
        &mut self,
        command: &CommandEnvelope,
        now_ms: i64,
    ) -> Result<CommandResult> {
        let request_json = serde_json::to_vec(command)?;
        let request_hash = digest(&request_json);

        if let Some((stored_hash, mut stored_result)) =
            self.store.find_command(&command.command_id)?
        {
            if stored_hash != request_hash {
                return Ok(CommandResult::rejected(
                    &command.command_id,
                    CommandStatus::InvalidArgument,
                    &self.state,
                    "COMMAND_ID_COLLISION",
                    "command_id was already used with a different payload",
                ));
            }
            if stored_result.status == CommandStatus::Committed {
                stored_result.status = CommandStatus::AlreadyCommitted;
            }
            return Ok(stored_result);
        }

        if let Some(result) = self.validate_envelope(command, now_ms) {
            self.store
                .record_rejection(&request_hash, &result, now_ms)?;
            return Ok(result);
        }

        let events = match self.build_command_events(command, now_ms) {
            Ok(events) => events,
            Err(result) => {
                let result = *result;
                self.store
                    .record_rejection(&request_hash, &result, now_ms)?;
                return Ok(result);
            }
        };
        let (state, result) = self.store.commit_command(
            &self.state,
            &command.command_id,
            &request_hash,
            now_ms,
            events,
        )?;
        self.state = state;
        Ok(result)
    }

    pub fn tick(&mut self, sample: ClockSample) -> Result<()> {
        if self.state.time_status() != &TimeStatus::Normal {
            return Ok(());
        }
        if sample.monotonic_elapsed_ms < 0 {
            return Err(WorldError::StateInvariant(
                "monotonic elapsed time cannot be negative".into(),
            ));
        }
        let expected_utc = self
            .last_clock_utc_ms
            .saturating_add(sample.monotonic_elapsed_ms);
        let drift = sample.utc_ms.saturating_sub(expected_utc).abs();
        if drift > MAX_CLOCK_DRIFT_MS {
            self.state = self.store.commit_system_events(
                &self.state,
                sample.utc_ms,
                vec![DomainEvent::TimeAnomalyDetected {
                    previous_utc_ms: self.last_clock_utc_ms,
                    observed_utc_ms: sample.utc_ms,
                    monotonic_elapsed_ms: sample.monotonic_elapsed_ms,
                }],
            )?;
            self.last_clock_utc_ms = sample.utc_ms;
            return Ok(());
        }

        self.complete_due_activities(sample.utc_ms)?;
        self.last_clock_utc_ms = sample.utc_ms;
        if sample
            .utc_ms
            .saturating_sub(self.last_clock_checkpoint_utc_ms)
            >= CLOCK_CHECKPOINT_INTERVAL_MS
        {
            self.checkpoint_clock(sample.utc_ms)?;
        }
        Ok(())
    }

    pub fn resume_after_downtime(&mut self, now_ms: i64) -> Result<()> {
        if self.state.time_status() != &TimeStatus::Normal {
            return Ok(());
        }
        let previous = self.last_clock_utc_ms;
        if now_ms < previous {
            self.state = self.store.commit_system_events(
                &self.state,
                now_ms,
                vec![DomainEvent::TimeAnomalyDetected {
                    previous_utc_ms: previous,
                    observed_utc_ms: now_ms,
                    monotonic_elapsed_ms: 0,
                }],
            )?;
        } else if now_ms.saturating_sub(previous) >= MIN_RECORDED_DOWNTIME_MS {
            let mut events = vec![DomainEvent::DowntimeObserved {
                from_utc_ms: previous,
                to_utc_ms: now_ms,
            }];
            events.extend(
                self.state
                    .due_activities(now_ms)
                    .into_iter()
                    .map(|activity| DomainEvent::ActivityCompleted {
                        activity_id: activity.activity_id,
                    }),
            );
            self.state = self
                .store
                .commit_system_events(&self.state, now_ms, events)?;
        }
        self.last_clock_utc_ms = now_ms;
        self.checkpoint_clock(now_ms)
    }

    pub fn checkpoint_clock(&mut self, now_ms: i64) -> Result<()> {
        self.store.record_clock_checkpoint(now_ms)?;
        self.last_clock_checkpoint_utc_ms = now_ms;
        Ok(())
    }

    pub fn snapshot(&mut self, now_ms: i64) -> Result<()> {
        self.store.force_snapshot(&self.state, now_ms)?;
        self.last_clock_checkpoint_utc_ms = now_ms;
        Ok(())
    }

    fn validate_envelope(&self, command: &CommandEnvelope, now_ms: i64) -> Option<CommandResult> {
        let reject = |status, code, message: String| {
            CommandResult::rejected(&command.command_id, status, &self.state, code, message)
        };
        if validate_external_id("command_id", &command.command_id).is_err()
            || validate_external_id("decision_id", &command.decision_id).is_err()
        {
            return Some(reject(
                CommandStatus::InvalidArgument,
                "INVALID_ID",
                "command_id and decision_id must be stable non-empty IDs".into(),
            ));
        }
        if command.identity_id != self.state.identity_id() || command.agent_id != PRIMARY_AGENT_ID {
            return Some(reject(
                CommandStatus::Unauthorized,
                "IDENTITY_MISMATCH",
                "command identity or agent does not match this world".into(),
            ));
        }
        if command.schema_version != COMMAND_SCHEMA_VERSION {
            return Some(reject(
                CommandStatus::InvalidArgument,
                "UNSUPPORTED_SCHEMA",
                format!("expected command schema version {COMMAND_SCHEMA_VERSION}"),
            ));
        }
        if command.ttl_ms <= 0 || command.ttl_ms > MAX_COMMAND_TTL_MS {
            return Some(reject(
                CommandStatus::InvalidArgument,
                "INVALID_TTL",
                format!("ttl_ms must be within 1..={MAX_COMMAND_TTL_MS}"),
            ));
        }
        if command.issued_at_ms.saturating_add(command.ttl_ms) < now_ms {
            return Some(reject(
                CommandStatus::ExpiredDecision,
                "EXPIRED_DECISION",
                "command TTL expired before execution".into(),
            ));
        }
        if command.expected_world_version != self.state.world_version() {
            return Some(reject(
                CommandStatus::StaleWorld,
                "STALE_WORLD",
                format!(
                    "expected world version {}, current version is {}",
                    command.expected_world_version,
                    self.state.world_version()
                ),
            ));
        }
        if self.state.time_status() != &TimeStatus::Normal {
            return Some(reject(
                CommandStatus::TemporarilyUnavailable,
                "WORLD_NOT_READY",
                format!("world time status is {:?}", self.state.time_status()),
            ));
        }
        None
    }

    fn build_command_events(
        &self,
        command: &CommandEnvelope,
        now_ms: i64,
    ) -> std::result::Result<Vec<DomainEvent>, Box<CommandResult>> {
        match &command.payload {
            CommandPayload::MoveTo { target_anchor_id } => {
                if !self.definition.contains_anchor(target_anchor_id) {
                    return Err(Box::new(self.command_error(
                        command,
                        CommandStatus::RejectedPrecondition,
                        "UNKNOWN_ANCHOR",
                        format!("anchor {target_anchor_id} does not exist"),
                    )));
                }
                if target_anchor_id == self.state.agent_anchor_id() {
                    return Err(Box::new(self.command_error(
                        command,
                        CommandStatus::RejectedPrecondition,
                        "ALREADY_THERE",
                        "agent is already at the requested anchor",
                    )));
                }
                let Some(duration_ms) = self
                    .definition
                    .movement_duration(self.state.agent_anchor_id(), target_anchor_id)
                else {
                    return Err(Box::new(self.command_error(
                        command,
                        CommandStatus::RejectedPrecondition,
                        "NO_PATH",
                        "target anchor is not reachable",
                    )));
                };
                self.ensure_resources(command, [Resource::Movement])?;
                Ok(vec![DomainEvent::ActivityScheduled {
                    activity: Activity {
                        activity_id: format!("activity-{}", command.command_id),
                        action_id: "world.move_to".into(),
                        target_id: target_anchor_id.clone(),
                        started_at_ms: now_ms,
                        completes_at_ms: now_ms.saturating_add(duration_ms),
                        resources: BTreeSet::from([Resource::Movement]),
                        completion: ActivityCompletion::MoveTo {
                            anchor_id: target_anchor_id.clone(),
                        },
                    },
                }])
            }
            CommandPayload::Perform {
                action_id,
                target_id,
                parameters,
            } => {
                let Some(affordance) = self.definition.object_action(target_id, action_id) else {
                    return Err(Box::new(self.command_error(
                        command,
                        CommandStatus::RejectedPrecondition,
                        "UNKNOWN_AFFORDANCE",
                        "object does not provide the requested action",
                    )));
                };
                let object_is_here = self.state.object_is_at_anchor(
                    target_id,
                    self.state.agent_anchor_id(),
                    &self.definition,
                ) && self.state.object_is_visible(target_id, &self.definition);
                if !object_is_here {
                    return Err(Box::new(self.command_error(
                        command,
                        CommandStatus::RejectedPrecondition,
                        "TARGET_NOT_REACHABLE",
                        "object is not available at the current anchor",
                    )));
                }
                if action_id != "object.relocate" && !parameters.is_empty() {
                    return Err(Box::new(self.command_error(
                        command,
                        CommandStatus::InvalidArgument,
                        "UNEXPECTED_PARAMETERS",
                        "this action does not accept parameters",
                    )));
                }
                let completion = match action_id.as_str() {
                    "object.toggle_power" => ActivityCompletion::SetObjectPower {
                        object_id: target_id.clone(),
                        powered: !self.state.object_power(target_id, &self.definition),
                    },
                    "object.toggle_open" => ActivityCompletion::SetObjectOpen {
                        object_id: target_id.clone(),
                        open: !self.state.object_open(target_id, &self.definition),
                    },
                    "object.relocate" => {
                        let placement = parse_relocation_parameters(parameters).map_err(
                            |(code, message)| {
                                Box::new(self.command_error(
                                    command,
                                    CommandStatus::InvalidArgument,
                                    code,
                                    message,
                                ))
                            },
                        )?;
                        if placement.anchor_id != self.state.agent_anchor_id() {
                            return Err(Box::new(self.command_error(
                                command,
                                CommandStatus::RejectedPrecondition,
                                "DESTINATION_NOT_REACHABLE",
                                "rearrangement destination must be at the current anchor",
                            )));
                        }
                        if self.definition.object_has_component(target_id, "powerable")
                            && self.state.object_power(target_id, &self.definition)
                        {
                            return Err(Box::new(self.command_error(
                                command,
                                CommandStatus::RejectedPrecondition,
                                "OBJECT_POWERED",
                                "powered object must be switched off before relocation",
                            )));
                        }
                        if let Some(parent_id) = placement.parent_object_id.as_deref() {
                            let parent_reachable =
                                self.state.object_is_at_anchor(
                                    parent_id,
                                    self.state.agent_anchor_id(),
                                    &self.definition,
                                ) && self.state.object_is_visible(parent_id, &self.definition);
                            if !parent_reachable {
                                return Err(Box::new(self.command_error(
                                    command,
                                    CommandStatus::RejectedPrecondition,
                                    "DESTINATION_NOT_REACHABLE",
                                    "placement parent is not available at the current anchor",
                                )));
                            }
                            if placement.relation == PlacementRelation::Container
                                && self.definition.object_has_component(parent_id, "openable")
                                && !self.state.object_open(parent_id, &self.definition)
                            {
                                return Err(Box::new(self.command_error(
                                    command,
                                    CommandStatus::RejectedPrecondition,
                                    "DESTINATION_CLOSED",
                                    "container must be open before placing an object inside",
                                )));
                            }
                        }
                        let placements = self.state.object_placements(&self.definition);
                        self.definition
                            .validate_placement_change(target_id, &placement, &placements)
                            .map_err(|violation| {
                                Box::new(self.command_error(
                                    command,
                                    CommandStatus::RejectedPrecondition,
                                    violation.code,
                                    violation.message,
                                ))
                            })?;
                        ActivityCompletion::SetObjectPlacement {
                            object_id: target_id.clone(),
                            placement,
                        }
                    }
                    _ => {
                        return Err(Box::new(self.command_error(
                            command,
                            CommandStatus::InvalidArgument,
                            "ACTION_NOT_IMPLEMENTED",
                            "the action exists in data but has no executor",
                        )));
                    }
                };
                let resources = affordance
                    .required_resources
                    .iter()
                    .filter_map(|resource| Resource::parse(resource))
                    .collect::<BTreeSet<_>>();
                self.ensure_resources(command, resources.iter().cloned())?;
                let activity = Activity {
                    activity_id: format!("activity-{}", command.command_id),
                    action_id: action_id.clone(),
                    target_id: target_id.clone(),
                    started_at_ms: now_ms,
                    completes_at_ms: now_ms.saturating_add(affordance.duration_ms),
                    resources,
                    completion,
                };
                let activity_id = activity.activity_id.clone();
                let mut events = vec![DomainEvent::ActivityScheduled { activity }];
                if affordance.duration_ms == 0 {
                    events.push(DomainEvent::ActivityCompleted { activity_id });
                }
                Ok(events)
            }
            CommandPayload::Inspect { .. } => Err(Box::new(self.command_error(
                command,
                CommandStatus::InvalidArgument,
                "ACTION_NOT_IMPLEMENTED",
                "inspect will be added with subjective observation events",
            ))),
        }
    }

    fn ensure_resources(
        &self,
        command: &CommandEnvelope,
        resources: impl IntoIterator<Item = Resource>,
    ) -> std::result::Result<(), Box<CommandResult>> {
        for resource in resources {
            if let Some(owner) = self.state.resource_owner(&resource) {
                return Err(Box::new(self.command_error(
                    command,
                    CommandStatus::ResourceConflict,
                    "RESOURCE_CONFLICT",
                    format!("resource {} is reserved by {owner}", resource.as_str()),
                )));
            }
        }
        Ok(())
    }

    fn command_error(
        &self,
        command: &CommandEnvelope,
        status: CommandStatus,
        code: &str,
        message: impl Into<String>,
    ) -> CommandResult {
        CommandResult::rejected(&command.command_id, status, &self.state, code, message)
    }

    fn complete_due_activities(&mut self, now_ms: i64) -> Result<()> {
        let events = self
            .state
            .due_activities(now_ms)
            .into_iter()
            .map(|activity| DomainEvent::ActivityCompleted {
                activity_id: activity.activity_id,
            })
            .collect();
        self.state = self
            .store
            .commit_system_events(&self.state, now_ms, events)?;
        Ok(())
    }
}

fn validate_external_id(kind: &str, value: &str) -> Result<()> {
    if value.is_empty()
        || value.len() > 128
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.' | b':'))
    {
        return Err(WorldError::StateInvariant(format!(
            "{kind} is not a valid stable ID"
        )));
    }
    Ok(())
}

fn parse_relocation_parameters(
    parameters: &BTreeMap<String, String>,
) -> std::result::Result<ObjectPlacement, (&'static str, String)> {
    let allowed = BTreeSet::from(["relation", "anchor_id", "parent_object_id", "slot_id"]);
    if let Some(key) = parameters
        .keys()
        .find(|key| !allowed.contains(key.as_str()))
    {
        return Err((
            "UNKNOWN_PARAMETER",
            format!("object.relocate does not accept parameter {key}"),
        ));
    }
    let anchor_id = required_parameter(parameters, "anchor_id")?;
    validate_parameter_id("anchor_id", anchor_id)?;
    let relation = match required_parameter(parameters, "relation")? {
        "anchor" => PlacementRelation::Anchor,
        "surface" => PlacementRelation::Surface,
        "container" => PlacementRelation::Container,
        value => {
            return Err((
                "INVALID_PLACEMENT",
                format!("unknown placement relation {value}"),
            ));
        }
    };
    let parent_object_id = parameters.get("parent_object_id").cloned();
    let slot_id = parameters.get("slot_id").cloned();
    match relation {
        PlacementRelation::Anchor if parent_object_id.is_some() || slot_id.is_some() => {
            return Err((
                "INVALID_PLACEMENT",
                "anchor placement cannot declare parent_object_id or slot_id".into(),
            ));
        }
        PlacementRelation::Surface | PlacementRelation::Container => {
            let Some(parent) = parent_object_id.as_deref() else {
                return Err((
                    "MISSING_PARAMETER",
                    "parent_object_id is required for placed objects".into(),
                ));
            };
            let Some(slot) = slot_id.as_deref() else {
                return Err((
                    "MISSING_PARAMETER",
                    "slot_id is required for placed objects".into(),
                ));
            };
            validate_parameter_id("parent_object_id", parent)?;
            validate_parameter_id("slot_id", slot)?;
        }
        PlacementRelation::Anchor => {}
    }
    Ok(ObjectPlacement {
        anchor_id: anchor_id.into(),
        relation,
        parent_object_id,
        slot_id,
    })
}

fn required_parameter<'a>(
    parameters: &'a BTreeMap<String, String>,
    name: &'static str,
) -> std::result::Result<&'a str, (&'static str, String)> {
    parameters.get(name).map(String::as_str).ok_or_else(|| {
        (
            "MISSING_PARAMETER",
            format!("{name} is required for object.relocate"),
        )
    })
}

fn validate_parameter_id(
    name: &str,
    value: &str,
) -> std::result::Result<(), (&'static str, String)> {
    validate_external_id(name, value)
        .map_err(|_| ("INVALID_PARAMETER", format!("{name} is not a stable ID")))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn definition() -> WorldDefinition {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../world-packages/test-room-v1/manifest.json");
        WorldDefinition::load(path, &PathGuard::default()).unwrap()
    }

    fn open(temp: &tempfile::TempDir, now_ms: i64) -> WorldEngine {
        WorldEngine::open(
            temp.path().join("world.db"),
            "test-makise",
            definition(),
            "bed",
            now_ms,
            &PathGuard::default(),
        )
        .unwrap()
    }

    fn move_command(engine: &WorldEngine, id: &str, now_ms: i64) -> CommandEnvelope {
        CommandEnvelope {
            command_id: id.into(),
            identity_id: "test-makise".into(),
            agent_id: "makise".into(),
            expected_world_version: engine.state().world_version(),
            schema_version: COMMAND_SCHEMA_VERSION,
            decision_id: format!("decision-{id}"),
            issued_at_ms: now_ms,
            ttl_ms: 30_000,
            payload: CommandPayload::MoveTo {
                target_anchor_id: "work_desk".into(),
            },
        }
    }

    #[test]
    fn duplicate_command_is_not_executed_twice() {
        let temp = tempfile::tempdir().unwrap();
        let mut engine = open(&temp, 1_000_000);
        let command = move_command(&engine, "cmd-1", 1_000_010);
        let first = engine.execute_command(&command, 1_000_010).unwrap();
        let version = engine.state().world_version();
        let duplicate = engine.execute_command(&command, 1_000_020).unwrap();

        assert_eq!(first.status, CommandStatus::Committed);
        assert_eq!(duplicate.status, CommandStatus::AlreadyCommitted);
        assert_eq!(engine.state().world_version(), version);
        assert_eq!(first.first_event_seq, duplicate.first_event_seq);
    }

    #[test]
    fn stale_decision_is_rejected_without_state_change() {
        let temp = tempfile::tempdir().unwrap();
        let mut engine = open(&temp, 2_000_000);
        let mut command = move_command(&engine, "cmd-stale", 2_000_010);
        command.expected_world_version = 0;
        let version = engine.state().world_version();

        let result = engine.execute_command(&command, 2_000_010).unwrap();
        assert_eq!(result.status, CommandStatus::StaleWorld);
        assert_eq!(engine.state().world_version(), version);
    }

    #[test]
    fn durable_activity_survives_restart_and_replays_identically() {
        let temp = tempfile::tempdir().unwrap();
        let before_hash;
        {
            let mut engine = open(&temp, 3_000_000);
            let command = move_command(&engine, "cmd-move", 3_000_010);
            engine.execute_command(&command, 3_000_010).unwrap();
            assert_eq!(engine.state().agent_anchor_id(), "bed");
            before_hash = engine.state().state_hash().unwrap();
        }

        let mut recovered = open(&temp, 3_001_000);
        assert_eq!(recovered.state().state_hash().unwrap(), before_hash);
        recovered.resume_after_downtime(3_005_000).unwrap();
        assert_eq!(recovered.state().agent_anchor_id(), "work_desk");
        let completed_hash = recovered.state().state_hash().unwrap();
        drop(recovered);

        let replayed = open(&temp, 3_005_000);
        assert_eq!(replayed.state().state_hash().unwrap(), completed_hash);
    }

    #[test]
    fn wall_clock_jump_enters_time_anomaly() {
        let temp = tempfile::tempdir().unwrap();
        let mut engine = open(&temp, 4_000_000);
        engine
            .tick(ClockSample {
                utc_ms: 4_300_000,
                monotonic_elapsed_ms: 1_000,
            })
            .unwrap();
        assert_eq!(engine.state().time_status(), &TimeStatus::TimeAnomaly);
    }

    #[test]
    fn perception_does_not_expose_hidden_object_properties() {
        let temp = tempfile::tempdir().unwrap();
        let mut engine = open(&temp, 5_000_000);
        let command = move_command(&engine, "cmd-perception", 5_000_010);
        engine.execute_command(&command, 5_000_010).unwrap();
        engine
            .tick(ClockSample {
                utc_ms: 5_005_000,
                monotonic_elapsed_ms: 5_000,
            })
            .unwrap();
        let json = serde_json::to_string(&engine.perception().unwrap()).unwrap();
        assert!(json.contains("desk_lamp"));
        assert!(!json.contains("factory_serial"));
        assert!(!json.contains("MAKISE-SECRET-001"));
    }

    #[test]
    fn command_id_collision_is_rejected() {
        let temp = tempfile::tempdir().unwrap();
        let mut engine = open(&temp, 6_000_000);
        let command = move_command(&engine, "cmd-collision", 6_000_010);
        engine.execute_command(&command, 6_000_010).unwrap();
        let mut collision = command.clone();
        collision.payload = CommandPayload::Inspect {
            target_id: "bed".into(),
        };
        let result = engine.execute_command(&collision, 6_000_020).unwrap();
        assert_eq!(result.status, CommandStatus::InvalidArgument);
        assert_eq!(result.error_code.as_deref(), Some("COMMAND_ID_COLLISION"));
    }
    #[test]
    fn clock_checkpoint_distinguishes_idle_runtime_from_real_downtime() {
        let temp = tempfile::tempdir().unwrap();
        {
            let mut engine = open(&temp, 7_000_000);
            engine
                .tick(ClockSample {
                    utc_ms: 7_010_000,
                    monotonic_elapsed_ms: 10_000,
                })
                .unwrap();
            assert_eq!(engine.state().last_event_seq(), 1);
        }

        let mut recovered = open(&temp, 7_010_500);
        recovered.resume_after_downtime(7_010_500).unwrap();
        assert_eq!(recovered.state().last_event_seq(), 1);
        recovered.resume_after_downtime(7_012_000).unwrap();
        assert_eq!(recovered.state().last_event_seq(), 2);
    }
}
