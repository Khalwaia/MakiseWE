#![forbid(unsafe_code)]

mod actor;
mod definition_stage4;
mod domain;
mod engine;
mod error;
mod path_guard;
mod rpc;
#[cfg(unix)]
mod server;
mod store;

pub use actor::{ActorError, EventBatch, HealthSnapshot, WorldActorConfig, WorldActorHandle};
pub use definition_stage4::{
    Affordance, ConnectionView, ObjectPlacement, ObjectView, PlacementRelation, WorldDefinition,
};
pub use domain::{
    ClockSample, CommandEnvelope, CommandPayload, CommandResult, CommandStatus, PerceptionWindow,
    PersistedEvent, TimeStatus, WorldState,
};
pub use engine::WorldEngine;
pub use error::{Result, WorldError};
pub use path_guard::{PROTECTED_MINA_RUNTIME, PathGuard};
pub use rpc::WorldRpc;
#[cfg(unix)]
pub use server::serve_uds;
