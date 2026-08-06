use std::path::{Path, PathBuf};

use rusqlite::{Connection, OptionalExtension, Transaction, TransactionBehavior, params};
use sha2::{Digest, Sha256};

use crate::domain::{
    CommandResult, CommandStatus, DomainEvent, EVENT_SCHEMA_VERSION, PersistedEvent, WorldState,
};
use crate::{PathGuard, Result, WorldError};

const SNAPSHOT_INTERVAL: u64 = 32;

pub(crate) struct EventStore {
    connection: Connection,
    database_path: PathBuf,
}

impl EventStore {
    pub(crate) fn open(
        database_path: impl AsRef<Path>,
        identity_id: &str,
        definition_hash: &str,
        guard: &PathGuard,
    ) -> Result<Self> {
        let safe_path = guard.validate(database_path)?;
        let parent = safe_path.parent().ok_or_else(|| WorldError::UnsafePath {
            path: safe_path.clone(),
            reason: "database path has no parent".into(),
        })?;
        guard.validate(parent)?;
        std::fs::create_dir_all(parent)?;

        let connection = Connection::open(&safe_path)?;
        connection.pragma_update(None, "journal_mode", "WAL")?;
        connection.pragma_update(None, "synchronous", "FULL")?;
        connection.pragma_update(None, "foreign_keys", "ON")?;
        connection.busy_timeout(std::time::Duration::from_secs(3))?;

        let mut store = Self {
            connection,
            database_path: safe_path,
        };
        store.initialize_schema()?;
        store.verify_or_initialize_meta(identity_id, definition_hash)?;
        Ok(store)
    }

    pub(crate) fn database_path(&self) -> &Path {
        &self.database_path
    }

    fn initialize_schema(&mut self) -> Result<()> {
        self.connection.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS meta (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS events (
                event_seq INTEGER PRIMARY KEY,
                event_id TEXT NOT NULL UNIQUE,
                world_version INTEGER NOT NULL UNIQUE,
                occurred_at_ms INTEGER NOT NULL,
                causation_command_id TEXT,
                event_type TEXT NOT NULL,
                envelope_json TEXT NOT NULL,
                envelope_sha256 TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS commands (
                command_id TEXT PRIMARY KEY,
                request_sha256 TEXT NOT NULL,
                result_json TEXT NOT NULL,
                recorded_at_ms INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS snapshots (
                event_seq INTEGER PRIMARY KEY,
                world_version INTEGER NOT NULL,
                state_json TEXT NOT NULL,
                state_sha256 TEXT NOT NULL,
                created_at_ms INTEGER NOT NULL
            );
            ",
        )?;
        Ok(())
    }

    fn verify_or_initialize_meta(
        &mut self,
        identity_id: &str,
        definition_hash: &str,
    ) -> Result<()> {
        let existing_identity = self.meta("identity_id")?;
        match existing_identity {
            Some(actual) if actual != identity_id => {
                return Err(WorldError::IdentityMismatch {
                    expected: identity_id.into(),
                    actual,
                });
            }
            None => self.set_meta("identity_id", identity_id)?,
            _ => {}
        }

        let existing_definition = self.meta("world_definition_hash")?;
        match existing_definition {
            Some(actual) if actual != definition_hash => {
                return Err(WorldError::DefinitionMismatch {
                    expected: definition_hash.into(),
                    actual,
                });
            }
            None => self.set_meta("world_definition_hash", definition_hash)?,
            _ => {}
        }
        if self.meta("world_version")?.is_none() {
            self.set_meta("world_version", "0")?;
            self.set_meta("last_event_seq", "0")?;
        }
        if self.meta("clock_checkpoint_utc_ms")?.is_none() {
            self.set_meta("clock_checkpoint_utc_ms", "0")?;
        }
        Ok(())
    }

    pub(crate) fn load_state(
        &self,
        identity_id: &str,
        definition_hash: &str,
    ) -> Result<WorldState> {
        let snapshot = self
            .connection
            .query_row(
                "SELECT event_seq, state_json, state_sha256 FROM snapshots ORDER BY event_seq DESC LIMIT 1",
                [],
                |row| {
                    Ok((
                        row.get::<_, u64>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                },
            )
            .optional()?;

        let mut state = if let Some((event_seq, state_json, expected_hash)) = snapshot {
            if digest(state_json.as_bytes()) != expected_hash {
                return Err(WorldError::SnapshotChecksumMismatch(event_seq));
            }
            let state: WorldState = serde_json::from_str(&state_json)?;
            if state.identity_id() != identity_id {
                return Err(WorldError::IdentityMismatch {
                    expected: identity_id.into(),
                    actual: state.identity_id().into(),
                });
            }
            if state.world_definition_hash() != definition_hash {
                return Err(WorldError::DefinitionMismatch {
                    expected: definition_hash.into(),
                    actual: state.world_definition_hash().into(),
                });
            }
            state
        } else {
            WorldState::empty(identity_id.into(), definition_hash.into())
        };

        let mut statement = self.connection.prepare(
            "SELECT event_seq, event_type, envelope_json, envelope_sha256
             FROM events WHERE event_seq > ?1 ORDER BY event_seq",
        )?;
        let rows = statement.query_map([state.last_event_seq()], |row| {
            Ok((
                row.get::<_, u64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ))
        })?;

        for row in rows {
            let (seq, stored_type, envelope_json, expected_hash) = row?;
            if digest(envelope_json.as_bytes()) != expected_hash {
                return Err(WorldError::EventChecksumMismatch(seq));
            }
            let envelope: PersistedEvent =
                serde_json::from_str(&envelope_json).map_err(|error| {
                    WorldError::InvalidPersistedEvent(format!("event {seq}: {error}"))
                })?;
            if envelope.payload.event_type() != stored_type {
                return Err(WorldError::InvalidPersistedEvent(format!(
                    "event {seq} type column does not match its payload"
                )));
            }
            state.apply(&envelope)?;
        }

        let expected_version = parse_meta_u64(self.meta("world_version")?, "world_version")?;
        let expected_seq = parse_meta_u64(self.meta("last_event_seq")?, "last_event_seq")?;
        if state.world_version() != expected_version {
            return Err(WorldError::WorldVersionGap {
                expected: expected_version,
                actual: state.world_version(),
            });
        }
        if state.last_event_seq() != expected_seq {
            return Err(WorldError::EventSequenceGap {
                expected: expected_seq,
                actual: state.last_event_seq(),
            });
        }
        if state.last_event_seq() > 0 {
            let expected_hash = self.meta("state_sha256")?.ok_or_else(|| {
                WorldError::InvalidPersistedEvent("missing meta key state_sha256".into())
            })?;
            if state.state_hash()? != expected_hash {
                return Err(WorldError::StateInvariant(
                    "persisted state hash mismatch".into(),
                ));
            }
        }
        Ok(state)
    }

    pub(crate) fn find_command(&self, command_id: &str) -> Result<Option<(String, CommandResult)>> {
        let row = self
            .connection
            .query_row(
                "SELECT request_sha256, result_json FROM commands WHERE command_id = ?1",
                [command_id],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )
            .optional()?;
        row.map(|(request_hash, result_json)| {
            Ok((request_hash, serde_json::from_str(&result_json)?))
        })
        .transpose()
    }

    pub(crate) fn events_after(&self, after_seq: u64) -> Result<Vec<PersistedEvent>> {
        let mut statement = self.connection.prepare(
            "SELECT event_seq, event_type, envelope_json, envelope_sha256
             FROM events WHERE event_seq > ?1 ORDER BY event_seq",
        )?;
        let rows = statement.query_map([after_seq], |row| {
            Ok((
                row.get::<_, u64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ))
        })?;
        let mut events = Vec::new();
        for row in rows {
            let (seq, stored_type, envelope_json, expected_hash) = row?;
            if digest(envelope_json.as_bytes()) != expected_hash {
                return Err(WorldError::EventChecksumMismatch(seq));
            }
            let event: PersistedEvent = serde_json::from_str(&envelope_json).map_err(|error| {
                WorldError::InvalidPersistedEvent(format!("event {seq}: {error}"))
            })?;
            if event.event_type() != stored_type {
                return Err(WorldError::InvalidPersistedEvent(format!(
                    "event {seq} type column does not match its payload"
                )));
            }
            events.push(event);
        }
        Ok(events)
    }

    pub(crate) fn commit_command(
        &mut self,
        state: &WorldState,
        command_id: &str,
        request_hash: &str,
        occurred_at_ms: i64,
        events: Vec<DomainEvent>,
    ) -> Result<(WorldState, CommandResult)> {
        if events.is_empty() {
            return Err(WorldError::StateInvariant(
                "a committed command must produce at least one event".into(),
            ));
        }
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        verify_database_head(&transaction, state)?;
        let mut next = state.clone();
        let first_event_seq = state.last_event_seq() + 1;
        append_events(
            &transaction,
            &mut next,
            occurred_at_ms,
            Some(command_id),
            events,
        )?;
        let result = CommandResult {
            command_id: command_id.into(),
            status: CommandStatus::Committed,
            committed_world_version: next.world_version(),
            first_event_seq,
            last_event_seq: next.last_event_seq(),
            error_code: None,
            error_message: None,
            suggested_recovery: Vec::new(),
        };
        transaction.execute(
            "INSERT INTO commands(command_id, request_sha256, result_json, recorded_at_ms)
             VALUES (?1, ?2, ?3, ?4)",
            params![
                command_id,
                request_hash,
                serde_json::to_string(&result)?,
                occurred_at_ms
            ],
        )?;
        finish_commit(&transaction, &next, occurred_at_ms)?;
        transaction.commit()?;
        Ok((next, result))
    }

    pub(crate) fn record_rejection(
        &mut self,
        request_hash: &str,
        result: &CommandResult,
        recorded_at_ms: i64,
    ) -> Result<()> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        transaction.execute(
            "INSERT INTO commands(command_id, request_sha256, result_json, recorded_at_ms)
             VALUES (?1, ?2, ?3, ?4)",
            params![
                result.command_id,
                request_hash,
                serde_json::to_string(result)?,
                recorded_at_ms
            ],
        )?;
        set_transaction_meta(
            &transaction,
            "clock_checkpoint_utc_ms",
            &recorded_at_ms.to_string(),
        )?;
        transaction.commit()?;
        Ok(())
    }

    pub(crate) fn commit_system_events(
        &mut self,
        state: &WorldState,
        occurred_at_ms: i64,
        events: Vec<DomainEvent>,
    ) -> Result<WorldState> {
        if events.is_empty() {
            return Ok(state.clone());
        }
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        verify_database_head(&transaction, state)?;
        let mut next = state.clone();
        append_events(&transaction, &mut next, occurred_at_ms, None, events)?;
        finish_commit(&transaction, &next, occurred_at_ms)?;
        transaction.commit()?;
        Ok(next)
    }

    pub(crate) fn clock_checkpoint_utc_ms(&self) -> Result<i64> {
        parse_meta_i64(
            self.meta("clock_checkpoint_utc_ms")?,
            "clock_checkpoint_utc_ms",
        )
    }

    pub(crate) fn record_clock_checkpoint(&self, now_ms: i64) -> Result<()> {
        self.set_meta("clock_checkpoint_utc_ms", &now_ms.to_string())
    }

    pub(crate) fn force_snapshot(&mut self, state: &WorldState, now_ms: i64) -> Result<()> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        write_snapshot(&transaction, state, now_ms)?;
        set_transaction_meta(&transaction, "clock_checkpoint_utc_ms", &now_ms.to_string())?;
        transaction.commit()?;
        Ok(())
    }

    fn meta(&self, key: &str) -> Result<Option<String>> {
        Ok(self
            .connection
            .query_row("SELECT value FROM meta WHERE key = ?1", [key], |row| {
                row.get(0)
            })
            .optional()?)
    }

    fn set_meta(&self, key: &str, value: &str) -> Result<()> {
        self.connection.execute(
            "INSERT INTO meta(key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            [key, value],
        )?;
        Ok(())
    }
}

fn append_events(
    transaction: &Transaction<'_>,
    state: &mut WorldState,
    occurred_at_ms: i64,
    causation_command_id: Option<&str>,
    events: Vec<DomainEvent>,
) -> Result<()> {
    for payload in events {
        let event_seq = state.last_event_seq() + 1;
        let world_version = state.world_version() + 1;
        let envelope = PersistedEvent {
            event_id: format!("{}:{event_seq}", state.identity_id()),
            event_seq,
            world_version,
            event_schema_version: EVENT_SCHEMA_VERSION,
            occurred_at_ms,
            causation_command_id: causation_command_id.map(str::to_owned),
            payload,
        };
        let event_type = envelope.payload.event_type();
        let envelope_json = serde_json::to_string(&envelope)?;
        let envelope_hash = digest(envelope_json.as_bytes());
        transaction.execute(
            "INSERT INTO events(
                 event_seq, event_id, world_version, occurred_at_ms,
                 causation_command_id, event_type, envelope_json, envelope_sha256
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                event_seq,
                envelope.event_id,
                world_version,
                occurred_at_ms,
                causation_command_id,
                event_type,
                envelope_json,
                envelope_hash
            ],
        )?;
        state.apply(&envelope)?;
    }
    Ok(())
}

fn finish_commit(transaction: &Transaction<'_>, state: &WorldState, now_ms: i64) -> Result<()> {
    set_transaction_meta(
        transaction,
        "world_version",
        &state.world_version().to_string(),
    )?;
    set_transaction_meta(
        transaction,
        "last_event_seq",
        &state.last_event_seq().to_string(),
    )?;
    set_transaction_meta(transaction, "state_sha256", &state.state_hash()?)?;
    set_transaction_meta(transaction, "clock_checkpoint_utc_ms", &now_ms.to_string())?;
    if state.last_event_seq().is_multiple_of(SNAPSHOT_INTERVAL) {
        write_snapshot(transaction, state, now_ms)?;
    }
    Ok(())
}

fn write_snapshot(transaction: &Transaction<'_>, state: &WorldState, now_ms: i64) -> Result<()> {
    let state_json = serde_json::to_string(state)?;
    transaction.execute(
        "INSERT INTO snapshots(event_seq, world_version, state_json, state_sha256, created_at_ms)
         VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT(event_seq) DO NOTHING",
        params![
            state.last_event_seq(),
            state.world_version(),
            state_json,
            digest(state_json.as_bytes()),
            now_ms
        ],
    )?;
    Ok(())
}

fn verify_database_head(transaction: &Transaction<'_>, state: &WorldState) -> Result<()> {
    let version = transaction.query_row(
        "SELECT value FROM meta WHERE key = 'world_version'",
        [],
        |row| row.get::<_, String>(0),
    )?;
    let seq = transaction.query_row(
        "SELECT value FROM meta WHERE key = 'last_event_seq'",
        [],
        |row| row.get::<_, String>(0),
    )?;
    let version = version.parse::<u64>().map_err(|_| {
        WorldError::InvalidPersistedEvent("meta world_version is not an integer".into())
    })?;
    let seq = seq.parse::<u64>().map_err(|_| {
        WorldError::InvalidPersistedEvent("meta last_event_seq is not an integer".into())
    })?;
    if version != state.world_version() || seq != state.last_event_seq() {
        return Err(WorldError::StateInvariant(
            "in-memory state is not the current database head".into(),
        ));
    }
    Ok(())
}

fn set_transaction_meta(transaction: &Transaction<'_>, key: &str, value: &str) -> Result<()> {
    transaction.execute(
        "INSERT INTO meta(key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        [key, value],
    )?;
    Ok(())
}

fn parse_meta_u64(value: Option<String>, key: &str) -> Result<u64> {
    value
        .ok_or_else(|| WorldError::InvalidPersistedEvent(format!("missing meta key {key}")))?
        .parse()
        .map_err(|_| WorldError::InvalidPersistedEvent(format!("invalid meta key {key}")))
}

fn parse_meta_i64(value: Option<String>, key: &str) -> Result<i64> {
    value
        .ok_or_else(|| WorldError::InvalidPersistedEvent(format!("missing meta key {key}")))?
        .parse()
        .map_err(|_| WorldError::InvalidPersistedEvent(format!("invalid meta key {key}")))
}

pub(crate) fn digest(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}
