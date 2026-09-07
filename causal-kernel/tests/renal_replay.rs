use makise_causal_kernel::{
    CommitError, CommitRequest, OpenSpec, StorageLocation, TimelineId, WorldEngine, WorldId,
};

fn spec(name: &str) -> OpenSpec {
    OpenSpec::new(
        WorldId::new(format!("{name}-world")).expect("valid"),
        TimelineId::new(format!("{name}-timeline")).expect("valid"),
    )
}

fn commit(engine: &mut WorldEngine) {
    engine
        .commit(CommitRequest::ingest_fluid("fluid", 0, 1_500_000, 86_400))
        .expect("commit fluid intake");
}

#[test]
fn renal_artifact_supports_fast_and_audit_replay_after_reopen() {
    let directory = tempfile::tempdir().expect("temp");
    let path = directory.path().join("timeline.sqlite");
    let (mut engine, _) =
        WorldEngine::open(spec("renal-replay"), StorageLocation::sqlite(&path)).expect("open");
    commit(&mut engine);
    let fast = engine.fast_replay().expect("fast replay");
    drop(engine);

    let (mut reopened, _) =
        WorldEngine::open(spec("renal-replay"), StorageLocation::sqlite(&path)).expect("reopen");
    assert_eq!(fast, reopened.audit_replay().expect("audit replay"));
    assert!(reopened.safe_stop().expect("read safe stop").is_none());
}

#[test]
fn successive_fluid_intakes_replay_from_the_preceding_committed_state() {
    let directory = tempfile::tempdir().expect("temp");
    let path = directory.path().join("timeline.sqlite");
    let (mut engine, _) =
        WorldEngine::open(spec("successive-intakes"), StorageLocation::sqlite(&path))
            .expect("open");
    engine
        .commit(CommitRequest::ingest_fluid("first", 0, 250_000, 35_000))
        .expect("first intake");
    drop(engine);

    let (mut engine, _) =
        WorldEngine::open(spec("successive-intakes"), StorageLocation::sqlite(&path))
            .expect("reopen");
    let second = CommitRequest::ingest_fluid("second", 1, 250_000, 35_000);
    engine.commit(second.clone()).expect("second intake");
    let expected = engine.organism().expect("organism").clone();
    // Two 250 ml / 35 mmol inputs add exactly 500 ml / 70 mmol.
    assert_eq!(expected.total_body_water_mm3(), 42_500_000);
    assert_eq!(expected.renal().plasma_sodium_umol(), 490_000);
    let fast = engine.fast_replay().expect("fast replay");
    drop(engine);

    let (mut reopened, _) =
        WorldEngine::open(spec("successive-intakes"), StorageLocation::sqlite(&path))
            .expect("reopen");
    assert_eq!(
        reopened.audit_replay().expect("audit successive intakes"),
        fast
    );
    assert!(reopened.safe_stop().expect("safe stop").is_none());
    assert_eq!(reopened.organism(), Some(&expected));
    assert!(reopened.commit(second).expect("retry").replayed_request());
    assert!(matches!(
        reopened.commit(CommitRequest::ingest_fluid("second", 1, 1, 0)),
        Err(CommitError::IdempotencyConflict)
    ));
    assert_eq!(reopened.fast_replay().expect("unchanged replay"), fast);
}

#[test]
fn missing_renal_artifact_durably_safe_stops_the_timeline() {
    let directory = tempfile::tempdir().expect("temp");
    let path = directory.path().join("timeline.sqlite");
    let (mut engine, _) = WorldEngine::open(
        spec("missing-renal-artifact"),
        StorageLocation::sqlite(&path),
    )
    .expect("open");
    commit(&mut engine);
    drop(engine);

    let connection = rusqlite::Connection::open(&path).expect("raw open");
    connection
        .execute("DELETE FROM artifact_archive", [])
        .expect("test corruption");
    drop(connection);

    let (mut reopened, _) = WorldEngine::open(
        spec("missing-renal-artifact"),
        StorageLocation::sqlite(&path),
    )
    .expect("reopen");
    assert!(reopened.audit_replay().is_err());
    assert!(reopened.safe_stop().expect("read safe stop").is_some());
    assert!(matches!(
        reopened.commit(CommitRequest::advance_to("blocked", 1, 1)),
        Err(CommitError::SafeStopped(_))
    ));
}

#[test]
fn digest_mismatched_renal_artifact_durably_safe_stops_the_timeline() {
    let directory = tempfile::tempdir().expect("temp");
    let path = directory.path().join("timeline.sqlite");
    let (mut engine, _) = WorldEngine::open(
        spec("mismatched-renal-artifact"),
        StorageLocation::sqlite(&path),
    )
    .expect("open");
    commit(&mut engine);
    drop(engine);

    let connection = rusqlite::Connection::open(&path).expect("raw open");
    connection
        .execute("UPDATE artifact_archive SET program_bytes = x'7b7d'", [])
        .expect("test corruption");
    drop(connection);

    let (mut reopened, _) = WorldEngine::open(
        spec("mismatched-renal-artifact"),
        StorageLocation::sqlite(&path),
    )
    .expect("reopen");
    assert!(reopened.audit_replay().is_err());
    assert!(reopened.safe_stop().expect("read safe stop").is_some());
}

#[test]
fn same_id_artifact_with_a_valid_new_digest_cannot_select_the_builtin_executor() {
    use sha2::{Digest, Sha256};

    let directory = tempfile::tempdir().expect("temp");
    let path = directory.path().join("timeline.sqlite");
    let (mut engine, _) = WorldEngine::open(
        spec("unknown-renal-program"),
        StorageLocation::sqlite(&path),
    )
    .expect("open");
    commit(&mut engine);
    let organism = engine.organism().expect("organism").clone();
    let fast = engine.fast_replay().expect("fast replay");
    drop(engine);

    // Fault injection only: maintain a valid content digest and the same ID,
    // but change the contract semantics. No built-in executor is bound to it.
    let mut artifact: serde_json::Value = serde_json::from_slice(include_bytes!(
        "../../contracts/fixtures/mechanisms/renal-fluid-electrolyte-v1.json"
    ))
    .expect("fixture");
    artifact["parameters"][0]["value"] = serde_json::json!(18);
    let bytes = serde_json::to_vec(&artifact).expect("encode changed artifact");
    let digest = format!("sha256:{:x}", Sha256::digest(&bytes));
    let connection = rusqlite::Connection::open(&path).expect("fault injection storage");
    connection
        .execute(
            "UPDATE artifact_archive SET digest=?1, program_bytes=?2",
            rusqlite::params![digest, bytes],
        )
        .expect("replace archived artifact");
    connection
        .execute(
            "UPDATE mechanism_execution SET artifact_digest=?1",
            [&digest],
        )
        .expect("replace execution reference");
    connection
        .execute(
            "UPDATE causal_transitions SET artifact_digests=?1",
            [serde_json::json!([digest]).to_string()],
        )
        .expect("replace event reference");
    drop(connection);

    let (mut reopened, _) = WorldEngine::open(
        spec("unknown-renal-program"),
        StorageLocation::sqlite(&path),
    )
    .expect("reopen");
    assert_eq!(reopened.fast_replay().expect("deltas still valid"), fast);
    assert!(
        reopened.audit_replay().is_err(),
        "same mechanism ID must not authorize an unknown executable identity"
    );
    assert!(reopened.safe_stop().expect("safe stop").is_some());
    assert_eq!(reopened.organism(), Some(&organism));
    assert_eq!(reopened.fast_replay().expect("unchanged deltas"), fast);
    drop(reopened);

    let (mut reopened, _) = WorldEngine::open(
        spec("unknown-renal-program"),
        StorageLocation::sqlite(&path),
    )
    .expect("reopen stopped timeline");
    assert!(reopened.safe_stop().expect("durable safe stop").is_some());
    assert!(matches!(
        reopened.commit(CommitRequest::ingest_fluid("blocked", 1, 1, 0)),
        Err(CommitError::SafeStopped(_))
    ));
    assert_eq!(reopened.organism(), Some(&organism));
}

#[test]
fn renal_audit_rejects_tampered_delta_or_conservation() {
    for (name, statement) in [
        (
            "renal-delta",
            "UPDATE causal_transitions SET unit_deltas = '[]'",
        ),
        (
            "renal-conservation",
            "UPDATE causal_transitions SET conservation_report = '{\"kind\":\"verified\",\"quantity\":\"water.body_plus_urine\",\"unit\":\"mm3\",\"opening\":42000000,\"external_input\":1500000,\"closing\":43500000,\"residual\":1}'",
        ),
    ] {
        let directory = tempfile::tempdir().expect("temp");
        let path = directory.path().join("timeline.sqlite");
        let (mut engine, _) =
            WorldEngine::open(spec(name), StorageLocation::sqlite(&path)).expect("open");
        commit(&mut engine);
        drop(engine);

        let connection = rusqlite::Connection::open(&path).expect("raw open");
        connection.execute(statement, []).expect("test corruption");
        drop(connection);

        let (mut reopened, _) =
            WorldEngine::open(spec(name), StorageLocation::sqlite(&path)).expect("reopen");
        assert!(reopened.audit_replay().is_err());
        assert!(reopened.safe_stop().expect("read safe stop").is_some());
    }
}

#[test]
fn renal_audit_requires_the_event_and_execution_to_reference_the_same_artifact() {
    let directory = tempfile::tempdir().expect("temp");
    let path = directory.path().join("timeline.sqlite");
    let (mut engine, _) = WorldEngine::open(
        spec("renal-artifact-reference"),
        StorageLocation::sqlite(&path),
    )
    .expect("open");
    commit(&mut engine);
    let organism = engine.organism().expect("organism").clone();
    drop(engine);

    let connection = rusqlite::Connection::open(&path).expect("fault injection storage");
    connection
        .execute("UPDATE causal_transitions SET artifact_digests='[]'", [])
        .expect("remove only the public artifact reference");
    drop(connection);

    let (mut reopened, _) = WorldEngine::open(
        spec("renal-artifact-reference"),
        StorageLocation::sqlite(&path),
    )
    .expect("reopen");
    assert!(
        reopened.audit_replay().is_err(),
        "execution metadata cannot substitute for committed artifact identity"
    );
    assert!(reopened.safe_stop().expect("safe stop").is_some());
    assert_eq!(reopened.organism(), Some(&organism));
}

#[test]
fn invalid_archived_renal_input_durably_stops_without_changing_state() {
    for input in [
        "{}",
        r#"{"water_mm3":-1,"sodium_umol":0}"#,
        r#"{"water_mm3":1500000,"sodium_umol":86400,"unknown":true}"#,
    ] {
        let directory = tempfile::tempdir().unwrap();
        let storage = StorageLocation::sqlite(directory.path().join("timeline.sqlite"));
        let (mut engine, _) = WorldEngine::open(spec("invalid-input"), storage.clone()).unwrap();
        commit(&mut engine);
        let organism = engine.organism().unwrap().clone();
        let fast = engine.fast_replay().unwrap();
        drop(engine);
        let connection =
            rusqlite::Connection::open(directory.path().join("timeline.sqlite")).unwrap();
        connection
            .execute("UPDATE mechanism_execution SET input_json=?1", [input])
            .unwrap();
        drop(connection);
        let (mut engine, _) = WorldEngine::open(spec("invalid-input"), storage.clone()).unwrap();
        assert!(engine.audit_replay().is_err());
        assert!(
            engine.safe_stop().unwrap().is_some(),
            "invalid input: {input}"
        );
        assert_eq!(engine.organism(), Some(&organism));
        assert_eq!(engine.fast_replay().unwrap(), fast);
        drop(engine);
        let (mut engine, _) = WorldEngine::open(spec("invalid-input"), storage).unwrap();
        assert!(engine.safe_stop().unwrap().is_some());
        assert!(matches!(
            engine.commit(CommitRequest::ingest_fluid("blocked", 1, 1, 0)),
            Err(CommitError::SafeStopped(_))
        ));
    }
}
