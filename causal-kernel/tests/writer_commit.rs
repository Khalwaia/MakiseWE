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
