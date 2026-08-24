use makise_causal_kernel::{
    CommitError, CommitRequest, OpenSpec, ResolutionChanged, StorageLocation, TimelineId,
    WorldEngine, WorldId,
};

fn spec(name: &str) -> OpenSpec {
    OpenSpec::new(
        WorldId::new(format!("{name}-world")).expect("valid"),
        TimelineId::new(format!("{name}-timeline")).expect("valid"),
    )
}

#[test]
fn resolution_change_is_explicit_durable_and_replayable() {
    let dir = tempfile::tempdir().expect("temp");
    let path = dir.path().join("t.sqlite");
    let (mut engine, _) =
        WorldEngine::open(spec("res"), StorageLocation::sqlite(&path)).expect("open");

    let change = ResolutionChanged::new("cohort-v1", "individual-cell-set-v1", 0x2042)
        .expect("valid resolution change");
    let request = CommitRequest::resolution_changed("change-1", 0, change);
    engine.commit(request.clone()).expect("commit change");

    assert_eq!(
        engine.resolution_id().expect("active"),
        "individual-cell-set-v1"
    );

    // Same request replays; conflicting payload rejects.
    let replay = engine
        .commit(CommitRequest::resolution_changed("change-1", 0, change))
        .expect("replay");
    assert!(replay.replayed_request());

    drop(engine);
    let (reopened, _) =
        WorldEngine::open(spec("res"), StorageLocation::sqlite(path)).expect("reopen");
    assert_eq!(
        reopened.resolution_id().expect("restored"),
        "individual-cell-set-v1"
    );
}

#[test]
fn resolution_change_preserves_organism_energy_exactly() {
    let dir = tempfile::tempdir().expect("temp");
    let (mut engine, _) = WorldEngine::open(
        spec("preserve"),
        StorageLocation::sqlite(dir.path().join("t.sqlite")),
    )
    .expect("open");

    engine
        .commit(CommitRequest::advance_to("advance", 0, 10))
        .expect("advance first");
    let before = engine.organism().unwrap();
    let before_total = before.total_accounted_uj() + before.ambient_internal_energy_uj();

    let change = ResolutionChanged::new("cohort-v1", "individual-cell-set-v1", 7).unwrap();
    engine
        .commit(CommitRequest::resolution_changed("change", 1, change))
        .expect("change resolution");

    let after = engine.organism().unwrap();
    let after_total = after.total_accounted_uj() + after.ambient_internal_energy_uj();
    assert_eq!(before_total, after_total);

    let error = engine
        .commit(CommitRequest::advance_to("stale", 1, 1))
        .expect_err("head moved by resolution transition");
    assert!(matches!(error, CommitError::ExpectedVersionConflict));
}
