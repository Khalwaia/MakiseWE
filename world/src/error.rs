use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum WorldError {
    #[error("unsafe path rejected: {path} ({reason})")]
    UnsafePath { path: PathBuf, reason: String },
    #[error("world definition is invalid: {0}")]
    InvalidDefinition(String),
    #[error("persisted identity mismatch: expected {expected}, found {actual}")]
    IdentityMismatch { expected: String, actual: String },
    #[error("world definition hash mismatch: expected {expected}, found {actual}")]
    DefinitionMismatch { expected: String, actual: String },
    #[error("event sequence gap: expected {expected}, found {actual}")]
    EventSequenceGap { expected: u64, actual: u64 },
    #[error("world version gap: expected {expected}, found {actual}")]
    WorldVersionGap { expected: u64, actual: u64 },
    #[error("event checksum mismatch at sequence {0}")]
    EventChecksumMismatch(u64),
    #[error("snapshot checksum mismatch at sequence {0}")]
    SnapshotChecksumMismatch(u64),
    #[error("unknown or malformed persisted event: {0}")]
    InvalidPersistedEvent(String),
    #[error("state invariant failed: {0}")]
    StateInvariant(String),
    #[error("invalid weather observation: {0}")]
    InvalidWeatherObservation(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Sqlite(#[from] rusqlite::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, WorldError>;
