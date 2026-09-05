//! Persistent causal kernel for Makise V1 timelines.

use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

use rusqlite::{Connection, OptionalExtension, params};
use thiserror::Error;

mod articulation;
mod artifact;
mod atmosphere;
mod balance;
mod blood;
mod cell_cohort;
mod circadian;
mod cognitive;
mod contact;
mod digestion;
mod episodes;
mod fluids;
mod infrastructure;
mod interoception;
mod morphotype;
mod neural_population;
mod organism;
mod physics_island;
mod propagation;
mod quantity;
mod resolution;
mod rigid_body;
mod thermal;
mod walk;

pub use articulation::{
    ArticulatedBody, ArticulationError, JointSpec, MotionStep, MotorTorqueProposal,
};
pub use artifact::{
    AdmissionError, AdmissionRecord, ArtifactBundle, ContractParseError, MechanismContract,
    ProgramAbi,
};
pub use atmosphere::{
    AtmosphereError, DRY_AIR_DENSITY_MG_PER_M3, DRY_AIR_SPECIFIC_HEAT_CV_J_PER_KG_K,
    FREE_CONVECTION_H_W_PER_M2_K, MAX_ROOM_TEMPERATURE_MK, MIN_ROOM_TEMPERATURE_MK, RoomAtmosphere,
    convective_conductance_uj_per_mk_s, heater_energy_uj,
};
pub use balance::{BalanceAssessment, BalanceError, BalanceState, balance_assessment};
pub use blood::{BloodError, BloodState, ENERGY_PER_UMOL_O2_UJ, RQ_DEN, RQ_NUM};
pub use cell_cohort::{CellCohort, FineCell};
pub use circadian::{
    ASLEEP_METABOLISM_UJ_PER_SECOND, AWAKE_METABOLISM_UJ_PER_SECOND,
    SLEEP_DEBT_ONSET_THRESHOLD_SECONDS, SLEEP_DEBT_PER_AWAKE_SECOND,
    SLEEP_RECOVERY_PER_ASLEEP_SECOND, SleepPhase, SleepTransition, awake_metabolism_for_second,
    evaluate_sleep_transition, metabolic_demand_uj_per_second,
};
pub use cognitive::{
    CognitiveDisposition, CognitiveGate, CognitiveGateError, CortexProposal, Intention,
};
pub use contact::{
    BoxCollider, CollisionResolution, CollisionResponseProposal, ContactError, ContactManifold,
    GraspAssessment, GraspRequest, HoldState, contact_proposal, grasp_proposal, hold_projection,
    resolve_collision,
};
pub use digestion::ABSORPTION_RATE_UJ_PER_SECOND;
pub use episodes::{
    CleanBlocker, CleanControlEpisode, CleanObservables, CleanStep, ControlEpisodeError,
    CookAction, CookBlocker, CookControlEpisode, CookObservables, CookPhase, CookStep,
    DressBlocker, DressControlEpisode, DressObservables, DressStep, clean_step, cook_step,
    dress_step,
};
pub use fluids::{
    FluidError, ImmersionVerdict, LiquidContainer, PourRequest, WATER_DENSITY_MG_PER_M3,
    buoyant_force_mgnm_per_s2, hydrostatic_pressure_npa, immersion_verdict, puddle_depth_nm,
};
pub use infrastructure::{
    BRANCH_BREAKER_W, InfrastructureError, KITCHEN_TAP_FLOW_MM3_PER_S, PowerNetwork, WaterNetwork,
};
pub use interoception::{INITIAL_CHEMICAL_STORE_UJ, InteroceptionObservables, advance_sleep_debt};
pub use morphotype::{
    AnatomyEdge, AnatomyNode, Morphotype, MorphotypeDefinition, MorphotypeError, OrganBinding,
};
pub use neural_population::{NeuralPopulation, NeuralPopulationError};
pub use organism::{
    AMBIENT_HEAT_CAPACITY_UJ_PER_MK, BASELINE_AMBIENT_INTERNAL_ENERGY_UJ,
    BASELINE_CORE_INTERNAL_ENERGY_UJ, OrganismError, OrganismState,
    REFERENCE_AMBIENT_TEMPERATURE_MK, REFERENCE_CORE_TEMPERATURE_MK,
};
pub use physics_island::{
    ENVIRONMENT_FLOOR_Y_NM, IslandError, IslandLayout, RestSuspension, advance_awake_bodies,
    advance_island_members, layout_islands, resting_islands, resume_island, suspend_island,
};
pub use propagation::{
    HEARING_THRESHOLD_INTENSITY_FW_PER_M2, LIGHT_MAX_DISTANCE_NM, LIGHT_MIN_DISTANCE_NM,
    ODOUR_MAX_DISTANCE_NM, ODOUR_MIN_DISTANCE_NM, PROPAGATION_REFERENCE_DISTANCE_NM,
    PropagationError, SOUND_MAX_DISTANCE_NM, SOUND_MIN_DISTANCE_NM, illuminance_mlx,
    odour_concentration_mg_per_m3, sound_intensity_fw_per_m2,
};
pub use quantity::{Dimension, Quantity, QuantityError, ReservoirState, StateHash, UnitScale};
pub use resolution::{ResolutionChanged, ResolutionError};
pub use rigid_body::{GravityProposal, RigidBody, RigidBodyError};
pub use thermal::{ReservoirPair, ThermalError, ThermalProposal, ThermalTransfer};
pub use walk::{
    COM_SHIFT_SPEED_NM_PER_S, STRIDE_LENGTH_NM, Side, WalkBlocker, WalkControlEpisode, WalkError,
    WalkPhase, WalkStep, WalkerObservables, step_walk_episode,
};

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

    /// Declared observable: how many canonical transitions precede
    /// this cursor in the durable stream.
    pub fn offset(&self) -> u64 {
        self.0
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
    interval_start_second: i64,
    interval_end_second: i64,
    previous_state_hash: String,
    resulting_state_hash: String,
    request_id: Option<String>,
    evidence: Option<TransitionEvidence>,
    state_hash_version: u8,
}

/// Immutable state anchor for a V1 timeline. Sequence zero is not a
/// transition; it records the exact state hash from which first transition
/// proceeds.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GenesisSnapshot {
    state_hash: String,
    state_hash_version: u8,
    simulated_second: i64,
    sleep_phase: circadian::SleepPhase,
    sleep_intention_accepted: bool,
    sleep_debt_seconds: i64,
    resolution_id: Option<String>,
}

impl GenesisSnapshot {
    pub fn sequence(&self) -> u64 {
        0
    }

    pub fn state_hash(&self) -> &str {
        &self.state_hash
    }

    pub fn state_hash_version(&self) -> u8 {
        self.state_hash_version
    }

    pub fn simulated_second(&self) -> i64 {
        self.simulated_second
    }

    pub fn sleep_phase(&self) -> circadian::SleepPhase {
        self.sleep_phase
    }

    pub fn sleep_intention_accepted(&self) -> bool {
        self.sleep_intention_accepted
    }

    pub fn sleep_debt_seconds(&self) -> i64 {
        self.sleep_debt_seconds
    }

    pub fn resolution_id(&self) -> Option<&str> {
        self.resolution_id.as_deref()
    }
}

fn initial_genesis_state_hash() -> String {
    compute_state_hash_v2(
        0,
        circadian::SleepPhase::Awake,
        false,
        0,
        None,
        None,
        &BTreeMap::new(),
    )
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CausalCause {
    kind: String,
}

impl CausalCause {
    pub fn kind(&self) -> &str {
        &self.kind
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnitDelta {
    quantity: String,
    unit: String,
    before: Option<i64>,
    after: Option<i64>,
}

impl UnitDelta {
    pub fn quantity(&self) -> &str {
        &self.quantity
    }
    pub fn unit(&self) -> &str {
        &self.unit
    }
    pub fn before(&self) -> Option<i64> {
        self.before
    }
    pub fn after(&self) -> Option<i64> {
        self.after
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TransitionUncertainty {
    Unknown { reason: String },
}

impl TransitionUncertainty {
    pub fn is_unknown(&self) -> bool {
        matches!(self, Self::Unknown { .. })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ConservationReport {
    Verified {
        quantity: String,
        unit: String,
        opening: i64,
        external_input: i64,
        closing: i64,
        residual: i64,
    },
    NotEvaluated {
        reason: String,
    },
}

impl ConservationReport {
    pub fn is_verified(&self) -> bool {
        matches!(self, Self::Verified { .. })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct TransitionEvidence {
    causes: Vec<CausalCause>,
    unit_deltas: Vec<UnitDelta>,
    artifact_digests: Vec<String>,
    uncertainty: TransitionUncertainty,
    conservation_report: ConservationReport,
}

impl CausalTransition {
    pub fn sequence(&self) -> u64 {
        self.sequence
    }

    pub fn interval_start_second(&self) -> i64 {
        self.interval_start_second
    }

    pub fn interval_end_second(&self) -> i64 {
        self.interval_end_second
    }

    pub fn previous_state_hash(&self) -> &str {
        &self.previous_state_hash
    }

    pub fn resulting_state_hash(&self) -> &str {
        &self.resulting_state_hash
    }

    /// Request identity is a durable direct cause for transitions written
    /// after the evidence-record expansion. `None` denotes a readable
    /// pre-expansion archive, not an invented cause.
    pub fn request_id(&self) -> Option<&str> {
        self.request_id.as_deref()
    }

    pub fn causes(&self) -> Option<&[CausalCause]> {
        self.evidence
            .as_ref()
            .map(|evidence| evidence.causes.as_slice())
    }

    pub fn unit_deltas(&self) -> Option<&[UnitDelta]> {
        self.evidence
            .as_ref()
            .map(|evidence| evidence.unit_deltas.as_slice())
    }

    pub fn artifact_digests(&self) -> Option<&[String]> {
        self.evidence
            .as_ref()
            .map(|evidence| evidence.artifact_digests.as_slice())
    }

    pub fn uncertainty(&self) -> Option<&TransitionUncertainty> {
        self.evidence.as_ref().map(|evidence| &evidence.uncertainty)
    }

    pub fn conservation_report(&self) -> Option<&ConservationReport> {
        self.evidence
            .as_ref()
            .map(|evidence| &evidence.conservation_report)
    }

    pub fn state_hash_version(&self) -> u8 {
        self.state_hash_version
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
pub enum ReadError {
    #[error("storage failure while reading events")]
    Storage(#[from] rusqlite::Error),
    #[error("stored body row cannot be reconstructed as a valid rigid body")]
    CorruptBodyState,
    #[error("stored transition evidence is malformed")]
    CorruptTransitionEvidence,
}

#[derive(Clone, Debug)]
pub struct CommitRequest {
    request_id: String,
    expected_version: u64,
    advance_to_seconds: i64,
    sleep_intention: bool,
    ingest_uj: Option<i64>,
    resolution_changed: Option<crate::resolution::ResolutionChanged>,
    body: Option<(String, crate::rigid_body::RigidBody)>,
    thermal: Option<(ReservoirPair, ArtifactBundle)>,
}

impl CommitRequest {
    pub fn ingest_food(request_id: &str, expected_version: u64, chemical_energy_uj: i64) -> Self {
        Self {
            request_id: request_id.to_owned(),
            expected_version,
            advance_to_seconds: 0,
            sleep_intention: false,
            ingest_uj: Some(chemical_energy_uj),
            resolution_changed: None,
            body: None,
            thermal: None,
        }
    }

    pub fn accept_sleep_intention(request_id: &str, expected_version: u64) -> Self {
        Self {
            request_id: request_id.to_owned(),
            expected_version,
            advance_to_seconds: 0,
            sleep_intention: true,
            ingest_uj: None,
            resolution_changed: None,
            body: None,
            thermal: None,
        }
    }

    pub fn advance_to(request_id: &str, expected_version: u64, advance_to_seconds: i64) -> Self {
        Self {
            request_id: request_id.to_owned(),
            expected_version,
            advance_to_seconds,
            sleep_intention: false,
            ingest_uj: None,
            resolution_changed: None,
            body: None,
            thermal: None,
        }
    }

    pub fn resolution_changed(
        request_id: &str,
        expected_version: u64,
        resolution_changed: ResolutionChanged,
    ) -> Self {
        Self {
            request_id: request_id.to_owned(),
            expected_version,
            advance_to_seconds: 0,
            sleep_intention: false,
            ingest_uj: None,
            resolution_changed: Some(resolution_changed),
            body: None,
            thermal: None,
        }
    }

    /// Stages a named metric rigid body as durable timeline state. The
    /// placement is one authoritative delta: retrying the identical
    /// request replays its receipt, a different pose under the same
    /// request id is an idempotency conflict.
    pub fn place_body(
        request_id: &str,
        expected_version: u64,
        body_id: impl Into<String>,
        body: crate::rigid_body::RigidBody,
    ) -> Self {
        Self {
            request_id: request_id.to_owned(),
            expected_version,
            advance_to_seconds: 0,
            sleep_intention: false,
            ingest_uj: None,
            resolution_changed: None,
            body: Some((body_id.into(), body)),
            thermal: None,
        }
    }

    pub fn thermal_exchange(
        request_id: &str,
        expected_version: u64,
        pair: ReservoirPair,
        artifact: ArtifactBundle,
    ) -> Self {
        Self {
            request_id: request_id.to_owned(),
            expected_version,
            advance_to_seconds: 0,
            sleep_intention: false,
            ingest_uj: None,
            resolution_changed: None,
            body: None,
            thermal: Some((pair, artifact)),
        }
    }

    fn payload_digest(&self) -> [u8; 32] {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(self.expected_version.to_be_bytes());
        hasher.update(self.advance_to_seconds.to_be_bytes());
        hasher.update([u8::from(self.sleep_intention)]);
        hasher.update(self.ingest_uj.unwrap_or(0).to_be_bytes());
        if let Some(resolution) = self.resolution_changed {
            hasher.update(b"resolution-changed");
            hasher.update(resolution.canonical_bytes());
        } else {
            hasher.update(b"no-resolution-change");
        }
        match &self.body {
            Some((body_id, body)) => {
                hasher.update(b"body");
                hasher.update(body_id.as_bytes());
                for value in [
                    body.mass_mg(),
                    body.position_nm()[0],
                    body.position_nm()[1],
                    body.position_nm()[2],
                    body.velocity_nm_per_s()[0],
                    body.velocity_nm_per_s()[1],
                    body.velocity_nm_per_s()[2],
                    body.center_of_mass_offset_nm()[0],
                    body.center_of_mass_offset_nm()[1],
                    body.center_of_mass_offset_nm()[2],
                    body.principal_inertia_mgm2()[0],
                    body.principal_inertia_mgm2()[1],
                    body.principal_inertia_mgm2()[2],
                    body.angular_velocity_urad_per_s()[0],
                    body.angular_velocity_urad_per_s()[1],
                    body.angular_velocity_urad_per_s()[2],
                ] {
                    hasher.update(value.to_be_bytes());
                }
            }
            None => hasher.update(b"no-body"),
        }
        if let Some((pair, artifact)) = &self.thermal {
            hasher.update(b"thermal");
            hasher.update(pair.hot().internal_energy_microjoule().to_be_bytes());
            hasher.update(pair.cold().internal_energy_microjoule().to_be_bytes());
            hasher.update(artifact.program_bytes());
        } else {
            hasher.update(b"no-thermal");
        }
        hasher.finalize().into()
    }

    fn cause_kind(&self) -> &'static str {
        if self.thermal.is_some() {
            return "thermal_exchange";
        }
        if self.body.is_some() {
            "place_body"
        } else if self.resolution_changed.is_some() {
            "resolution_changed"
        } else if self.ingest_uj.is_some() {
            "ingest_food"
        } else if self.sleep_intention {
            "accept_sleep_intention"
        } else {
            "advance_time"
        }
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
    #[error("ingestion amount must be positive")]
    InvalidIngestion,
    #[error("ingestion exceeds declared chemical capacity")]
    DigestiveCapacityExceeded,
    #[error("metabolism rejected during advance: {0}")]
    MetabolismRejected(crate::organism::OrganismError),
    #[error("resolution change rejected before commit: {0}")]
    ResolutionRejected(#[from] crate::resolution::ResolutionError),
    #[error("body identifier cannot be empty")]
    InvalidBodyId,
    #[error("storage failure during commit")]
    Storage(#[from] rusqlite::Error),
    #[error("timeline is in SafeStop: {0}")]
    SafeStopped(String),
    #[error("artifact admission failed: {0}")]
    ArtifactRejected(#[from] AdmissionError),
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
    #[error("committed transition chain failed integrity verification")]
    CorruptTransitionChain,
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
    sleep_intention_accepted: bool,
    sleep_debt_seconds: i64,
    simulated_second: i64,
    resolution_id: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SafeStop {
    reason: String,
}

impl SafeStop {
    pub fn reason(&self) -> &str {
        &self.reason
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReplayResult {
    timeline_version: u64,
    resulting_state_hash: String,
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
            verify_transition_chain(&connection).map_err(|error| {
                if error == rusqlite::Error::QueryReturnedNoRows {
                    OpenError::CorruptTransitionChain
                } else {
                    OpenError::Storage
                }
            })?;
            RecoveryStatus::Recovered
        };

        let head_version = read_head_version(&connection)?;
        let simulated_second = read_simulated_second(&connection)?;
        let sleep_phase = read_sleep_phase(&connection);
        let organism = read_organism(&connection);
        let sleep_intention_accepted = read_sleep_intention(&connection);
        let sleep_debt_seconds = read_sleep_debt(&connection);
        let resolution_id = read_resolution_id(&connection);
        let reservoirs = read_thermal_reservoirs(&connection);

        Ok((
            Self {
                _connection: connection,
                timeline_id: spec.timeline_id,
                head_version,
                receipts: std::collections::HashMap::new(),
                reservoirs,
                organism,
                sleep_phase,
                sleep_intention_accepted,
                sleep_debt_seconds,
                simulated_second,
                resolution_id,
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

    pub fn simulated_second(&self) -> i64 {
        self.simulated_second
    }

    pub fn safe_stop(&self) -> Result<Option<SafeStop>, ReadError> {
        self._connection
            .query_row(
                "SELECT reason FROM timeline_safe_stop WHERE singleton = 1",
                [],
                |row| {
                    Ok(SafeStop {
                        reason: row.get(0)?,
                    })
                },
            )
            .optional()
            .map_err(ReadError::Storage)
    }

    pub fn fast_replay(&self) -> Result<ReplayResult, ReadError> {
        let rows = self._connection.prepare("SELECT t.resulting_state_hash, t.unit_deltas, e.hot_capacity_uj_per_mk, e.cold_capacity_uj_per_mk FROM causal_transitions t LEFT JOIN thermal_execution e ON e.sequence=t.sequence ORDER BY t.sequence")?.query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, Option<i64>>(2)?, row.get::<_, Option<i64>>(3)?)))?.collect::<Result<Vec<_>, _>>()?;
        let mut last = initial_genesis_state_hash();
        for (stored_hash, deltas, hot_capacity, cold_capacity) in rows {
            let (hot, cold) = thermal_delta_after(&deltas)?;
            let (hot_capacity, cold_capacity) = match (hot_capacity, cold_capacity) {
                (Some(h), Some(c)) => (h, c),
                _ => return Err(ReadError::CorruptTransitionEvidence),
            };
            let pair = ReservoirPair::new(
                crate::quantity::ReservoirState::new(hot, hot_capacity),
                crate::quantity::ReservoirState::new(cold, cold_capacity),
            );
            let hash = compute_state_hash_v3(
                0,
                circadian::SleepPhase::Awake,
                false,
                0,
                None,
                None,
                &BTreeMap::new(),
                Some(&pair),
            );
            if hash != stored_hash {
                return Err(ReadError::CorruptTransitionEvidence);
            }
            last = hash;
        }
        Ok(ReplayResult {
            timeline_version: self.head_version,
            resulting_state_hash: last,
        })
    }

    pub fn audit_replay(&mut self) -> Result<ReplayResult, ReadError> {
        let rows = self._connection.prepare("SELECT t.resulting_state_hash, t.unit_deltas, t.conservation_report, e.artifact_digest, e.hot_energy_uj, e.hot_capacity_uj_per_mk, e.cold_energy_uj, e.cold_capacity_uj_per_mk, e.conductance_uj_per_mk_s FROM causal_transitions t LEFT JOIN thermal_execution e ON e.sequence=t.sequence ORDER BY t.sequence")?.query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?, row.get::<_, Option<String>>(3)?, row.get::<_, Option<i64>>(4)?, row.get::<_, Option<i64>>(5)?, row.get::<_, Option<i64>>(6)?, row.get::<_, Option<i64>>(7)?, row.get::<_, Option<i64>>(8)?)))?.collect::<Result<Vec<_>, _>>()?;
        let mut last = initial_genesis_state_hash();
        for (
            stored_hash,
            deltas,
            conservation,
            digest,
            hot_e,
            hot_c,
            cold_e,
            cold_c,
            conductance,
        ) in rows
        {
            let (digest, hot_e, hot_c, cold_e, cold_c, conductance) =
                match (digest, hot_e, hot_c, cold_e, cold_c, conductance) {
                    (Some(d), Some(a), Some(b), Some(c), Some(e), Some(f)) => (d, a, b, c, e, f),
                    _ => {
                        self.durable_safe_stop("unsupported_audit_transition")?;
                        return Err(ReadError::CorruptTransitionEvidence);
                    }
                };
            let bytes: Option<Vec<u8>> = self
                ._connection
                .query_row(
                    "SELECT program_bytes FROM artifact_archive WHERE digest=?1",
                    params![digest],
                    |row| row.get(0),
                )
                .optional()?;
            if bytes
                .as_deref()
                .is_none_or(|bytes| sha256_digest(bytes) != digest)
            {
                self.durable_safe_stop("missing_or_mismatched_artifact")?;
                return Err(ReadError::CorruptTransitionEvidence);
            }
            let archived_conductance = bytes
                .as_deref()
                .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(bytes).ok())
                .and_then(|value| {
                    value
                        .get("conductance_uj_per_mk_s")
                        .and_then(serde_json::Value::as_i64)
                });
            if archived_conductance != Some(conductance) {
                self.durable_safe_stop("audit_input_mismatch")?;
                return Err(ReadError::CorruptTransitionEvidence);
            }
            let mut pair = ReservoirPair::new(
                crate::quantity::ReservoirState::new(hot_e, hot_c),
                crate::quantity::ReservoirState::new(cold_e, cold_c),
            );
            let proposal = ThermalProposal::one_second(&pair, conductance)
                .map_err(|_| ReadError::CorruptTransitionEvidence)?;
            apply_transfer(&mut pair, proposal.transfer());
            if thermal_delta_after(&deltas)?
                != (
                    pair.hot().internal_energy_microjoule(),
                    pair.cold().internal_energy_microjoule(),
                )
                || !verified_thermal_conservation(
                    &conservation,
                    hot_e + cold_e,
                    pair.hot().internal_energy_microjoule()
                        + pair.cold().internal_energy_microjoule(),
                )
            {
                self.durable_safe_stop("audit_delta_or_conservation_mismatch")?;
                return Err(ReadError::CorruptTransitionEvidence);
            }
            let hash = compute_state_hash_v3(
                0,
                circadian::SleepPhase::Awake,
                false,
                0,
                None,
                None,
                &BTreeMap::new(),
                Some(&pair),
            );
            if hash != stored_hash {
                self.durable_safe_stop("audit_hash_mismatch")?;
                return Err(ReadError::CorruptTransitionEvidence);
            }
            last = hash;
        }
        Ok(ReplayResult {
            timeline_version: self.head_version,
            resulting_state_hash: last,
        })
    }

    fn durable_safe_stop(&mut self, reason: &str) -> Result<(), ReadError> {
        self._connection.execute("INSERT INTO timeline_safe_stop (singleton, reason) VALUES (1, ?1) ON CONFLICT(singleton) DO UPDATE SET reason = excluded.reason", params![reason])?;
        Ok(())
    }

    /// Returns sequence-zero state anchor. A legacy timeline can lack one
    /// because its original initial state was never durable.
    pub fn genesis(&self) -> Result<Option<GenesisSnapshot>, ReadError> {
        self._connection
            .query_row(
                "SELECT state_hash, state_hash_version, simulated_second, sleep_phase,
                        sleep_intention_accepted, sleep_debt_seconds, resolution_id
                 FROM genesis_snapshot WHERE singleton = 1",
                [],
                |row| {
                    let phase: String = row.get(3)?;
                    let phase = circadian::SleepPhase::from_canonical_name(&phase)
                        .ok_or(rusqlite::Error::InvalidQuery)?;
                    Ok(GenesisSnapshot {
                        state_hash: row.get(0)?,
                        state_hash_version: row.get::<_, i64>(1)?.clamp(1, 3) as u8,
                        simulated_second: row.get(2)?,
                        sleep_phase: phase,
                        sleep_intention_accepted: row.get::<_, i64>(4)? != 0,
                        sleep_debt_seconds: row.get(5)?,
                        resolution_id: row.get(6)?,
                    })
                },
            )
            .optional()
            .map_err(ReadError::Storage)
    }

    pub fn commit(&mut self, request: CommitRequest) -> Result<CommitReceipt, CommitError> {
        if let Some(stop) = self
            .safe_stop()
            .map_err(|_| CommitError::Storage(rusqlite::Error::InvalidQuery))?
        {
            return Err(CommitError::SafeStopped(stop.reason));
        }
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

        if let Some((body_id, _)) = &request.body
            && body_id.trim().is_empty()
        {
            return Err(CommitError::InvalidBodyId);
        }

        let mut current_resolution_id = self.resolution_id.clone();
        if let Some(resolution) = request.resolution_changed {
            resolution.validate()?;
            current_resolution_id = Some(resolution.to_resolution_id().to_owned());
        }

        let mut current_sleep_intention = self.sleep_intention_accepted || request.sleep_intention;

        let mut current = self.reservoirs.clone();
        if let Some((pair, artifact)) = &request.thermal {
            artifact.admit()?;
            let conductance = artifact.thermal_exchange_conductance_uj_per_mk_s().ok_or(
                CommitError::ProposalRejected(ThermalError::OutsideValidityRange),
            )?;
            let mut result = pair.clone();
            let proposal = ThermalProposal::one_second(&result, conductance)?;
            apply_transfer(&mut result, proposal.transfer());
            current = Some(result);
        }
        let mut current_organism = self.organism.clone();
        if current_organism.is_none()
            && ((request.advance_to_seconds == 0 && request.thermal.is_none())
                || request.ingest_uj.is_some())
        {
            current_organism = Some(initial_organism());
        }
        let mut current_sleep_debt = self.sleep_debt_seconds;
        let mut current_sleep_phase = self.sleep_phase;
        for _second in 0..request.advance_to_seconds.max(0) {
            let canonical_second = self.simulated_second + _second;
            match circadian::evaluate_sleep_transition(
                current_sleep_phase,
                current_sleep_intention,
                current_sleep_debt,
                canonical_second,
            ) {
                circadian::SleepTransition::FallAsleep => {
                    current_sleep_phase = circadian::SleepPhase::Asleep;
                }
                circadian::SleepTransition::WakeUp => {
                    current_sleep_phase = circadian::SleepPhase::Awake;
                    current_sleep_intention = false;
                }
                circadian::SleepTransition::None => {}
            }
            if let Some(pair) = current.as_mut() {
                let proposal = ThermalProposal::one_second(pair, THERMAL_CONDUCTANCE_UJ_PER_MK_S)?;
                apply_transfer(pair, proposal.transfer());
            }
            if current_organism.is_none() {
                current_organism = Some(initial_organism());
            }
            if let Some(organism) = current_organism.as_mut() {
                crate::digestion::absorb_one_second(organism);
                organism
                    .apply_ambient_exchange()
                    .map_err(CommitError::MetabolismRejected)?;
                let demand = if current_sleep_phase == circadian::SleepPhase::Asleep {
                    circadian::metabolic_demand_uj_per_second(circadian::SleepPhase::Asleep)
                } else {
                    circadian::awake_metabolism_for_second(self.simulated_second + _second)
                };
                // Blood gas exchange must succeed before chemical burn: O₂ overdraft
                // rejects without partial metabolism (INVARIANTS §18, §42).
                organism
                    .apply_gas_exchange_for_second(demand)
                    .map_err(CommitError::MetabolismRejected)?;
                organism
                    .apply_metabolism(demand)
                    .map_err(CommitError::MetabolismRejected)?;
            }
            current_sleep_debt =
                interoception::advance_sleep_debt(current_sleep_debt, current_sleep_phase);
        }
        if let (Some(energy), Some(organism)) = (request.ingest_uj, current_organism.as_mut()) {
            if energy <= 0 {
                return Err(CommitError::InvalidIngestion);
            }
            organism
                .stage_ingestion(energy)
                .map_err(|error| match error {
                    crate::organism::OrganismError::DigestiveCapacityExceeded => {
                        CommitError::DigestiveCapacityExceeded
                    }
                    crate::organism::OrganismError::Overflow => CommitError::InvalidIngestion,
                    other => CommitError::MetabolismRejected(other),
                })?;
        }
        let candidate_simulated_second = self.simulated_second + request.advance_to_seconds.max(0);
        let previous_bodies = read_body_states_for_hash(&self._connection)?;
        let mut candidate_bodies = previous_bodies.clone();
        if let Some((body_id, body)) = &request.body {
            replace_body_state(&mut candidate_bodies, body_id, body);
        }
        let previous_hash_version = self.previous_state_hash_version()?;
        let previous_state_hash =
            self.compute_state_hash_for_previous(previous_hash_version, &previous_bodies);
        let state_hash_version = if request.thermal.is_some() { 3 } else { 2 };
        let resulting_state_hash = if state_hash_version == 3 {
            compute_state_hash_v3(
                candidate_simulated_second,
                current_sleep_phase,
                current_sleep_intention,
                current_sleep_debt,
                current_organism.as_ref(),
                current_resolution_id.as_deref(),
                &candidate_bodies,
                current.as_ref(),
            )
        } else {
            compute_state_hash_v2(
                candidate_simulated_second,
                current_sleep_phase,
                current_sleep_intention,
                current_sleep_debt,
                current_organism.as_ref(),
                current_resolution_id.as_deref(),
                &candidate_bodies,
            )
        };
        let mut evidence = transition_evidence(
            &request,
            EvidenceState::new(
                self.simulated_second,
                self.sleep_phase,
                self.sleep_intention_accepted,
                self.sleep_debt_seconds,
                self.organism.as_ref(),
                self.resolution_id.as_deref(),
                &previous_bodies,
            ),
            EvidenceState::new(
                candidate_simulated_second,
                current_sleep_phase,
                current_sleep_intention,
                current_sleep_debt,
                current_organism.as_ref(),
                current_resolution_id.as_deref(),
                &candidate_bodies,
            ),
        );
        if let Some((before, artifact)) = &request.thermal {
            let after = current
                .as_ref()
                .expect("thermal request defines reservoir state");
            evidence.unit_deltas.extend([
                UnitDelta {
                    quantity: "reservoir.hot.internal_energy".to_owned(),
                    unit: "uJ".to_owned(),
                    before: Some(before.hot().internal_energy_microjoule()),
                    after: Some(after.hot().internal_energy_microjoule()),
                },
                UnitDelta {
                    quantity: "reservoir.cold.internal_energy".to_owned(),
                    unit: "uJ".to_owned(),
                    before: Some(before.cold().internal_energy_microjoule()),
                    after: Some(after.cold().internal_energy_microjoule()),
                },
            ]);
            evidence.artifact_digests = vec![sha256_digest(artifact.program_bytes())];
            evidence.conservation_report = ConservationReport::Verified {
                quantity: "energy.total".to_owned(),
                unit: "uJ".to_owned(),
                opening: before.hot().internal_energy_microjoule()
                    + before.cold().internal_energy_microjoule(),
                external_input: 0,
                closing: after.hot().internal_energy_microjoule()
                    + after.cold().internal_energy_microjoule(),
                residual: 0,
            };
        }

        let receipt = CommitReceipt {
            timeline_version: self.head_version + 1,
            replayed_request: false,
        };
        {
            let transaction = self._connection.transaction()?;
            if let Some((_, artifact)) = &request.thermal {
                let digest = sha256_digest(artifact.program_bytes());
                transaction.execute(
                    "INSERT INTO artifact_archive (digest, program_bytes) VALUES (?1, ?2)
                     ON CONFLICT(digest) DO NOTHING",
                    params![digest, artifact.program_bytes()],
                )?;
            }
            if let Some((pair, artifact)) = &request.thermal {
                transaction.execute(
                    "INSERT INTO thermal_execution (sequence, artifact_digest, hot_energy_uj, hot_capacity_uj_per_mk, cold_energy_uj, cold_capacity_uj_per_mk, conductance_uj_per_mk_s) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                    params![receipt.timeline_version as i64, sha256_digest(artifact.program_bytes()), pair.hot().internal_energy_microjoule(), pair.hot().heat_capacity_microjoule_per_millikelvin(), pair.cold().internal_energy_microjoule(), pair.cold().heat_capacity_microjoule_per_millikelvin(), artifact.thermal_exchange_conductance_uj_per_mk_s()],
                )?;
            }
            if let Some(pair) = &current {
                transaction.execute(
                    "INSERT INTO thermal_reservoir_state (singleton, hot_energy_uj, hot_capacity_uj_per_mk, cold_energy_uj, cold_capacity_uj_per_mk) VALUES (1, ?1, ?2, ?3, ?4) ON CONFLICT(singleton) DO UPDATE SET hot_energy_uj=excluded.hot_energy_uj, hot_capacity_uj_per_mk=excluded.hot_capacity_uj_per_mk, cold_energy_uj=excluded.cold_energy_uj, cold_capacity_uj_per_mk=excluded.cold_capacity_uj_per_mk",
                    params![pair.hot().internal_energy_microjoule(), pair.hot().heat_capacity_microjoule_per_millikelvin(), pair.cold().internal_energy_microjoule(), pair.cold().heat_capacity_microjoule_per_millikelvin()],
                )?;
            }
            if request.advance_to_seconds > 0 {
                transaction.execute(
                    "INSERT INTO causal_transitions
                     (sequence, interval_start_second, interval_end_second,
                      previous_state_hash, resulting_state_hash, request_id,
                      causes, unit_deltas, artifact_digests, uncertainty,
                      conservation_report, state_hash_version)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                    params![
                        receipt.timeline_version as i64,
                        self.simulated_second,
                        candidate_simulated_second,
                        previous_state_hash,
                        resulting_state_hash,
                        request.request_id,
                        encode_causes(&evidence.causes),
                        encode_unit_deltas(&evidence.unit_deltas),
                        encode_artifact_digests(&evidence.artifact_digests),
                        encode_uncertainty(&evidence.uncertainty),
                        encode_conservation_report(&evidence.conservation_report),
                        i64::from(state_hash_version),
                    ],
                )?;
            }
            if request.advance_to_seconds <= 0 {
                if self.head_version == 0 {
                    transaction.execute(
                        "INSERT INTO causal_transitions
                         (sequence, interval_start_second, interval_end_second,
                          previous_state_hash, resulting_state_hash, request_id,
                          causes, unit_deltas, artifact_digests, uncertainty,
                          conservation_report, state_hash_version)
                         VALUES (1, ?1, ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                        params![
                            self.simulated_second,
                            previous_state_hash,
                            resulting_state_hash,
                            request.request_id,
                            encode_causes(&evidence.causes),
                            encode_unit_deltas(&evidence.unit_deltas),
                            encode_artifact_digests(&evidence.artifact_digests),
                            encode_uncertainty(&evidence.uncertainty),
                            encode_conservation_report(&evidence.conservation_report),
                            i64::from(state_hash_version),
                        ],
                    )?;
                } else {
                    let last_transition: i64 = transaction.query_row(
                        "SELECT interval_end_second FROM causal_transitions
                         ORDER BY sequence DESC LIMIT 1",
                        [],
                        |row| row.get(0),
                    )?;
                    transaction.execute(
                        "INSERT INTO causal_transitions
                         (sequence, interval_start_second, interval_end_second,
                          previous_state_hash, resulting_state_hash, request_id,
                          causes, unit_deltas, artifact_digests, uncertainty,
                          conservation_report, state_hash_version)
                         VALUES (?1, ?2, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                        params![
                            receipt.timeline_version as i64,
                            last_transition,
                            previous_state_hash,
                            resulting_state_hash,
                            request.request_id,
                            encode_causes(&evidence.causes),
                            encode_unit_deltas(&evidence.unit_deltas),
                            encode_artifact_digests(&evidence.artifact_digests),
                            encode_uncertainty(&evidence.uncertainty),
                            encode_conservation_report(&evidence.conservation_report),
                            i64::from(state_hash_version),
                        ],
                    )?;
                }
            }
            transaction.execute(
                "INSERT INTO simulated_clock (singleton, second) VALUES (1, ?1)
                 ON CONFLICT(singleton) DO UPDATE SET second = excluded.second",
                params![candidate_simulated_second],
            )?;
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
            if let Some((body_id, body)) = &request.body {
                transaction.execute(
                    "INSERT INTO body_state (
                        body_id, mass_mg,
                        position_x_nm, position_y_nm, position_z_nm,
                        velocity_x_nm_per_s, velocity_y_nm_per_s, velocity_z_nm_per_s,
                        com_offset_x_nm, com_offset_y_nm, com_offset_z_nm,
                        inertia_x_mgm2, inertia_y_mgm2, inertia_z_mgm2,
                        angular_x_urad_per_s, angular_y_urad_per_s, angular_z_urad_per_s
                     ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)
                     ON CONFLICT(body_id) DO UPDATE SET
                        mass_mg = excluded.mass_mg,
                        position_x_nm = excluded.position_x_nm,
                        position_y_nm = excluded.position_y_nm,
                        position_z_nm = excluded.position_z_nm,
                        velocity_x_nm_per_s = excluded.velocity_x_nm_per_s,
                        velocity_y_nm_per_s = excluded.velocity_y_nm_per_s,
                        velocity_z_nm_per_s = excluded.velocity_z_nm_per_s,
                        com_offset_x_nm = excluded.com_offset_x_nm,
                        com_offset_y_nm = excluded.com_offset_y_nm,
                        com_offset_z_nm = excluded.com_offset_z_nm,
                        inertia_x_mgm2 = excluded.inertia_x_mgm2,
                        inertia_y_mgm2 = excluded.inertia_y_mgm2,
                        inertia_z_mgm2 = excluded.inertia_z_mgm2,
                        angular_x_urad_per_s = excluded.angular_x_urad_per_s,
                        angular_y_urad_per_s = excluded.angular_y_urad_per_s,
                        angular_z_urad_per_s = excluded.angular_z_urad_per_s",
                    params![
                        body_id,
                        body.mass_mg(),
                        body.position_nm()[0],
                        body.position_nm()[1],
                        body.position_nm()[2],
                        body.velocity_nm_per_s()[0],
                        body.velocity_nm_per_s()[1],
                        body.velocity_nm_per_s()[2],
                        body.center_of_mass_offset_nm()[0],
                        body.center_of_mass_offset_nm()[1],
                        body.center_of_mass_offset_nm()[2],
                        body.principal_inertia_mgm2()[0],
                        body.principal_inertia_mgm2()[1],
                        body.principal_inertia_mgm2()[2],
                        body.angular_velocity_urad_per_s()[0],
                        body.angular_velocity_urad_per_s()[1],
                        body.angular_velocity_urad_per_s()[2],
                    ],
                )?;
            }
            transaction.execute(
                "INSERT INTO sleep_state (singleton, phase) VALUES (1, ?1)
                 ON CONFLICT(singleton) DO UPDATE SET phase = excluded.phase",
                params![current_sleep_phase.as_canonical_name()],
            )?;
            transaction.execute(
                "INSERT INTO sleep_intention (singleton, accepted) VALUES (1, ?1)
                 ON CONFLICT(singleton) DO UPDATE SET accepted = excluded.accepted",
                params![i64::from(current_sleep_intention)],
            )?;
            transaction.execute(
                "INSERT INTO sleep_debt (singleton, debt_seconds) VALUES (1, ?1)
                 ON CONFLICT(singleton) DO UPDATE SET debt_seconds = excluded.debt_seconds",
                params![current_sleep_debt],
            )?;
            if let Some(resolution_id) = current_resolution_id.as_deref() {
                transaction.execute(
                    "INSERT INTO active_resolution (singleton, resolution_id) VALUES (1, ?1)
                     ON CONFLICT(singleton) DO UPDATE SET resolution_id = excluded.resolution_id",
                    params![resolution_id],
                )?;
            }
            if let Some(organism) = &current_organism {
                transaction.execute(
                "INSERT INTO organism_state (singleton, chemical_store_uj, digestion_buffer_uj, core_internal_energy_uj, ambient_internal_energy_uj, ambient_heat_capacity_uj_per_mk, blood_volume_mm3, hb_tetramer_umol, arterial_o2_umol, venous_co2_umol, map_mpa, lung_diffusion_umol_per_s)
                 VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
                 ON CONFLICT(singleton) DO UPDATE SET
                    chemical_store_uj = excluded.chemical_store_uj,
                    digestion_buffer_uj = excluded.digestion_buffer_uj,
                    core_internal_energy_uj = excluded.core_internal_energy_uj,
                    ambient_internal_energy_uj = excluded.ambient_internal_energy_uj,
                    ambient_heat_capacity_uj_per_mk = excluded.ambient_heat_capacity_uj_per_mk,
                    blood_volume_mm3 = excluded.blood_volume_mm3,
                    hb_tetramer_umol = excluded.hb_tetramer_umol,
                    arterial_o2_umol = excluded.arterial_o2_umol,
                    venous_co2_umol = excluded.venous_co2_umol,
                    map_mpa = excluded.map_mpa,
                    lung_diffusion_umol_per_s = excluded.lung_diffusion_umol_per_s",
                params![
                    organism.chemical_store_uj(),
                    organism.digestion_buffer_uj(),
                    organism.core_internal_energy_uj(),
                    organism.ambient_internal_energy_uj(),
                    organism
                        .ambient_reservoir()
                        .heat_capacity_microjoule_per_millikelvin(),
                    organism.blood_volume_mm3(),
                    organism.blood().hb_tetramer_umol(),
                    organism.arterial_o2_umol(),
                    organism.venous_co2_umol(),
                    organism.mean_arterial_pressure_mpa(),
                    organism.blood().lung_diffusion_umol_per_s(),
                ],
            )?;
            }
            transaction.commit()?;
        }
        self.reservoirs = current;
        self.organism = current_organism;
        self.sleep_debt_seconds = current_sleep_debt;
        self.sleep_phase = current_sleep_phase;
        self.sleep_intention_accepted = current_sleep_intention;
        self.simulated_second = candidate_simulated_second;
        self.resolution_id = current_resolution_id;
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

    /// Reads one durable body record; a row that cannot be
    /// reconstructed as a valid rigid body is a typed corruption, not
    /// silent garbage (INVARIANTS §42).
    pub fn body(&self, body_id: &str) -> Result<Option<RigidBody>, ReadError> {
        let row: Option<[i64; 16]> = self
            ._connection
            .query_row(
                "SELECT mass_mg,
                        position_x_nm, position_y_nm, position_z_nm,
                        velocity_x_nm_per_s, velocity_y_nm_per_s, velocity_z_nm_per_s,
                        com_offset_x_nm, com_offset_y_nm, com_offset_z_nm,
                        inertia_x_mgm2, inertia_y_mgm2, inertia_z_mgm2,
                        angular_x_urad_per_s, angular_y_urad_per_s, angular_z_urad_per_s
                 FROM body_state WHERE body_id = ?1",
                params![body_id],
                |row| {
                    Ok([
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                        row.get(6)?,
                        row.get(7)?,
                        row.get(8)?,
                        row.get(9)?,
                        row.get(10)?,
                        row.get(11)?,
                        row.get(12)?,
                        row.get(13)?,
                        row.get(14)?,
                        row.get(15)?,
                    ])
                },
            )
            .optional()?;
        let Some(values) = row else {
            return Ok(None);
        };
        let mass = values[0];
        let position = [values[1], values[2], values[3]];
        let velocity = [values[4], values[5], values[6]];
        let com_offset = [values[7], values[8], values[9]];
        let inertia = [values[10], values[11], values[12]];
        let angular = [values[13], values[14], values[15]];
        RigidBody::new(mass, position, velocity, com_offset, inertia, angular)
            .map(Some)
            .map_err(|_| ReadError::CorruptBodyState)
    }

    /// Lists durable body identifiers in insertion order — stable
    /// across restart because rows carry implicit rowids.
    pub fn body_ids(&self) -> Result<Vec<String>, ReadError> {
        let mut statement = self
            ._connection
            .prepare("SELECT body_id FROM body_state ORDER BY rowid ASC")?;
        let rows = statement.query_map([], |row| row.get::<_, String>(0))?;
        let mut ids = Vec::new();
        for row in rows {
            ids.push(row?);
        }
        Ok(ids)
    }

    pub fn sleep_phase(&self) -> circadian::SleepPhase {
        self.sleep_phase
    }

    pub fn resolution_id(&self) -> Option<&str> {
        self.resolution_id.as_deref()
    }

    pub fn interoception(&self) -> Option<InteroceptionObservables> {
        self.organism.as_ref().map(|organism| {
            InteroceptionObservables::from_state(
                organism,
                self.sleep_debt_seconds,
                self.sleep_phase,
            )
        })
    }

    pub fn request_sleep_without_intention(&mut self) -> Result<(), CommitError> {
        Err(CommitError::SleepIntentionRequired)
    }

    pub fn events(&self, query: EventQuery) -> Result<EventPage, ReadError> {
        let mut statement = self._connection.prepare(
            "SELECT sequence, interval_start_second, interval_end_second,
                    COALESCE(previous_state_hash,''), COALESCE(resulting_state_hash,''),
                    COALESCE(request_id,''), COALESCE(causes,''), COALESCE(unit_deltas,''),
                    COALESCE(artifact_digests,''), COALESCE(uncertainty,''),
                    COALESCE(conservation_report,''), COALESCE(state_hash_version, 1)
             FROM causal_transitions WHERE sequence > ?1 ORDER BY sequence ASC LIMIT ?2",
        )?;
        let rows =
            statement.query_map(params![query.after.0 as i64, query.limit as i64], |row| {
                Ok((
                    row.get::<_, i64>(0)?.max(0) as u64,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, String>(7)?,
                    row.get::<_, String>(8)?,
                    row.get::<_, String>(9)?,
                    row.get::<_, String>(10)?,
                    row.get::<_, i64>(11)?,
                ))
            })?;
        let mut events = Vec::new();
        for row in rows {
            let (
                sequence,
                interval_start_second,
                interval_end_second,
                previous_state_hash,
                resulting_state_hash,
                request_id,
                causes,
                unit_deltas,
                artifact_digests,
                uncertainty,
                conservation_report,
                state_hash_version,
            ) = row?;
            let state_hash_version = u8::try_from(state_hash_version)
                .ok()
                .filter(|version| matches!(version, 1..=3))
                .ok_or(ReadError::CorruptTransitionEvidence)?;
            let evidence = decode_transition_evidence(
                &causes,
                &unit_deltas,
                &artifact_digests,
                &uncertainty,
                &conservation_report,
            )
            .map_err(|_| ReadError::CorruptTransitionEvidence)?;
            if state_hash_version >= 2 && (request_id.is_empty() || evidence.is_none()) {
                return Err(ReadError::CorruptTransitionEvidence);
            }
            events.push(CausalTransition {
                sequence,
                interval_start_second,
                interval_end_second,
                previous_state_hash,
                resulting_state_hash,
                request_id: (!request_id.is_empty()).then_some(request_id),
                evidence,
                state_hash_version,
            });
        }
        let next_cursor_value = events
            .last()
            .map_or(query.after.0, |transition| transition.sequence);
        Ok(EventPage {
            events,
            next_cursor: EventCursor(next_cursor_value),
        })
    }
}

const THERMAL_CONDUCTANCE_UJ_PER_MK_S: i64 = 1_000;

type StateQuantities = BTreeMap<String, (String, i64)>;

struct EvidenceState<'a> {
    simulated_second: i64,
    sleep_phase: circadian::SleepPhase,
    sleep_intention: bool,
    sleep_debt: i64,
    organism: Option<&'a OrganismState>,
    resolution_id: Option<&'a str>,
    bodies: &'a BTreeMap<String, RigidBody>,
}

impl<'a> EvidenceState<'a> {
    fn new(
        simulated_second: i64,
        sleep_phase: circadian::SleepPhase,
        sleep_intention: bool,
        sleep_debt: i64,
        organism: Option<&'a OrganismState>,
        resolution_id: Option<&'a str>,
        bodies: &'a BTreeMap<String, RigidBody>,
    ) -> Self {
        Self {
            simulated_second,
            sleep_phase,
            sleep_intention,
            sleep_debt,
            organism,
            resolution_id,
            bodies,
        }
    }
}

fn transition_evidence(
    request: &CommitRequest,
    previous: EvidenceState<'_>,
    resulting: EvidenceState<'_>,
) -> TransitionEvidence {
    let before = authoritative_quantities(&previous);
    let after = authoritative_quantities(&resulting);
    let mut keys = before.keys().chain(after.keys()).collect::<Vec<_>>();
    keys.sort();
    keys.dedup();
    let unit_deltas = keys
        .into_iter()
        .filter_map(|quantity| {
            let prior = before.get(quantity);
            let next = after.get(quantity);
            let unit = prior.or(next)?.0.clone();
            let before_value = prior.map(|value| value.1);
            let after_value = next.map(|value| value.1);
            (before_value != after_value).then(|| UnitDelta {
                quantity: quantity.clone(),
                unit,
                before: before_value,
                after: after_value,
            })
        })
        .collect();

    let thermal_digest = request
        .thermal
        .as_ref()
        .map(|(_, artifact)| sha256_digest(artifact.program_bytes()));
    TransitionEvidence {
        causes: vec![CausalCause {
            kind: request.cause_kind().to_owned(),
        }],
        unit_deltas,
        // Current built-in mechanisms are not admitted content-addressed
        // artifacts. An empty list records that fact; it is not a substitute
        // digest and keeps audit replay outside this slice's scope.
        artifact_digests: thermal_digest.into_iter().collect(),
        uncertainty: TransitionUncertainty::Unknown {
            reason: "built-in mechanism fidelity envelope is not yet archived as an artifact"
                .to_owned(),
        },
        conservation_report: if let Some((pair, _)) = &request.thermal {
            ConservationReport::Verified {
                quantity: "energy.total".to_owned(),
                unit: "uJ".to_owned(),
                opening: pair.hot().internal_energy_microjoule()
                    + pair.cold().internal_energy_microjoule(),
                external_input: 0,
                closing: pair.hot().internal_energy_microjoule()
                    + pair.cold().internal_energy_microjoule(),
                residual: 0,
            }
        } else {
            ConservationReport::NotEvaluated {
            reason:
                "cross-domain conservation accounting is not yet implemented for this transition"
                    .to_owned(),
        }
        },
    }
}

fn authoritative_quantities(state: &EvidenceState<'_>) -> StateQuantities {
    let mut quantities = BTreeMap::new();
    quantities.insert(
        "clock.simulated_second".to_owned(),
        ("s".to_owned(), state.simulated_second),
    );
    quantities.insert("sleep.debt".to_owned(), ("s".to_owned(), state.sleep_debt));
    quantities.insert(
        "sleep.phase".to_owned(),
        (
            "canonical_sleep_phase".to_owned(),
            match state.sleep_phase {
                circadian::SleepPhase::Awake => 0,
                circadian::SleepPhase::Asleep => 1,
            },
        ),
    );
    quantities.insert(
        "sleep.intention_accepted".to_owned(),
        ("1".to_owned(), i64::from(state.sleep_intention)),
    );
    if let Some(resolution_id) = state.resolution_id {
        quantities.insert(
            format!("resolution/{resolution_id}.active"),
            ("1".to_owned(), 1),
        );
    }
    if let Some(organism) = state.organism {
        quantities.insert(
            "organism.chemical_store".to_owned(),
            ("uJ".to_owned(), organism.chemical_store_uj()),
        );
        quantities.insert(
            "organism.digestion_buffer".to_owned(),
            ("uJ".to_owned(), organism.digestion_buffer_uj()),
        );
        quantities.insert(
            "organism.core_internal_energy".to_owned(),
            ("uJ".to_owned(), organism.core_internal_energy_uj()),
        );
        quantities.insert(
            "organism.ambient_internal_energy".to_owned(),
            ("uJ".to_owned(), organism.ambient_internal_energy_uj()),
        );
        quantities.insert(
            "organism.ambient_heat_capacity".to_owned(),
            (
                "uJ/mK".to_owned(),
                organism
                    .ambient_reservoir()
                    .heat_capacity_microjoule_per_millikelvin(),
            ),
        );
        quantities.insert(
            "organism.blood.volume".to_owned(),
            ("mm3".to_owned(), organism.blood_volume_mm3()),
        );
        quantities.insert(
            "organism.blood.hb_tetramer".to_owned(),
            ("umol".to_owned(), organism.blood().hb_tetramer_umol()),
        );
        quantities.insert(
            "organism.blood.arterial_o2".to_owned(),
            ("umol".to_owned(), organism.arterial_o2_umol()),
        );
        quantities.insert(
            "organism.blood.venous_co2".to_owned(),
            ("umol".to_owned(), organism.venous_co2_umol()),
        );
        quantities.insert(
            "organism.blood.mean_arterial_pressure".to_owned(),
            ("mPa".to_owned(), organism.mean_arterial_pressure_mpa()),
        );
        quantities.insert(
            "organism.blood.lung_diffusion".to_owned(),
            (
                "umol/s".to_owned(),
                organism.blood().lung_diffusion_umol_per_s(),
            ),
        );
    }
    for (body_id, body) in state.bodies {
        let prefix = format!("body/{body_id}");
        quantities.insert(format!("{prefix}.mass"), ("mg".to_owned(), body.mass_mg()));
        for (axis, value) in ["x", "y", "z"].into_iter().zip(body.position_nm()) {
            quantities.insert(
                format!("{prefix}.position.{axis}"),
                ("nm".to_owned(), value),
            );
        }
        for (axis, value) in ["x", "y", "z"].into_iter().zip(body.velocity_nm_per_s()) {
            quantities.insert(
                format!("{prefix}.velocity.{axis}"),
                ("nm/s".to_owned(), value),
            );
        }
        for (axis, value) in ["x", "y", "z"]
            .into_iter()
            .zip(body.center_of_mass_offset_nm())
        {
            quantities.insert(
                format!("{prefix}.center_of_mass_offset.{axis}"),
                ("nm".to_owned(), value),
            );
        }
        for (axis, value) in ["x", "y", "z"]
            .into_iter()
            .zip(body.principal_inertia_mgm2())
        {
            quantities.insert(
                format!("{prefix}.principal_inertia.{axis}"),
                ("mg*m2".to_owned(), value),
            );
        }
        for (axis, value) in ["x", "y", "z"]
            .into_iter()
            .zip(body.angular_velocity_urad_per_s())
        {
            quantities.insert(
                format!("{prefix}.angular_velocity.{axis}"),
                ("urad/s".to_owned(), value),
            );
        }
    }
    quantities
}

fn encode_causes(causes: &[CausalCause]) -> String {
    serde_json::json!({ "kinds": causes.iter().map(|cause| &cause.kind).collect::<Vec<_>>() })
        .to_string()
}

fn encode_unit_deltas(deltas: &[UnitDelta]) -> String {
    serde_json::json!(
        deltas
            .iter()
            .map(|delta| serde_json::json!({
                "quantity": delta.quantity,
                "unit": delta.unit,
                "before": delta.before,
                "after": delta.after,
            }))
            .collect::<Vec<_>>()
    )
    .to_string()
}

fn encode_artifact_digests(digests: &[String]) -> String {
    serde_json::json!(digests).to_string()
}

fn encode_uncertainty(uncertainty: &TransitionUncertainty) -> String {
    match uncertainty {
        TransitionUncertainty::Unknown { reason } => {
            serde_json::json!({ "kind": "unknown", "reason": reason }).to_string()
        }
    }
}

fn encode_conservation_report(report: &ConservationReport) -> String {
    match report {
        ConservationReport::NotEvaluated { reason } => {
            serde_json::json!({ "kind": "not_evaluated", "reason": reason }).to_string()
        }
        ConservationReport::Verified {
            quantity,
            unit,
            opening,
            external_input,
            closing,
            residual,
        } => serde_json::json!({
            "kind": "verified", "quantity": quantity, "unit": unit, "opening": opening,
            "external_input": external_input, "closing": closing, "residual": residual,
        })
        .to_string(),
    }
}

fn decode_transition_evidence(
    causes: &str,
    unit_deltas: &str,
    artifact_digests: &str,
    uncertainty: &str,
    conservation_report: &str,
) -> Result<Option<TransitionEvidence>, ()> {
    if causes.is_empty()
        && unit_deltas.is_empty()
        && artifact_digests.is_empty()
        && uncertainty.is_empty()
        && conservation_report.is_empty()
    {
        return Ok(None);
    }
    let causes_value: serde_json::Value = serde_json::from_str(causes).map_err(|_| ())?;
    let causes = causes_value
        .get("kinds")
        .and_then(serde_json::Value::as_array)
        .ok_or(())?
        .iter()
        .map(|value| {
            value
                .as_str()
                .map(|kind| CausalCause {
                    kind: kind.to_owned(),
                })
                .ok_or(())
        })
        .collect::<Result<Vec<_>, _>>()?;
    let deltas_value: serde_json::Value = serde_json::from_str(unit_deltas).map_err(|_| ())?;
    let unit_deltas = deltas_value
        .as_array()
        .ok_or(())?
        .iter()
        .map(|value| {
            Ok(UnitDelta {
                quantity: value
                    .get("quantity")
                    .and_then(serde_json::Value::as_str)
                    .ok_or(())?
                    .to_owned(),
                unit: value
                    .get("unit")
                    .and_then(serde_json::Value::as_str)
                    .ok_or(())?
                    .to_owned(),
                before: value.get("before").and_then(serde_json::Value::as_i64),
                after: value.get("after").and_then(serde_json::Value::as_i64),
            })
        })
        .collect::<Result<Vec<_>, ()>>()?;
    let artifact_digests = serde_json::from_str::<Vec<String>>(artifact_digests).map_err(|_| ())?;
    let uncertainty_value: serde_json::Value = serde_json::from_str(uncertainty).map_err(|_| ())?;
    let uncertainty = match uncertainty_value
        .get("kind")
        .and_then(serde_json::Value::as_str)
    {
        Some("unknown") => TransitionUncertainty::Unknown {
            reason: uncertainty_value
                .get("reason")
                .and_then(serde_json::Value::as_str)
                .ok_or(())?
                .to_owned(),
        },
        _ => return Err(()),
    };
    let report_value: serde_json::Value =
        serde_json::from_str(conservation_report).map_err(|_| ())?;
    let conservation_report = match report_value.get("kind").and_then(serde_json::Value::as_str) {
        Some("not_evaluated") => ConservationReport::NotEvaluated {
            reason: report_value
                .get("reason")
                .and_then(serde_json::Value::as_str)
                .ok_or(())?
                .to_owned(),
        },
        Some("verified") => ConservationReport::Verified {
            quantity: report_value
                .get("quantity")
                .and_then(serde_json::Value::as_str)
                .ok_or(())?
                .to_owned(),
            unit: report_value
                .get("unit")
                .and_then(serde_json::Value::as_str)
                .ok_or(())?
                .to_owned(),
            opening: report_value
                .get("opening")
                .and_then(serde_json::Value::as_i64)
                .ok_or(())?,
            external_input: report_value
                .get("external_input")
                .and_then(serde_json::Value::as_i64)
                .ok_or(())?,
            closing: report_value
                .get("closing")
                .and_then(serde_json::Value::as_i64)
                .ok_or(())?,
            residual: report_value
                .get("residual")
                .and_then(serde_json::Value::as_i64)
                .ok_or(())?,
        },
        _ => return Err(()),
    };
    Ok(Some(TransitionEvidence {
        causes,
        unit_deltas,
        artifact_digests,
        uncertainty,
        conservation_report,
    }))
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

fn sha256_digest(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let digest: [u8; 32] = Sha256::digest(bytes).into();
    format!(
        "sha256:{}",
        digest
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    )
}

fn thermal_delta_after(encoded: &str) -> Result<(i64, i64), ReadError> {
    let deltas: Vec<serde_json::Value> =
        serde_json::from_str(encoded).map_err(|_| ReadError::CorruptTransitionEvidence)?;
    let find = |quantity| {
        deltas
            .iter()
            .find(|value| {
                value.get("quantity").and_then(serde_json::Value::as_str) == Some(quantity)
            })
            .and_then(|value| value.get("after").and_then(serde_json::Value::as_i64))
            .ok_or(ReadError::CorruptTransitionEvidence)
    };
    Ok((
        find("reservoir.hot.internal_energy")?,
        find("reservoir.cold.internal_energy")?,
    ))
}

fn verified_thermal_conservation(encoded: &str, opening: i64, closing: i64) -> bool {
    let value: serde_json::Value = match serde_json::from_str(encoded) {
        Ok(value) => value,
        Err(_) => return false,
    };
    value.get("kind").and_then(serde_json::Value::as_str) == Some("verified")
        && value.get("quantity").and_then(serde_json::Value::as_str) == Some("energy.total")
        && value.get("opening").and_then(serde_json::Value::as_i64) == Some(opening)
        && value
            .get("external_input")
            .and_then(serde_json::Value::as_i64)
            == Some(0)
        && value.get("closing").and_then(serde_json::Value::as_i64) == Some(closing)
        && value.get("residual").and_then(serde_json::Value::as_i64) == Some(0)
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
    let genesis_hash = initial_genesis_state_hash();
    transaction.execute(
        "INSERT INTO genesis_snapshot (
            singleton, sequence, state_hash, state_hash_version, simulated_second, sleep_phase,
            sleep_intention_accepted, sleep_debt_seconds, resolution_id
         ) VALUES (1, 0, ?1, 2, 0, ?2, 0, 0, NULL)",
        params![
            genesis_hash,
            circadian::SleepPhase::Awake.as_canonical_name()
        ],
    )?;
    transaction.pragma_update(None, "application_id", APPLICATION_ID)?;
    transaction.pragma_update(None, "user_version", SCHEMA_VERSION)?;
    transaction.commit()?;
    Ok(())
}

const RUNTIME_SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS causal_transitions (
    sequence INTEGER PRIMARY KEY,
    interval_start_second INTEGER NOT NULL,
    interval_end_second INTEGER NOT NULL,
    previous_state_hash TEXT NOT NULL DEFAULT '',
    resulting_state_hash TEXT NOT NULL DEFAULT '',
    mechanism_digest TEXT NOT NULL DEFAULT '',
    conservation_report TEXT NOT NULL DEFAULT '',
    request_id TEXT NOT NULL DEFAULT '',
    causes TEXT NOT NULL DEFAULT '',
    unit_deltas TEXT NOT NULL DEFAULT '',
    artifact_digests TEXT NOT NULL DEFAULT '',
    uncertainty TEXT NOT NULL DEFAULT '',
    state_hash_version INTEGER NOT NULL DEFAULT 1
);
CREATE TABLE IF NOT EXISTS genesis_snapshot (
    singleton INTEGER PRIMARY KEY CHECK (singleton = 1),
    sequence INTEGER NOT NULL CHECK (sequence = 0),
    state_hash TEXT NOT NULL,
    state_hash_version INTEGER NOT NULL,
    simulated_second INTEGER NOT NULL,
    sleep_phase TEXT NOT NULL,
    sleep_intention_accepted INTEGER NOT NULL,
    sleep_debt_seconds INTEGER NOT NULL,
    resolution_id TEXT
);
CREATE TABLE IF NOT EXISTS artifact_archive (
    digest TEXT PRIMARY KEY,
    program_bytes BLOB NOT NULL
);
CREATE TABLE IF NOT EXISTS timeline_safe_stop (
    singleton INTEGER PRIMARY KEY CHECK (singleton = 1),
    reason TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS thermal_reservoir_state (
    singleton INTEGER PRIMARY KEY CHECK (singleton = 1),
    hot_energy_uj INTEGER NOT NULL,
    hot_capacity_uj_per_mk INTEGER NOT NULL,
    cold_energy_uj INTEGER NOT NULL,
    cold_capacity_uj_per_mk INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS thermal_execution (
    sequence INTEGER PRIMARY KEY,
    artifact_digest TEXT NOT NULL,
    hot_energy_uj INTEGER NOT NULL,
    hot_capacity_uj_per_mk INTEGER NOT NULL,
    cold_energy_uj INTEGER NOT NULL,
    cold_capacity_uj_per_mk INTEGER NOT NULL,
    conductance_uj_per_mk_s INTEGER NOT NULL
);
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
CREATE TABLE IF NOT EXISTS sleep_intention (
    singleton INTEGER PRIMARY KEY CHECK (singleton = 1),
    accepted INTEGER NOT NULL DEFAULT 0
);
CREATE TABLE IF NOT EXISTS organism_state (
    singleton INTEGER PRIMARY KEY CHECK (singleton = 1),
    chemical_store_uj INTEGER NOT NULL,
    digestion_buffer_uj INTEGER NOT NULL DEFAULT 0,
    core_internal_energy_uj INTEGER NOT NULL,
    ambient_internal_energy_uj INTEGER NOT NULL DEFAULT 2931500000000000,
    ambient_heat_capacity_uj_per_mk INTEGER NOT NULL DEFAULT 10000000000,
    blood_volume_mm3 INTEGER NOT NULL DEFAULT 5000000,
    hb_tetramer_umol INTEGER NOT NULL DEFAULT 11500,
    arterial_o2_umol INTEGER NOT NULL DEFAULT 45080,
    venous_co2_umol INTEGER NOT NULL DEFAULT 24000,
    map_mpa INTEGER NOT NULL DEFAULT 12400000,
    lung_diffusion_umol_per_s INTEGER NOT NULL DEFAULT 300
);
CREATE TABLE IF NOT EXISTS sleep_debt (
    singleton INTEGER PRIMARY KEY CHECK (singleton = 1),
    debt_seconds INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS active_resolution (
    singleton INTEGER PRIMARY KEY CHECK (singleton = 1),
    resolution_id TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS body_state (
    body_id TEXT PRIMARY KEY,
    mass_mg INTEGER NOT NULL,
    position_x_nm INTEGER NOT NULL,
    position_y_nm INTEGER NOT NULL,
    position_z_nm INTEGER NOT NULL,
    velocity_x_nm_per_s INTEGER NOT NULL,
    velocity_y_nm_per_s INTEGER NOT NULL,
    velocity_z_nm_per_s INTEGER NOT NULL,
    com_offset_x_nm INTEGER NOT NULL,
    com_offset_y_nm INTEGER NOT NULL,
    com_offset_z_nm INTEGER NOT NULL,
    inertia_x_mgm2 INTEGER NOT NULL,
    inertia_y_mgm2 INTEGER NOT NULL,
    inertia_z_mgm2 INTEGER NOT NULL,
    angular_x_urad_per_s INTEGER NOT NULL,
    angular_y_urad_per_s INTEGER NOT NULL,
    angular_z_urad_per_s INTEGER NOT NULL
);
";

fn ensure_runtime_tables(connection: &Connection) -> Result<(), rusqlite::Error> {
    connection.execute_batch("BEGIN")?;
    connection.execute_batch(RUNTIME_SCHEMA)?;
    migrate_organism_state_columns(connection)?;
    migrate_causal_transitions_columns(connection)?;
    connection.execute_batch("COMMIT")
}

/// Expand migrations for timelines created before later organism state
/// columns existed. Existing rows keep their values and gain declared
/// defaults.
fn migrate_organism_state_columns(connection: &Connection) -> Result<(), rusqlite::Error> {
    let columns: Vec<String> = {
        let mut statement =
            connection.prepare("SELECT name FROM pragma_table_info('organism_state')")?;
        let rows = statement.query_map([], |row| row.get::<_, String>(0))?;
        rows.collect::<Result<Vec<_>, _>>()?
    };

    fn ensure_column(
        columns: &[String],
        connection: &Connection,
        name: &str,
        default_expr: &str,
    ) -> Result<(), rusqlite::Error> {
        if !columns.iter().any(|column| column == name) {
            connection.execute_batch(&format!(
                "ALTER TABLE organism_state ADD COLUMN {name} INTEGER NOT NULL DEFAULT {default_expr};"
            ))?;
        }
        Ok(())
    }

    ensure_column(
        &columns,
        connection,
        "ambient_heat_capacity_uj_per_mk",
        "10000000000",
    )?;
    ensure_column(&columns, connection, "digestion_buffer_uj", "0")?;
    ensure_column(&columns, connection, "blood_volume_mm3", "5000000")?;
    ensure_column(&columns, connection, "hb_tetramer_umol", "11500")?;
    ensure_column(&columns, connection, "arterial_o2_umol", "45080")?;
    ensure_column(&columns, connection, "venous_co2_umol", "24000")?;
    ensure_column(&columns, connection, "map_mpa", "12400000")?;
    ensure_column(&columns, connection, "lung_diffusion_umol_per_s", "300")?;
    Ok(())
}

fn migrate_causal_transitions_columns(connection: &Connection) -> Result<(), rusqlite::Error> {
    let columns: Vec<String> = {
        let mut statement =
            connection.prepare("SELECT name FROM pragma_table_info('causal_transitions')")?;
        let rows = statement.query_map([], |row| row.get::<_, String>(0))?;
        rows.collect::<Result<Vec<_>, _>>()?
    };
    let ensure_text_column = |name: &str| -> Result<(), rusqlite::Error> {
        if !columns.iter().any(|column| column == name) {
            connection.execute_batch(&format!(
                "ALTER TABLE causal_transitions ADD COLUMN {name} TEXT NOT NULL DEFAULT '';"
            ))?;
        }
        Ok(())
    };
    ensure_text_column("previous_state_hash")?;
    ensure_text_column("resulting_state_hash")?;
    ensure_text_column("mechanism_digest")?;
    ensure_text_column("conservation_report")?;
    ensure_text_column("request_id")?;
    ensure_text_column("causes")?;
    ensure_text_column("unit_deltas")?;
    ensure_text_column("artifact_digests")?;
    ensure_text_column("uncertainty")?;
    if !columns.iter().any(|column| column == "state_hash_version") {
        connection.execute_batch(
            "ALTER TABLE causal_transitions ADD COLUMN state_hash_version INTEGER NOT NULL DEFAULT 1;",
        )?;
    }
    Ok(())
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

fn read_sleep_intention(connection: &Connection) -> bool {
    connection
        .query_row(
            "SELECT accepted FROM sleep_intention WHERE singleton = 1",
            [],
            |row| row.get::<_, i64>(0),
        )
        .map(|value| value != 0)
        .unwrap_or(false)
}

fn initial_organism() -> OrganismState {
    OrganismState::physiological_baseline(&crate::morphotype::Morphotype::human())
}

fn read_organism(connection: &Connection) -> Option<OrganismState> {
    // Try new blood columns first; fall back to old rows via COALESCE defaults.
    let blood_row = connection.query_row(
        "SELECT chemical_store_uj,
                    COALESCE(digestion_buffer_uj, 0),
                    core_internal_energy_uj,
                    COALESCE(ambient_internal_energy_uj, 2931500000000000),
                    COALESCE(ambient_heat_capacity_uj_per_mk, 10000000000),
                    COALESCE(blood_volume_mm3, 5000000),
                    COALESCE(hb_tetramer_umol, 11500),
                    COALESCE(arterial_o2_umol, 45080),
                    COALESCE(venous_co2_umol, 24000),
                    COALESCE(map_mpa, 12400000),
                    COALESCE(lung_diffusion_umol_per_s, 300)
             FROM organism_state WHERE singleton = 1",
        [],
        |row| {
            Ok(OrganismState::with_blood_from_row(
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, i64>(4)?,
                row.get(5)?,
                row.get(6)?,
                row.get(7)?,
                row.get(8)?,
                row.get(9)?,
                row.get(10)?,
            ))
        },
    );
    if let Ok(org) = blood_row {
        return Some(org);
    }
    // Legacy fallback without blood columns (pre-migration DB).
    connection
        .query_row(
            "SELECT chemical_store_uj,
                    COALESCE(digestion_buffer_uj, 0),
                    core_internal_energy_uj,
                    COALESCE(ambient_internal_energy_uj, 2931500000000000),
                    COALESCE(ambient_heat_capacity_uj_per_mk, 10000000000)
             FROM organism_state WHERE singleton = 1",
            [],
            |row| {
                Ok(OrganismState::with_ambient_from_row(
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, i64>(4)?,
                ))
            },
        )
        .ok()
}

fn read_sleep_debt(connection: &Connection) -> i64 {
    connection
        .query_row(
            "SELECT debt_seconds FROM sleep_debt WHERE singleton = 1",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0)
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

fn read_resolution_id(connection: &Connection) -> Option<String> {
    connection
        .query_row(
            "SELECT resolution_id FROM active_resolution WHERE singleton = 1",
            [],
            |row| row.get(0),
        )
        .optional()
        .ok()
        .flatten()
}

fn read_thermal_reservoirs(connection: &Connection) -> Option<ReservoirPair> {
    connection.query_row(
        "SELECT hot_energy_uj, hot_capacity_uj_per_mk, cold_energy_uj, cold_capacity_uj_per_mk FROM thermal_reservoir_state WHERE singleton = 1",
        [],
        |row| Ok(ReservoirPair::new(
            crate::quantity::ReservoirState::new(row.get(0)?, row.get(1)?),
            crate::quantity::ReservoirState::new(row.get(2)?, row.get(3)?),
        )),
    ).optional().ok().flatten()
}

fn compute_state_hash_v1(
    simulated_second: i64,
    sleep_phase: crate::circadian::SleepPhase,
    sleep_debt_seconds: i64,
    organism: Option<&OrganismState>,
    resolution_id: Option<&str>,
) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(b"makise-state-v1");
    hasher.update(simulated_second.to_be_bytes());
    hasher.update(sleep_phase.as_canonical_name().as_bytes());
    hasher.update(sleep_debt_seconds.to_be_bytes());
    if let Some(res) = resolution_id {
        hasher.update(res.as_bytes());
    }
    if let Some(org) = organism {
        hasher.update(org.chemical_store_uj().to_be_bytes());
        hasher.update(org.digestion_buffer_uj().to_be_bytes());
        hasher.update(org.core_internal_energy_uj().to_be_bytes());
        hasher.update(org.ambient_internal_energy_uj().to_be_bytes());
        hasher.update(
            org.ambient_reservoir()
                .heat_capacity_microjoule_per_millikelvin()
                .to_be_bytes(),
        );
        hasher.update(org.blood_volume_mm3().to_be_bytes());
        hasher.update(org.blood().hb_tetramer_umol().to_be_bytes());
        hasher.update(org.arterial_o2_umol().to_be_bytes());
        hasher.update(org.venous_co2_umol().to_be_bytes());
        hasher.update(org.mean_arterial_pressure_mpa().to_be_bytes());
        hasher.update(org.blood().lung_diffusion_umol_per_s().to_be_bytes());
    }
    let bytes: [u8; 32] = hasher.finalize().into();
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

impl WorldEngine {
    fn previous_state_hash_version(&self) -> Result<u8, CommitError> {
        let version: Option<i64> = self
            ._connection
            .query_row(
                "SELECT state_hash_version FROM causal_transitions ORDER BY sequence DESC LIMIT 1",
                [],
                |row| row.get(0),
            )
            .optional()?;
        Ok(version.unwrap_or(2).clamp(1, 3) as u8)
    }

    fn compute_state_hash_for_previous(
        &self,
        version: u8,
        bodies: &BTreeMap<String, RigidBody>,
    ) -> String {
        if version == 1 {
            return compute_state_hash_v1(
                self.simulated_second,
                self.sleep_phase,
                self.sleep_debt_seconds,
                self.organism.as_ref(),
                self.resolution_id.as_deref(),
            );
        }
        if version == 3 {
            return compute_state_hash_v3(
                self.simulated_second,
                self.sleep_phase,
                self.sleep_intention_accepted,
                self.sleep_debt_seconds,
                self.organism.as_ref(),
                self.resolution_id.as_deref(),
                bodies,
                self.reservoirs.as_ref(),
            );
        }
        compute_state_hash_v2(
            self.simulated_second,
            self.sleep_phase,
            self.sleep_intention_accepted,
            self.sleep_debt_seconds,
            self.organism.as_ref(),
            self.resolution_id.as_deref(),
            bodies,
        )
    }
}

#[allow(clippy::too_many_arguments)] // State-hash versions preserve stable historical call shape.
fn compute_state_hash_v3(
    simulated_second: i64,
    sleep_phase: crate::circadian::SleepPhase,
    sleep_intention_accepted: bool,
    sleep_debt_seconds: i64,
    organism: Option<&OrganismState>,
    resolution_id: Option<&str>,
    bodies: &BTreeMap<String, RigidBody>,
    reservoirs: Option<&ReservoirPair>,
) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(b"makise-state-v3");
    hasher.update(
        compute_state_hash_v2(
            simulated_second,
            sleep_phase,
            sleep_intention_accepted,
            sleep_debt_seconds,
            organism,
            resolution_id,
            bodies,
        )
        .as_bytes(),
    );
    match reservoirs {
        Some(pair) => {
            for value in [
                pair.hot().internal_energy_microjoule(),
                pair.hot().heat_capacity_microjoule_per_millikelvin(),
                pair.cold().internal_energy_microjoule(),
                pair.cold().heat_capacity_microjoule_per_millikelvin(),
            ] {
                hasher.update(value.to_be_bytes());
            }
        }
        None => hasher.update([0]),
    }
    let bytes: [u8; 32] = hasher.finalize().into();
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn compute_state_hash_v2(
    simulated_second: i64,
    sleep_phase: crate::circadian::SleepPhase,
    sleep_intention_accepted: bool,
    sleep_debt_seconds: i64,
    organism: Option<&OrganismState>,
    resolution_id: Option<&str>,
    bodies: &BTreeMap<String, RigidBody>,
) -> String {
    use sha2::{Digest, Sha256};
    fn write_string(hasher: &mut Sha256, value: &str) {
        hasher.update((value.len() as u64).to_be_bytes());
        hasher.update(value.as_bytes());
    }
    fn write_body(hasher: &mut Sha256, body: &RigidBody) {
        for value in [
            body.mass_mg(),
            body.position_nm()[0],
            body.position_nm()[1],
            body.position_nm()[2],
            body.velocity_nm_per_s()[0],
            body.velocity_nm_per_s()[1],
            body.velocity_nm_per_s()[2],
            body.center_of_mass_offset_nm()[0],
            body.center_of_mass_offset_nm()[1],
            body.center_of_mass_offset_nm()[2],
            body.principal_inertia_mgm2()[0],
            body.principal_inertia_mgm2()[1],
            body.principal_inertia_mgm2()[2],
            body.angular_velocity_urad_per_s()[0],
            body.angular_velocity_urad_per_s()[1],
            body.angular_velocity_urad_per_s()[2],
        ] {
            hasher.update(value.to_be_bytes());
        }
    }
    let mut hasher = Sha256::new();
    hasher.update(b"makise-state-v2");
    hasher.update(simulated_second.to_be_bytes());
    write_string(&mut hasher, sleep_phase.as_canonical_name());
    hasher.update([u8::from(sleep_intention_accepted)]);
    hasher.update(sleep_debt_seconds.to_be_bytes());
    match resolution_id {
        Some(value) => {
            hasher.update([1]);
            write_string(&mut hasher, value);
        }
        None => hasher.update([0]),
    }
    match organism {
        Some(org) => {
            hasher.update([1]);
            for value in [
                org.chemical_store_uj(),
                org.digestion_buffer_uj(),
                org.core_internal_energy_uj(),
                org.ambient_internal_energy_uj(),
                org.ambient_reservoir()
                    .heat_capacity_microjoule_per_millikelvin(),
                org.blood_volume_mm3(),
                org.blood().hb_tetramer_umol(),
                org.arterial_o2_umol(),
                org.venous_co2_umol(),
                org.mean_arterial_pressure_mpa(),
                org.blood().lung_diffusion_umol_per_s(),
            ] {
                hasher.update(value.to_be_bytes());
            }
            let morphotype = org.morphotype();
            for value in [
                morphotype.awake_metabolism_uj_per_second(),
                morphotype.asleep_metabolism_uj_per_second(),
                morphotype.night_awake_metabolism_uj_per_second(),
                morphotype.core_heat_capacity_uj_per_mk(),
                morphotype.ambient_conductance_uj_per_mk_s(),
                morphotype.blood_volume_mm3(),
                morphotype.hb_tetramer_umol(),
                morphotype.mean_arterial_pressure_mpa(),
                morphotype.lung_diffusion_umol_per_s(),
            ] {
                hasher.update(value.to_be_bytes());
            }
            hasher.update((morphotype.anatomy_nodes().len() as u64).to_be_bytes());
            for node in morphotype.anatomy_nodes() {
                write_string(&mut hasher, &node.node_id);
                write_string(&mut hasher, &node.kind);
                hasher.update(node.count.to_be_bytes());
            }
            hasher.update((morphotype.organ_bindings().len() as u64).to_be_bytes());
            for binding in morphotype.organ_bindings() {
                write_string(&mut hasher, &binding.anatomy_node_id);
                write_string(&mut hasher, &binding.mechanism_id);
                write_string(&mut hasher, &binding.mechanism_digest);
                write_string(&mut hasher, &binding.resolution_id);
            }
        }
        None => hasher.update([0]),
    }
    hasher.update((bodies.len() as u64).to_be_bytes());
    for (body_id, body) in bodies {
        write_string(&mut hasher, body_id);
        write_body(&mut hasher, body);
    }
    let bytes: [u8; 32] = hasher.finalize().into();
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn replace_body_state(bodies: &mut BTreeMap<String, RigidBody>, body_id: &str, body: &RigidBody) {
    bodies.insert(body_id.to_owned(), *body);
}

fn read_body_states_for_hash(
    connection: &Connection,
) -> Result<BTreeMap<String, RigidBody>, rusqlite::Error> {
    let mut statement = connection.prepare(
        "SELECT body_id, mass_mg, position_x_nm, position_y_nm, position_z_nm,
                velocity_x_nm_per_s, velocity_y_nm_per_s, velocity_z_nm_per_s,
                com_offset_x_nm, com_offset_y_nm, com_offset_z_nm,
                inertia_x_mgm2, inertia_y_mgm2, inertia_z_mgm2,
                angular_x_urad_per_s, angular_y_urad_per_s, angular_z_urad_per_s
         FROM body_state ORDER BY body_id ASC",
    )?;
    let rows = statement.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            RigidBody::new(
                row.get(1)?,
                [row.get(2)?, row.get(3)?, row.get(4)?],
                [row.get(5)?, row.get(6)?, row.get(7)?],
                [row.get(8)?, row.get(9)?, row.get(10)?],
                [row.get(11)?, row.get(12)?, row.get(13)?],
                [row.get(14)?, row.get(15)?, row.get(16)?],
            )
            .map_err(|_| rusqlite::Error::InvalidQuery)?,
        ))
    })?;
    rows.collect()
}

fn verify_transition_chain(connection: &Connection) -> Result<(), rusqlite::Error> {
    let genesis_hash: Option<String> = connection
        .query_row(
            "SELECT state_hash, state_hash_version, simulated_second, sleep_phase,
                    sleep_intention_accepted, sleep_debt_seconds, resolution_id
             FROM genesis_snapshot WHERE singleton = 1",
            [],
            |row| {
                let version: i64 = row.get(1)?;
                let second: i64 = row.get(2)?;
                let phase: String = row.get(3)?;
                let accepted: i64 = row.get(4)?;
                let debt: i64 = row.get(5)?;
                let resolution_id: Option<String> = row.get(6)?;
                if version != 2
                    || second != 0
                    || phase != circadian::SleepPhase::Awake.as_canonical_name()
                    || accepted != 0
                    || debt != 0
                    || resolution_id.is_some()
                    || row.get::<_, String>(0)? != initial_genesis_state_hash()
                {
                    return Err(rusqlite::Error::QueryReturnedNoRows);
                }
                row.get(0)
            },
        )
        .optional()?;
    let mut statement = connection.prepare(
        "SELECT sequence, interval_start_second, interval_end_second,
                COALESCE(previous_state_hash,''), COALESCE(resulting_state_hash,''),
                COALESCE(request_id,''), COALESCE(causes,''), COALESCE(unit_deltas,''),
                COALESCE(artifact_digests,''), COALESCE(uncertainty,''),
                COALESCE(conservation_report,''), COALESCE(state_hash_version, 1)
         FROM causal_transitions ORDER BY sequence ASC",
    )?;
    let rows = statement.query_map([], |row| {
        let request_id: String = row.get(5)?;
        let evidence = decode_transition_evidence(
            row.get::<_, String>(6)?.as_str(),
            row.get::<_, String>(7)?.as_str(),
            row.get::<_, String>(8)?.as_str(),
            row.get::<_, String>(9)?.as_str(),
            row.get::<_, String>(10)?.as_str(),
        )
        .map_err(|_| rusqlite::Error::QueryReturnedNoRows)?;
        Ok(CausalTransition {
            sequence: row.get::<_, i64>(0)?.max(0) as u64,
            interval_start_second: row.get(1)?,
            interval_end_second: row.get(2)?,
            previous_state_hash: row.get(3)?,
            resulting_state_hash: row.get(4)?,
            request_id: (!request_id.is_empty()).then_some(request_id),
            evidence,
            state_hash_version: row.get::<_, i64>(11)?.clamp(1, 3) as u8,
        })
    })?;

    let mut previous_end_second: Option<i64> = None;
    let mut previous_resulting_hash = genesis_hash;
    let mut last_end_second = 0;
    for (expected_index, row) in (1u64..).zip(rows) {
        let transition = row?;
        if transition.sequence != expected_index {
            return Err(rusqlite::Error::QueryReturnedNoRows);
        }
        let expected_start = previous_end_second.unwrap_or(transition.interval_start_second);
        if transition.interval_start_second != expected_start {
            return Err(rusqlite::Error::QueryReturnedNoRows);
        }
        if transition.interval_end_second < transition.interval_start_second {
            return Err(rusqlite::Error::QueryReturnedNoRows);
        }
        if transition.state_hash_version >= 2
            && (transition.request_id.is_none() || transition.evidence.is_none())
        {
            return Err(rusqlite::Error::QueryReturnedNoRows);
        }
        // Expand-migration: legacy rows have empty hashes — skip check.
        // New rows must form hash chain.
        if !transition.previous_state_hash.is_empty()
            && !transition.resulting_state_hash.is_empty()
            && let Some(prev_hash) = &previous_resulting_hash
            && &transition.previous_state_hash != prev_hash
        {
            return Err(rusqlite::Error::QueryReturnedNoRows);
        }
        if !transition.previous_state_hash.is_empty() && !transition.resulting_state_hash.is_empty()
        {
            previous_resulting_hash = Some(transition.resulting_state_hash.clone());
        }
        last_end_second = transition.interval_end_second;
        previous_end_second = Some(transition.interval_end_second);
    }

    let simulated_clock: Option<i64> = connection
        .query_row(
            "SELECT second FROM simulated_clock WHERE singleton = 1",
            [],
            |row| row.get(0),
        )
        .optional()?;
    if simulated_clock.unwrap_or(0) != last_end_second {
        return Err(rusqlite::Error::QueryReturnedNoRows);
    }
    Ok(())
}
