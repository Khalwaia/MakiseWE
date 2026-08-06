use std::sync::mpsc::{Receiver, SyncSender, TrySendError, sync_channel};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use tokio::sync::{broadcast, oneshot};

use crate::{
    ClockSample, CommandEnvelope, CommandResult, PerceptionWindow, PersistedEvent, TimeStatus,
    WeatherObservation, WorldEngine, WorldError,
};

#[derive(Clone, Debug)]
pub struct WorldActorConfig {
    pub command_queue_capacity: usize,
    pub event_broadcast_capacity: usize,
    pub tick_interval: Duration,
}

impl Default for WorldActorConfig {
    fn default() -> Self {
        Self {
            command_queue_capacity: 64,
            event_broadcast_capacity: 256,
            tick_interval: Duration::from_millis(100),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ActorError {
    #[error("world actor command queue is full")]
    Busy,
    #[error("world actor stopped")]
    Stopped,
    #[error("world clock is unavailable: {0}")]
    Clock(String),
    #[error(transparent)]
    World(#[from] WorldError),
}

#[derive(Clone, Debug)]
pub struct EventBatch {
    pub head_seq: u64,
    pub events: Vec<PersistedEvent>,
}

#[derive(Clone, Debug)]
pub struct HealthSnapshot {
    pub identity_id: String,
    pub world_definition_hash: String,
    pub world_version: u64,
    pub last_event_seq: u64,
    pub time_status: TimeStatus,
    pub last_error: Option<String>,
}

#[derive(Clone)]
pub struct WorldActorHandle {
    sender: SyncSender<ActorRequest>,
    event_sender: broadcast::Sender<PersistedEvent>,
}

enum ActorRequest {
    Execute {
        command: CommandEnvelope,
        respond: oneshot::Sender<Result<CommandResult, ActorError>>,
    },
    CommandResult {
        command_id: String,
        respond: oneshot::Sender<Result<Option<CommandResult>, ActorError>>,
    },
    ObserveWeather {
        observation: WeatherObservation,
        respond: oneshot::Sender<Result<bool, ActorError>>,
    },
    Perception {
        respond: oneshot::Sender<Result<PerceptionWindow, ActorError>>,
    },
    EventsAfter {
        after_seq: u64,
        respond: oneshot::Sender<Result<EventBatch, ActorError>>,
    },
    Health {
        respond: oneshot::Sender<HealthSnapshot>,
    },
}

impl WorldActorHandle {
    pub fn spawn(mut engine: WorldEngine, config: WorldActorConfig) -> Result<Self, ActorError> {
        if config.command_queue_capacity == 0 || config.event_broadcast_capacity == 0 {
            return Err(ActorError::World(WorldError::StateInvariant(
                "actor queue capacities must be greater than zero".into(),
            )));
        }
        if config.tick_interval.is_zero() {
            return Err(ActorError::World(WorldError::StateInvariant(
                "actor tick interval must be greater than zero".into(),
            )));
        }

        engine.resume_after_downtime(unix_time_ms()?)?;
        let (sender, receiver) = sync_channel(config.command_queue_capacity);
        let (event_sender, _) = broadcast::channel(config.event_broadcast_capacity);
        let actor_event_sender = event_sender.clone();
        std::thread::Builder::new()
            .name("makise-world-writer".into())
            .spawn(move || actor_loop(engine, receiver, actor_event_sender, config.tick_interval))
            .map_err(|error| ActorError::Clock(format!("failed to start actor thread: {error}")))?;
        Ok(Self {
            sender,
            event_sender,
        })
    }

    pub async fn execute(&self, command: CommandEnvelope) -> Result<CommandResult, ActorError> {
        let (respond, response) = oneshot::channel();
        self.enqueue(ActorRequest::Execute { command, respond })?;
        response.await.map_err(|_| ActorError::Stopped)?
    }

    pub async fn command_result(
        &self,
        command_id: String,
    ) -> Result<Option<CommandResult>, ActorError> {
        let (respond, response) = oneshot::channel();
        self.enqueue(ActorRequest::CommandResult {
            command_id,
            respond,
        })?;
        response.await.map_err(|_| ActorError::Stopped)?
    }

    pub async fn observe_weather(
        &self,
        observation: WeatherObservation,
    ) -> Result<bool, ActorError> {
        let (respond, response) = oneshot::channel();
        self.enqueue(ActorRequest::ObserveWeather {
            observation,
            respond,
        })?;
        response.await.map_err(|_| ActorError::Stopped)?
    }

    pub async fn perception(&self) -> Result<PerceptionWindow, ActorError> {
        let (respond, response) = oneshot::channel();
        self.enqueue(ActorRequest::Perception { respond })?;
        response.await.map_err(|_| ActorError::Stopped)?
    }

    pub async fn events_after(&self, after_seq: u64) -> Result<EventBatch, ActorError> {
        let (respond, response) = oneshot::channel();
        self.enqueue(ActorRequest::EventsAfter { after_seq, respond })?;
        response.await.map_err(|_| ActorError::Stopped)?
    }

    pub async fn health(&self) -> Result<HealthSnapshot, ActorError> {
        let (respond, response) = oneshot::channel();
        self.enqueue(ActorRequest::Health { respond })?;
        response.await.map_err(|_| ActorError::Stopped)
    }

    pub fn subscribe_events(&self) -> broadcast::Receiver<PersistedEvent> {
        self.event_sender.subscribe()
    }

    fn enqueue(&self, request: ActorRequest) -> Result<(), ActorError> {
        match self.sender.try_send(request) {
            Ok(()) => Ok(()),
            Err(TrySendError::Full(_)) => Err(ActorError::Busy),
            Err(TrySendError::Disconnected(_)) => Err(ActorError::Stopped),
        }
    }
}

fn actor_loop(
    mut engine: WorldEngine,
    receiver: Receiver<ActorRequest>,
    event_sender: broadcast::Sender<PersistedEvent>,
    tick_interval: Duration,
) {
    let mut last_tick = Instant::now();
    let mut published_seq = engine.state().last_event_seq();

    loop {
        match receiver.recv_timeout(tick_interval) {
            Ok(request) => {
                let clock = advance_clock(
                    &mut engine,
                    &event_sender,
                    &mut last_tick,
                    &mut published_seq,
                );
                let mut request_error = clock.as_ref().err().map(ToString::to_string);
                handle_request(
                    request,
                    clock,
                    &mut engine,
                    &event_sender,
                    &mut published_seq,
                    &mut request_error,
                );
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                let _ = advance_clock(
                    &mut engine,
                    &event_sender,
                    &mut last_tick,
                    &mut published_seq,
                );
            }
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                if let Ok(now_ms) = advance_clock(
                    &mut engine,
                    &event_sender,
                    &mut last_tick,
                    &mut published_seq,
                ) {
                    let _ = engine.checkpoint_clock(now_ms);
                }
                break;
            }
        }
    }
}

fn handle_request(
    request: ActorRequest,
    clock: Result<i64, ActorError>,
    engine: &mut WorldEngine,
    event_sender: &broadcast::Sender<PersistedEvent>,
    published_seq: &mut u64,
    last_error: &mut Option<String>,
) {
    match request {
        ActorRequest::Execute { command, respond } => {
            let result = clock.and_then(|now_ms| {
                let result = engine.execute_command(&command, now_ms)?;
                publish_new_events(engine, event_sender, published_seq)?;
                Ok(result)
            });
            if let Err(error) = &result {
                *last_error = Some(error.to_string());
            }
            let _ = respond.send(result);
        }
        ActorRequest::CommandResult {
            command_id,
            respond,
        } => {
            let result = clock.and_then(|_| Ok(engine.command_result(&command_id)?));
            if let Err(error) = &result {
                *last_error = Some(error.to_string());
            }
            let _ = respond.send(result);
        }
        ActorRequest::ObserveWeather {
            observation,
            respond,
        } => {
            let result = clock.and_then(|now_ms| {
                let changed = engine.observe_weather(observation, now_ms)?;
                if changed {
                    publish_new_events(engine, event_sender, published_seq)?;
                }
                Ok(changed)
            });
            if let Err(error) = &result {
                *last_error = Some(error.to_string());
            }
            let _ = respond.send(result);
        }
        ActorRequest::Perception { respond } => {
            let result = clock.and_then(|_| Ok(engine.perception()?));
            if let Err(error) = &result {
                *last_error = Some(error.to_string());
            }
            let _ = respond.send(result);
        }
        ActorRequest::EventsAfter { after_seq, respond } => {
            let result = clock.and_then(|_| {
                Ok(EventBatch {
                    head_seq: engine.state().last_event_seq(),
                    events: engine.events_after(after_seq)?,
                })
            });
            if let Err(error) = &result {
                *last_error = Some(error.to_string());
            }
            let _ = respond.send(result);
        }
        ActorRequest::Health { respond } => {
            let state = engine.state();
            let _ = respond.send(HealthSnapshot {
                identity_id: state.identity_id().into(),
                world_definition_hash: state.world_definition_hash().into(),
                world_version: state.world_version(),
                last_event_seq: state.last_event_seq(),
                time_status: state.time_status().clone(),
                last_error: last_error.clone(),
            });
        }
    }
}

fn advance_clock(
    engine: &mut WorldEngine,
    event_sender: &broadcast::Sender<PersistedEvent>,
    last_tick: &mut Instant,
    published_seq: &mut u64,
) -> Result<i64, ActorError> {
    let now_instant = Instant::now();
    let elapsed = now_instant.duration_since(*last_tick);
    *last_tick = now_instant;
    let monotonic_elapsed_ms = i64::try_from(elapsed.as_millis())
        .map_err(|_| ActorError::Clock("monotonic interval does not fit in i64".into()))?;
    let utc_ms = unix_time_ms()?;
    engine.tick(ClockSample {
        utc_ms,
        monotonic_elapsed_ms,
    })?;
    publish_new_events(engine, event_sender, published_seq)?;
    Ok(utc_ms)
}

fn publish_new_events(
    engine: &WorldEngine,
    event_sender: &broadcast::Sender<PersistedEvent>,
    published_seq: &mut u64,
) -> Result<(), ActorError> {
    for event in engine.events_after(*published_seq)? {
        let expected = published_seq.saturating_add(1);
        if event.event_seq != expected {
            return Err(ActorError::World(WorldError::EventSequenceGap {
                expected,
                actual: event.event_seq,
            }));
        }
        *published_seq = event.event_seq;
        let _ = event_sender.send(event);
    }
    Ok(())
}

fn unix_time_ms() -> Result<i64, ActorError> {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| ActorError::Clock(error.to_string()))?;
    i64::try_from(duration.as_millis())
        .map_err(|_| ActorError::Clock("Unix time does not fit in i64".into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounded_queue_reports_backpressure() {
        let (sender, _receiver) = sync_channel(1);
        let (event_sender, _) = broadcast::channel(1);
        let handle = WorldActorHandle {
            sender,
            event_sender,
        };
        let (first, _) = oneshot::channel();
        handle
            .enqueue(ActorRequest::Health { respond: first })
            .unwrap();
        let (second, _) = oneshot::channel();
        assert!(matches!(
            handle.enqueue(ActorRequest::Health { respond: second }),
            Err(ActorError::Busy)
        ));
    }

    #[test]
    fn disconnected_queue_reports_stopped() {
        let (sender, receiver) = sync_channel(1);
        drop(receiver);
        let (event_sender, _) = broadcast::channel(1);
        let handle = WorldActorHandle {
            sender,
            event_sender,
        };
        let (respond, _) = oneshot::channel();
        assert!(matches!(
            handle.enqueue(ActorRequest::Health { respond }),
            Err(ActorError::Stopped)
        ));
    }
}
