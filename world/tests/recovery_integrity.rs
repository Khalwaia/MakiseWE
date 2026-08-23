use std::path::Path;

use makise_world::{CommandEnvelope, CommandPayload, PathGuard, WorldDefinition, WorldEngine};
use rusqlite::Connection;
use sha2::{Digest, Sha256};

fn definition() -> WorldDefinition {
    WorldDefinition::load(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../world-packages/test-room-v1/manifest.json"),
        &PathGuard::default(),
    )
    .unwrap()
}

fn file_digest(path: &Path) -> String {
    Sha256::digest(std::fs::read(path).unwrap())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[test]
fn public_legacy_reader_preserves_archive_bytes() {
    let temp = tempfile::tempdir().unwrap();
    let database = temp.path().join("world.db");

    {
        let engine = WorldEngine::open(
            &database,
            "legacy-baseline",
            definition(),
            "bed",
            1_000_000,
            &PathGuard::default(),
        )
        .unwrap();
        assert!(!engine.events_after(0).unwrap().is_empty());
    }

    let digest_before_read = file_digest(&database);
    {
        let engine = WorldEngine::open(
            &database,
            "legacy-baseline",
            definition(),
            "bed",
            1_000_000,
            &PathGuard::default(),
        )
        .unwrap();
        assert!(!engine.events_after(0).unwrap().is_empty());
    }

    assert_eq!(file_digest(&database), digest_before_read);
}

#[test]
fn unknown_persisted_event_blocks_replay() {
    let temp = tempfile::tempdir().unwrap();
    let database = temp.path().join("world.db");
    {
        let mut engine = WorldEngine::open(
            &database,
            "test-makise",
            definition(),
            "bed",
            1_000_000,
            &PathGuard::default(),
        )
        .unwrap();
        let command = CommandEnvelope {
            command_id: "cmd-future-event".into(),
            identity_id: "test-makise".into(),
            agent_id: "makise".into(),
            expected_world_version: engine.state().world_version(),
            schema_version: 1,
            decision_id: "decision-future-event".into(),
            issued_at_ms: 1_000_010,
            ttl_ms: 30_000,
            payload: CommandPayload::MoveTo {
                target_anchor_id: "work_desk".into(),
            },
        };
        engine.execute_command(&command, 1_000_010).unwrap();
    }

    let connection = Connection::open(&database).unwrap();
    let envelope: String = connection
        .query_row(
            "SELECT envelope_json FROM events WHERE event_seq = 2",
            [],
            |row| row.get(0),
        )
        .unwrap();
    let mut value: serde_json::Value = serde_json::from_str(&envelope).unwrap();
    value["payload"]["type"] = "future_unknown_event".into();
    let corrupted = serde_json::to_string(&value).unwrap();
    let checksum = Sha256::digest(corrupted.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    connection
        .execute(
            "UPDATE events
             SET event_type = 'future_unknown_event',
                 envelope_json = ?1,
                 envelope_sha256 = ?2
             WHERE event_seq = 2",
            [&corrupted, &checksum],
        )
        .unwrap();
    drop(connection);

    let recovered = WorldEngine::open(
        &database,
        "test-makise",
        definition(),
        "bed",
        1_001_000,
        &PathGuard::default(),
    );
    assert!(recovered.is_err(), "unknown events must stop replay");
}
