//! Persistent causal kernel for Makise V1 timelines.

use std::path::{Path, PathBuf};

use rusqlite::{Connection, OptionalExtension, params};
use thiserror::Error;

mod artifact;
mod quantity;
mod thermal;

pub use artifact::{
    AdmissionError, AdmissionRecord, ArtifactBundle, ContractParseError, MechanismContract,
    ProgramAbi,
};
pub use quantity::{Dimension, Quantity, QuantityError, ReservoirState, StateHash, UnitScale};
pub use thermal::{ReservoirPair, ThermalError, ThermalProposal, ThermalTransfer};

const APPLICATION_ID: i32 = 0x4d4b_5631;
const SCHEMA_VERSION: i32 = 1;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorldId(String);

impl WorldId {
    pub fn new(value: impl Into<String>) -> Result<Self, IdentifierError> {
        Identifier::validate(value.into()).map(|value| Self(value.0))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TimelineId(String);

impl TimelineId {
    pub fn new(value: impl Into<String>) -> Result<Self, IdentifierError> {
        Identifier::validate(value.into()).map(|value| Self(value.0))
    }
}

struct Identifier(String);

impl Identifier {
    fn validate(value: String) -> Result<Self, IdentifierError> {
        if value.trim().is_empty() {
            return Err(IdentifierError::Empty);
        }
        Ok(Self(value))
    }
}

#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum IdentifierError {
    #[error("identifier cannot be empty")]
    Empty,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OpenSpec {
    world_id: WorldId,
    timeline_id: TimelineId,
}

impl OpenSpec {
    pub fn new(world_id: WorldId, timeline_id: TimelineId) -> Self {
        Self {
            world_id,
            timeline_id,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StorageLocation(PathBuf);

impl StorageLocation {
    pub fn sqlite(path: impl Into<PathBuf>) -> Self {
        Self(path.into())
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RecoveryStatus {
    Created,
    Recovered,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecoveryReport {
    status: RecoveryStatus,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ProjectionRequest {
    _private: (),
}

impl ProjectionRequest {
    pub fn current() -> Self {
        Self { _private: () }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Projection {
    timeline_id: TimelineId,
    timeline_version: u64,
    entity_count: usize,
}

impl Projection {
    pub fn timeline_id(&self) -> &TimelineId {
        &self.timeline_id
    }

    pub fn timeline_version(&self) -> u64 {
        self.timeline_version
    }

    pub fn is_empty(&self) -> bool {
        self.entity_count == 0
    }
}

#[derive(Debug, Error)]
pub enum ProjectionError {}

#[derive(Clone, Copy, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
pub struct EventCursor(u64);

impl EventCursor {
    pub fn start() -> Self {
        Self(0)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EventQuery {
    after: EventCursor,
    limit: usize,
}

impl EventQuery {
    pub fn new(after: EventCursor, limit: usize) -> Result<Self, EventQueryError> {
        if limit == 0 {
            return Err(EventQueryError::ZeroLimit);
        }
        Ok(Self { after, limit })
    }
}

#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum EventQueryError {
    #[error("event page limit must be greater than zero")]
    ZeroLimit,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CausalTransition {
    sequence: u64,
}

impl CausalTransition {
    pub fn sequence(&self) -> u64 {
        self.sequence
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EventPage {
    events: Vec<CausalTransition>,
    next_cursor: EventCursor,
}

impl EventPage {
    pub fn events(&self) -> &[CausalTransition] {
        &self.events
    }

    pub fn next_cursor(&self) -> EventCursor {
        self.next_cursor
    }
}

#[derive(Debug, Error)]
pub enum ReadError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommitRequest {
    _private: (),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommitReceipt {
    _private: (),
}

#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum CommitError {
    #[error("causal transition commits are not enabled in this kernel slice")]
    NotEnabled,
}

impl RecoveryReport {
    pub fn status(&self) -> RecoveryStatus {
        self.status
    }
}

#[derive(Debug, Error)]
pub enum OpenError {
    #[error("cannot access timeline storage")]
    Storage,
    #[error("storage is not a Makise V1 causal timeline")]
    IncompatibleStorage,
    #[error("timeline identity does not match open specification")]
    IdentityMismatch,
}

impl From<rusqlite::Error> for OpenError {
    fn from(_error: rusqlite::Error) -> Self {
        Self::Storage
    }
}

pub struct WorldEngine {
    _connection: Connection,
    timeline_id: TimelineId,
}

impl WorldEngine {
    pub fn open(
        spec: OpenSpec,
        storage: StorageLocation,
    ) -> Result<(Self, RecoveryReport), OpenError> {
        let mut connection = Connection::open(storage.path())?;
        let application_id: i32 =
            connection.pragma_query_value(None, "application_id", |row| row.get(0))?;
        let schema_version: i32 =
            connection.pragma_query_value(None, "user_version", |row| row.get(0))?;

        let status = if application_id == 0 && schema_version == 0 {
            if !is_pristine_storage(&connection)? {
                return Err(OpenError::IncompatibleStorage);
            }
            create_timeline(&mut connection, &spec)?;
            RecoveryStatus::Created
        } else {
            if application_id != APPLICATION_ID || schema_version != SCHEMA_VERSION {
                return Err(OpenError::IncompatibleStorage);
            }
            verify_identity(&connection, &spec)?;
            RecoveryStatus::Recovered
        };

        Ok((
            Self {
                _connection: connection,
                timeline_id: spec.timeline_id,
            },
            RecoveryReport { status },
        ))
    }

    pub fn project(&self, _request: ProjectionRequest) -> Result<Projection, ProjectionError> {
        Ok(Projection {
            timeline_id: self.timeline_id.clone(),
            timeline_version: 0,
            entity_count: 0,
        })
    }

    pub fn commit(&mut self, _request: CommitRequest) -> Result<CommitReceipt, CommitError> {
        Err(CommitError::NotEnabled)
    }

    pub fn events(&self, query: EventQuery) -> Result<EventPage, ReadError> {
        let _limit = query.limit;
        Ok(EventPage {
            events: Vec::new(),
            next_cursor: query.after,
        })
    }
}

fn is_pristine_storage(connection: &Connection) -> Result<bool, rusqlite::Error> {
    let object_count: u64 = connection.query_row(
        "SELECT COUNT(*) FROM sqlite_schema
         WHERE type IN ('table', 'index', 'view', 'trigger')
           AND name NOT LIKE 'sqlite_%'",
        [],
        |row| row.get(0),
    )?;
    Ok(object_count == 0)
}

fn create_timeline(connection: &mut Connection, spec: &OpenSpec) -> Result<(), OpenError> {
    let transaction = connection.transaction()?;
    transaction.execute_batch(
        "CREATE TABLE timeline_metadata (
            singleton INTEGER PRIMARY KEY CHECK (singleton = 1),
            world_id TEXT NOT NULL,
            timeline_id TEXT NOT NULL
        );",
    )?;
    transaction.execute(
        "INSERT INTO timeline_metadata (singleton, world_id, timeline_id)
         VALUES (1, ?1, ?2)",
        params![spec.world_id.0, spec.timeline_id.0],
    )?;
    transaction.pragma_update(None, "application_id", APPLICATION_ID)?;
    transaction.pragma_update(None, "user_version", SCHEMA_VERSION)?;
    transaction.commit()?;
    Ok(())
}

fn verify_identity(connection: &Connection, spec: &OpenSpec) -> Result<(), OpenError> {
    let stored: Option<(String, String)> = connection
        .query_row(
            "SELECT world_id, timeline_id FROM timeline_metadata WHERE singleton = 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;

    match stored {
        Some((world_id, timeline_id))
            if world_id == spec.world_id.0 && timeline_id == spec.timeline_id.0 =>
        {
            Ok(())
        }
        Some(_) => Err(OpenError::IdentityMismatch),
        None => Err(OpenError::IncompatibleStorage),
    }
}
