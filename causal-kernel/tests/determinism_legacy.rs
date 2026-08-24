use makise_causal_kernel::{
    CommitRequest, OpenError, OpenSpec, Projection, ProjectionRequest, StorageLocation, TimelineId,
    WorldEngine, WorldId,
};

fn spec() -> OpenSpec {
    OpenSpec::new(
        WorldId::new("world-alpha").expect("valid"),
        TimelineId::new("timeline-main").expect("valid"),
    )
}

fn projection(engine: &WorldEngine) -> Projection {
    engine
        .project(ProjectionRequest::current())
        .expect("projection must succeed")
}

#[test]
fn partitioned_requests_match_single_request() {
    let directory_a = tempfile::tempdir().expect("temp dir");
    let directory_b = tempfile::tempdir().expect("temp dir");

    let (mut whole, _) = WorldEngine::open(
        spec(),
        StorageLocation::sqlite(directory_a.path().join("t.sqlite")),
    )
    .expect("open a");
    whole
        .commit(CommitRequest::advance_to("whole", 0, 6))
        .expect("whole advance");

    let (mut parts, _) = WorldEngine::open(
        spec(),
        StorageLocation::sqlite(directory_b.path().join("t.sqlite")),
    )
    .expect("open b");
    for index in 0..6u64 {
        parts
            .commit(CommitRequest::advance_to(
                &format!("part-{index}"),
                index,
                1,
            ))
            .expect("partitioned advance");
    }

    let whole_projection = projection(&whole);
    let parts_projection = projection(&parts);
    assert_eq!(
        whole_projection.simulated_second(),
        parts_projection.simulated_second()
    );
}

#[test]
fn restart_between_partitions_preserves_state() {
    let directory = tempfile::tempdir().expect("temp dir");
    let path = directory.path().join("t.sqlite");

    let (mut engine, _) = WorldEngine::open(spec(), StorageLocation::sqlite(&path)).expect("open");
    engine
        .commit(CommitRequest::advance_to("first", 0, 2))
        .expect("first");
    drop(engine);

    let (mut engine, _) =
        WorldEngine::open(spec(), StorageLocation::sqlite(&path)).expect("reopen");
    engine
        .commit(CommitRequest::advance_to("second", 1, 4))
        .expect("second");

    let reference_directory = tempfile::tempdir().expect("temp dir");
    let (mut whole, _) = WorldEngine::open(
        spec(),
        StorageLocation::sqlite(reference_directory.path().join("t.sqlite")),
    )
    .expect("open reference");
    whole
        .commit(CommitRequest::advance_to("whole", 0, 6))
        .expect("whole");

    whole
        .commit(CommitRequest::advance_to("probe-whole", 1, 2))
        .expect("probe whole");
    engine
        .commit(CommitRequest::advance_to("probe-split", 2, 2))
        .expect("probe split");
    assert_eq!(
        projection(&whole).simulated_second(),
        projection(&engine).simulated_second()
    );
}

#[test]
fn v1_open_rejects_representative_legacy_archive_without_modifying_it() {
    let directory = tempfile::tempdir().expect("temp dir");
    let path = directory.path().join("legacy.sqlite");
    let connection = rusqlite::Connection::open(&path).expect("create legacy archive");
    connection
        .execute_batch(
            "CREATE TABLE legacy_events (payload BLOB NOT NULL);
             INSERT INTO legacy_events (payload) VALUES (X'4d616b697365');",
        )
        .expect("populate legacy archive");
    drop(connection);
    let original = std::fs::read(&path).expect("read archive bytes");

    let error = WorldEngine::open(spec(), StorageLocation::sqlite(&path))
        .err()
        .expect("legacy archive must be rejected");

    assert!(matches!(error, OpenError::IncompatibleStorage));
    assert_eq!(
        std::fs::read(&path).expect("reread archive bytes"),
        original
    );
}
