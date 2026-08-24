//! Persistent causal kernel for Makise V1 timelines.

use std::path::{Path, PathBuf};

use rusqlite::{Connection, OptionalExtension, params};
use thiserror::Error;

mod artifact;
mod circadian;
mod organism;
mod quantity;
mod thermal;

pub use artifact::{
    AdmissionError, AdmissionRecord, ArtifactBundle, ContractParseError, MechanismContract,
    ProgramAbi,
};
pub use circadian::{
    ASLEEP_METABOLISM_UJ_PER_SECOND, AWAKE_METABOLISM_UJ_PER_SECOND, SleepPhase,
    metabolic_demand_uj_per_second,
};
pub use organism::{OrganismError, OrganismState};
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
    simulated_second: i64,
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

    pub fn simulated_second(&self) -> i64 {
        self.simulated_second
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
    request_id: String,
    expected_version: u64,
    advance_to_seconds: i64,
    sleep_intention: bool,
}

impl CommitRequest {
    pub fn accept_sleep_intention(request_id: &str, expected_version: u64) -> Self {
        Self {
            request_id: request_id.to_owned(),
            expected_version,
            advance_to_seconds: 0,
            sleep_intention: true,
        }
    }

    pub fn advance_to(request_id: &str, expected_version: u64, advance_to_seconds: i64) -> Self {
        Self {
            request_id: request_id.to_owned(),
            expected_version,
            advance_to_seconds,
            sleep_intention: false,
        }
    }

    fn payload_digest(&self) -> [u8; 32] {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(self.expected_version.to_be_bytes());
        hasher.update(self.advance_to_seconds.to_be_bytes());
        hasher.update([u8::from(self.sleep_intention)]);
        hasher.finalize().into()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommitReceipt {
    timeline_version: u64,
    replayed_request: bool,
}

impl CommitReceipt {
    pub fn timeline_version(&self) -> u64 {
        self.timeline_version
    }

    pub fn replayed_request(&self) -> bool {
        self.replayed_request
    }
}

#[derive(Debug, Error, PartialEq)]
pub enum CommitError {
    #[error("causal transition commits are not enabled in this kernel slice")]
    NotEnabled,
    #[error("same request_id with different payload")]
    IdempotencyConflict,
    #[error("expected timeline version does not match current head")]
    ExpectedVersionConflict,
    #[error("thermal proposal failed validation before commit: {0}")]
    ProposalRejected(#[from] crate::thermal::ThermalError),
    #[error("sleep transition requires an accepted intention")]
    SleepIntentionRequired,
    #[error("metabolism rejected during advance: {0}")]
    MetabolismRejected(crate::organism::OrganismError),
    #[error("storage failure during commit")]
    Storage(#[from] rusqlite::Error),
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
    head_version: u64,
    receipts: std::collections::HashMap<String, ([u8; 32], CommitReceipt)>,
    reservoirs: Option<ReservoirPair>,
    organism: Option<OrganismState>,
    sleep_phase: circadian::SleepPhase,
    simulated_second: i64,
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
            ensure_runtime_tables(&connection)?;
            RecoveryStatus::Recovered
        };

        let head_version = read_head_version(&connection)?;
        let simulated_second = read_simulated_second(&connection)?;
        let sleep_phase = read_sleep_phase(&connection);
        let organism = read_organism(&connection);

        Ok((
            Self {
                _connection: connection,
                timeline_id: spec.timeline_id,
                head_version,
                receipts: std::collections::HashMap::new(),
                reservoirs: None,
                organism,
                sleep_phase,
                simulated_second,
            },
            RecoveryReport { status },
        ))
    }

    pub fn project(&self, _request: ProjectionRequest) -> Result<Projection, ProjectionError> {
        Ok(Projection {
            timeline_id: self.timeline_id.clone(),
            timeline_version: self.head_version,
            simulated_second: self.simulated_second,
            entity_count: 0,
        })
    }

    pub fn commit(&mut self, request: CommitRequest) -> Result<CommitReceipt, CommitError> {
        if let Some((payload_digest, receipt)) = self.receipts.get(&request.request_id) {
            if *payload_digest != request.payload_digest() {
                return Err(CommitError::IdempotencyConflict);
            }
            let mut replay = receipt.clone();
            replay.replayed_request = true;
            return Ok(replay);
        }
        let stored_receipt: Option<(Vec<u8>, i64)> = self
            ._connection
            .query_row(
                "SELECT payload_digest, timeline_version FROM request_receipts WHERE request_id = ?1",
                params![request.request_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;
        if let Some((stored_digest, stored_version)) = stored_receipt {
            if stored_digest != request.payload_digest().to_vec() {
                return Err(CommitError::IdempotencyConflict);
            }
            let mut replay = CommitReceipt {
                timeline_version: stored_version.max(0) as u64,
                replayed_request: false,
            };
            replay.replayed_request = true;
            return Ok(replay);
        }

        if request.expected_version != self.head_version {
            return Err(CommitError::ExpectedVersionConflict);
        }

        if request.sleep_intention {
            self.sleep_phase = circadian::SleepPhase::Asleep;
        }

        let mut current = self.reservoirs.clone();
        let mut current_organism = self.organism;
        for _second in 0..request.advance_to_seconds.max(0) {
            if current.is_none() {
                current = Some(initial_reservoirs());
            }
            if let Some(pair) = current.as_mut() {
                let proposal = ThermalProposal::one_second(pair, THERMAL_CONDUCTANCE_UJ_PER_MK_S)?;
                apply_transfer(pair, proposal.transfer());
            }
            if current_organism.is_none() {
                current_organism = Some(initial_organism());
            }
            if let Some(organism) = current_organism.as_mut() {
                organism
                    .apply_metabolism(circadian::metabolic_demand_uj_per_second(self.sleep_phase))
                    .map_err(CommitError::MetabolismRejected)?;
            }
        }
        self.reservoirs = current;
        self.organism = current_organism;

        self.simulated_second += request.advance_to_seconds.max(0);

        let receipt = CommitReceipt {
            timeline_version: self.head_version + 1,
            replayed_request: false,
        };
        {
            let transaction = self._connection.transaction()?;
            transaction.execute(
                "INSERT OR REPLACE INTO request_receipts (request_id, payload_digest, timeline_version)
                 VALUES (?1, ?2, ?3)",
                params![request.request_id, request.payload_digest().to_vec(), receipt.timeline_version],
            )?;
            transaction.execute(
                "INSERT INTO timeline_head (singleton, version) VALUES (1, ?1)
                 ON CONFLICT(singleton) DO UPDATE SET version = excluded.version",
                params![receipt.timeline_version],
            )?;
            transaction.execute(
                "INSERT INTO simulated_clock (singleton, second) VALUES (1, ?1)
                 ON CONFLICT(singleton) DO UPDATE SET second = excluded.second",
                params![self.simulated_second],
            )?;
            transaction.execute(
                "INSERT INTO sleep_state (singleton, phase) VALUES (1, ?1)
                 ON CONFLICT(singleton) DO UPDATE SET phase = excluded.phase",
                params![self.sleep_phase.as_canonical_name()],
            )?;
            if let Some(organism) = self.organism {
                transaction.execute(
                    "INSERT INTO organism_state (singleton, chemical_store_uj, core_internal_energy_uj)
                     VALUES (1, ?1, ?2)
                     ON CONFLICT(singleton) DO UPDATE SET
                        chemical_store_uj = excluded.chemical_store_uj,
                        core_internal_energy_uj = excluded.core_internal_energy_uj",
                    params![organism.chemical_store_uj(), organism.core_internal_energy_uj()],
                )?;
            }
            transaction.commit()?;
        }
        self.receipts.insert(
            request.request_id.clone(),
            (request.payload_digest(), receipt.clone()),
        );
        self.head_version = receipt.timeline_version;

        Ok(receipt)
    }

    pub fn organism(&self) -> Option<&OrganismState> {
        self.organism.as_ref()
    }

    pub fn sleep_phase(&self) -> circadian::SleepPhase {
        self.sleep_phase
    }

    pub fn request_sleep_without_intention(&mut self) -> Result<(), CommitError> {
        Err(CommitError::SleepIntentionRequired)
    }

    pub fn events(&self, query: EventQuery) -> Result<EventPage, ReadError> {
        let _limit = query.limit;
        Ok(EventPage {
            events: Vec::new(),
            next_cursor: query.after,
        })
    }
}

const THERMAL_CONDUCTANCE_UJ_PER_MK_S: i64 = 1_000;

fn initial_reservoirs() -> ReservoirPair {
    ReservoirPair::new(
        crate::quantity::ReservoirState::new(20_000_000_000_000, 4_000),
        crate::quantity::ReservoirState::new(10_000_000_000_000, 6_000),
    )
}

fn apply_transfer(pair: &mut ReservoirPair, transfer: &ThermalTransfer) {
    let hot = pair.hot();
    let new_hot_energy = hot.internal_energy_microjoule() + transfer.delta_hot_uj();
    let cold = pair.cold();
    let new_cold_energy = cold.internal_energy_microjoule() + transfer.delta_cold_uj();
    *pair = ReservoirPair::new(
        crate::quantity::ReservoirState::new(
            new_hot_energy,
            hot.heat_capacity_microjoule_per_millikelvin(),
        ),
        crate::quantity::ReservoirState::new(
            new_cold_energy,
            cold.heat_capacity_microjoule_per_millikelvin(),
        ),
    );
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
    transaction.execute_batch(RUNTIME_SCHEMA)?;
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

const RUNTIME_SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS request_receipts (
    request_id TEXT PRIMARY KEY,
    payload_digest BLOB NOT NULL,
    timeline_version INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS timeline_head (
    singleton INTEGER PRIMARY KEY CHECK (singleton = 1),
    version INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS simulated_clock (
    singleton INTEGER PRIMARY KEY CHECK (singleton = 1),
    second INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS sleep_state (
    singleton INTEGER PRIMARY KEY CHECK (singleton = 1),
    phase TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS organism_state (
    singleton INTEGER PRIMARY KEY CHECK (singleton = 1),
    chemical_store_uj INTEGER NOT NULL,
    core_internal_energy_uj INTEGER NOT NULL
);
";

fn ensure_runtime_tables(connection: &Connection) -> Result<(), rusqlite::Error> {
    connection.execute_batch("BEGIN")?;
    connection.execute_batch(RUNTIME_SCHEMA)?;
    connection.execute_batch("COMMIT")
}

fn read_head_version(connection: &Connection) -> Result<u64, rusqlite::Error> {
    let version: Option<i64> = connection
        .query_row(
            "SELECT version FROM timeline_head WHERE singleton = 1",
            [],
            |row| row.get(0),
        )
        .optional()?;
    Ok(version.map_or(0, |value| value.max(0) as u64))
}

fn read_simulated_second(connection: &Connection) -> Result<i64, rusqlite::Error> {
    let second: Option<i64> = connection
        .query_row(
            "SELECT second FROM simulated_clock WHERE singleton = 1",
            [],
            |row| row.get(0),
        )
        .optional()?;
    Ok(second.unwrap_or(0))
}

fn read_sleep_phase(connection: &Connection) -> circadian::SleepPhase {
    connection
        .query_row(
            "SELECT phase FROM sleep_state WHERE singleton = 1",
            [],
            |row| row.get::<_, String>(0),
        )
        .ok()
        .and_then(|phase| circadian::SleepPhase::from_canonical_name(&phase))
        .unwrap_or(circadian::SleepPhase::Awake)
}

fn initial_organism() -> OrganismState {
    OrganismState::new(8_400_000_000_000, 20_000_000_000_000)
}

fn read_organism(connection: &Connection) -> Option<OrganismState> {
    connection
        .query_row(
            "SELECT chemical_store_uj, core_internal_energy_uj FROM organism_state WHERE singleton = 1",
            [],
            |row| {
                Ok(OrganismState::new(row.get(0)?, row.get(1)?))
            },
        )
        .ok()
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
