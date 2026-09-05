use makise_causal_kernel::{
    ArtifactBundle, CommitError, CommitRequest, MechanismContract, OpenSpec, ProgramAbi,
    ReservoirPair, ReservoirState, StorageLocation, TimelineId, WorldEngine, WorldId,
};

fn spec(name: &str) -> OpenSpec {
    OpenSpec::new(
        WorldId::new(format!("{name}-world")).expect("valid"),
        TimelineId::new(format!("{name}-timeline")).expect("valid"),
    )
}

fn bundle() -> ArtifactBundle {
    let contract = MechanismContract::from_json(include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../contracts/fixtures/mechanisms/two-reservoir-thermal-exchange.json"
    )))
    .expect("fixture contract");
    ArtifactBundle::new(
        contract,
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../contracts/fixtures/mechanisms/two-reservoir-thermal-exchange.program.json"
        ))
        .to_vec(),
        ProgramAbi::ThermalExchangeV1,
    )
}

fn pair() -> ReservoirPair {
    ReservoirPair::new(
        ReservoirState::new(20_000_000, 10),
        ReservoirState::new(10_000_000, 10),
    )
}

#[test]
fn committed_thermal_artifact_supports_fast_and_audit_replay_after_reopen() {
    let directory = tempfile::tempdir().expect("temp");
    let path = directory.path().join("timeline.sqlite");
    let (mut engine, _) =
        WorldEngine::open(spec("thermal-replay"), StorageLocation::sqlite(&path)).expect("open");
    engine
        .commit(CommitRequest::thermal_exchange(
            "exchange",
            0,
            pair(),
            bundle(),
        ))
        .expect("commit thermal exchange");
    let fast = engine.fast_replay().expect("fast replay");
    drop(engine);

    let (mut reopened, _) =
        WorldEngine::open(spec("thermal-replay"), StorageLocation::sqlite(&path)).expect("reopen");
    let audit = reopened.audit_replay().expect("audit replay");
    assert_eq!(fast, audit);
    assert!(reopened.safe_stop().expect("read safe stop").is_none());
}

#[test]
fn missing_archived_bytes_durably_safe_stop_and_block_later_commits() {
    let directory = tempfile::tempdir().expect("temp");
    let path = directory.path().join("timeline.sqlite");
    let (mut engine, _) =
        WorldEngine::open(spec("missing-artifact"), StorageLocation::sqlite(&path)).expect("open");
    engine
        .commit(CommitRequest::thermal_exchange(
            "exchange",
            0,
            pair(),
            bundle(),
        ))
        .expect("commit thermal exchange");
    drop(engine);

    let connection = rusqlite::Connection::open(&path).expect("raw open");
    connection
        .execute("DELETE FROM artifact_archive", [])
        .expect("test corruption");
    drop(connection);

    let (mut engine, _) =
        WorldEngine::open(spec("missing-artifact"), StorageLocation::sqlite(&path))
            .expect("reopen");
    assert!(engine.audit_replay().is_err());
    assert!(engine.safe_stop().expect("read safe stop").is_some());
    assert!(matches!(
        engine.commit(CommitRequest::advance_to("blocked", 1, 1)),
        Err(CommitError::SafeStopped(_))
    ));
}

#[test]
fn digest_mismatched_archived_bytes_durably_safe_stop() {
    let directory = tempfile::tempdir().expect("temp");
    let path = directory.path().join("timeline.sqlite");
    let (mut engine, _) =
        WorldEngine::open(spec("mismatched-artifact"), StorageLocation::sqlite(&path))
            .expect("open");
    engine
        .commit(CommitRequest::thermal_exchange(
            "exchange",
            0,
            pair(),
            bundle(),
        ))
        .expect("commit thermal exchange");
    drop(engine);

    let connection = rusqlite::Connection::open(&path).expect("raw open");
    connection
        .execute("UPDATE artifact_archive SET program_bytes = x'7b7d'", [])
        .expect("test corruption");
    drop(connection);

    let (mut engine, _) =
        WorldEngine::open(spec("mismatched-artifact"), StorageLocation::sqlite(&path))
            .expect("reopen");
    assert!(engine.audit_replay().is_err());
    assert!(engine.safe_stop().expect("read safe stop").is_some());
}
