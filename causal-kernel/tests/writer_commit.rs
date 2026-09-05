use makise_causal_kernel::{
    CommitError, CommitRequest, OpenSpec, StorageLocation, TimelineId, WorldEngine, WorldId,
};

fn open_engine() -> (tempfile::TempDir, WorldEngine) {
    let directory = tempfile::tempdir().expect("temp dir");
    let spec = OpenSpec::new(
        WorldId::new("world-alpha").expect("valid"),
        TimelineId::new("timeline-main").expect("valid"),
    );
    let (engine, _) = WorldEngine::open(
        spec,
        StorageLocation::sqlite(directory.path().join("t.sqlite")),
    )
    .expect("open");
    (directory, engine)
}

fn advance_request(request_id: &str, expected_version: u64, seconds: i64) -> CommitRequest {
    CommitRequest::advance_to(request_id, expected_version, seconds)
}

#[test]
fn same_request_id_replays_original_receipt() {
    let (_dir, mut engine) = open_engine();
    let request = advance_request("req-1", 0, 3);

    let first = engine.commit(request.clone()).expect("first commit");
    let second = engine.commit(request).expect("retry commit");

    assert!(second.replayed_request());
    assert_eq!(first.timeline_version(), second.timeline_version());
}

#[test]
fn conflicting_payload_for_same_id_is_rejected() {
    let (_dir, mut engine) = open_engine();

    engine
        .commit(advance_request("req-1", 0, 3))
        .expect("first commit");

    let error = engine
        .commit(advance_request("req-1", 0, 5))
        .expect_err("conflicting payload must be rejected");

    assert!(matches!(error, CommitError::IdempotencyConflict));
}

#[test]
fn stale_expected_version_is_rejected_without_mutation() {
    let (_dir, mut engine) = open_engine();

    engine
        .commit(advance_request("req-1", 0, 2))
        .expect("first commit moves head");

    let error = engine
        .commit(advance_request("req-2", 0, 2))
        .expect_err("stale version must be rejected");

    assert!(matches!(error, CommitError::ExpectedVersionConflict));
}

#[test]
fn failed_durable_append_leaves_the_authoritative_state_unchanged() {
    let (directory, mut engine) = open_engine();
    let path = directory.path().join("t.sqlite");
    let lock = rusqlite::Connection::open(path).expect("open competing SQLite connection");
    lock.execute_batch("BEGIN IMMEDIATE")
        .expect("hold the writer lock after the engine is open");

    let request = advance_request("req-after-failed-append", 0, 3);
    let error = engine
        .commit(request.clone())
        .expect_err("a blocked durable append must fail");
    assert!(matches!(error, CommitError::Storage(_)));

    let projection = engine
        .project(makise_causal_kernel::ProjectionRequest::current())
        .expect("project after rejected commit");
    assert_eq!(projection.timeline_version(), 0);
    assert_eq!(projection.simulated_second(), 0);

    lock.execute_batch("ROLLBACK")
        .expect("release competing writer lock");
    let receipt = engine
        .commit(request)
        .expect("the same request commits once after storage recovers");
    assert_eq!(receipt.timeline_version(), 1);
    assert!(!receipt.replayed_request());
}
