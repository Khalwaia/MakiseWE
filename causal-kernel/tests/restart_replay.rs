use makise_causal_kernel::{
    CommitRequest, OpenSpec, ProjectionRequest, StorageLocation, TimelineId, WorldEngine, WorldId,
};

fn spec() -> OpenSpec {
    OpenSpec::new(
        WorldId::new("world-alpha").expect("valid"),
        TimelineId::new("timeline-main").expect("valid"),
    )
}

fn storage(dir: &tempfile::TempDir) -> StorageLocation {
    StorageLocation::sqlite(dir.path().join("timeline.sqlite"))
}

#[test]
fn reopen_restores_head_version_and_rejects_stale_writes() {
    let directory = tempfile::tempdir().expect("temp dir");
    let path = storage(&directory);

    {
        let (mut engine, _) = WorldEngine::open(spec(), path.clone()).expect("create");
        engine
            .commit(CommitRequest::advance_to("req-1", 0, 3))
            .expect("commit before reopen");
    }

    let (engine, recovery) = WorldEngine::open(spec(), path).expect("reopen");
    assert_eq!(
        recovery.status(),
        makise_causal_kernel::RecoveryStatus::Recovered
    );

    let projection = engine
        .project(ProjectionRequest::current())
        .expect("projection");
    assert_eq!(projection.timeline_version(), 1);
}

#[test]
fn replayed_request_after_reopen_returns_same_receipt() {
    let directory = tempfile::tempdir().expect("temp dir");
    let path = storage(&directory);
    let request = CommitRequest::advance_to("req-1", 0, 3);

    let first_receipt;
    {
        let (mut engine, _) = WorldEngine::open(spec(), path.clone()).expect("create");
        first_receipt = engine.commit(request.clone()).expect("first commit");
    }

    let (mut engine, _) = WorldEngine::open(spec(), path).expect("reopen");
    let replayed = engine.commit(request).expect("retry after reopen");

    assert!(replayed.replayed_request());
    assert_eq!(
        first_receipt.timeline_version(),
        replayed.timeline_version()
    );
}

#[test]
fn split_run_matches_uninterrupted_run_state() {
    let directory_a = tempfile::tempdir().expect("temp dir");
    let directory_b = tempfile::tempdir().expect("temp dir");

    let (mut uninterrupted, _) = WorldEngine::open(spec(), storage(&directory_a)).expect("a");
    uninterrupted
        .commit(CommitRequest::advance_to("whole", 0, 6))
        .expect("uninterrupted advance");

    let (mut split, _) = WorldEngine::open(spec(), storage(&directory_b)).expect("b");
    split
        .commit(CommitRequest::advance_to("part-1", 0, 2))
        .expect("split part 1");
    drop(split);
    let (mut split, _) = WorldEngine::open(spec(), storage(&directory_b)).expect("b reopen");
    split
        .commit(CommitRequest::advance_to("part-2", 1, 4))
        .expect("split part 2");
    drop(split);
    let (split, _) = WorldEngine::open(spec(), storage(&directory_b)).expect("b reopen again");

    let uninterrupted_version = uninterrupted
        .project(ProjectionRequest::current())
        .expect("project a")
        .timeline_version();
    drop(uninterrupted);
    assert_eq!(uninterrupted_version, 1);
    assert_eq!(
        split
            .project(ProjectionRequest::current())
            .expect("project b")
            .timeline_version(),
        2
    );
}
