use makise_causal_kernel::{
    CommitError, CommitReceipt, CommitRequest, EventCursor, EventPage, EventQuery, OpenError,
    OpenSpec, Projection, ProjectionError, ProjectionRequest, ReadError, RecoveryReport,
    RecoveryStatus, StorageLocation, TimelineId, WorldEngine, WorldId,
};

#[test]
fn timeline_can_be_created_and_reopened() {
    let directory = tempfile::tempdir().expect("create temporary timeline directory");
    let storage = StorageLocation::sqlite(directory.path().join("timeline.sqlite"));
    let spec = OpenSpec::new(
        WorldId::new("world-alpha").expect("valid world ID"),
        TimelineId::new("timeline-main").expect("valid timeline ID"),
    );

    let (engine, created) =
        WorldEngine::open(spec.clone(), storage.clone()).expect("create V1 timeline");
    assert_eq!(created.status(), RecoveryStatus::Created);
    drop(engine);

    let (_engine, reopened) = WorldEngine::open(spec, storage).expect("reopen V1 timeline");
    assert_eq!(reopened.status(), RecoveryStatus::Recovered);
}

#[test]
fn new_timeline_has_an_empty_projection() {
    let directory = tempfile::tempdir().expect("create temporary timeline directory");
    let storage = StorageLocation::sqlite(directory.path().join("timeline.sqlite"));
    let timeline_id = TimelineId::new("timeline-main").expect("valid timeline ID");
    let spec = OpenSpec::new(
        WorldId::new("world-alpha").expect("valid world ID"),
        timeline_id.clone(),
    );
    let (engine, _) = WorldEngine::open(spec, storage).expect("create V1 timeline");

    let projection = engine
        .project(ProjectionRequest::current())
        .expect("project empty timeline");

    assert_eq!(projection.timeline_id(), &timeline_id);
    assert_eq!(projection.timeline_version(), 0);
    assert!(projection.is_empty());
}

#[test]
fn empty_event_pagination_is_stable_across_reopen() {
    let directory = tempfile::tempdir().expect("create temporary timeline directory");
    let storage = StorageLocation::sqlite(directory.path().join("timeline.sqlite"));
    let spec = OpenSpec::new(
        WorldId::new("world-alpha").expect("valid world ID"),
        TimelineId::new("timeline-main").expect("valid timeline ID"),
    );
    let query = EventQuery::new(EventCursor::start(), 2).expect("valid event query");

    let (engine, _) = WorldEngine::open(spec.clone(), storage.clone()).expect("create V1 timeline");
    let created_page = engine.events(query.clone()).expect("read created timeline");
    assert!(created_page.events().is_empty());
    assert_eq!(created_page.next_cursor(), EventCursor::start());
    drop(engine);

    let (engine, _) = WorldEngine::open(spec, storage).expect("reopen V1 timeline");
    let reopened_page = engine.events(query).expect("read reopened timeline");
    assert_eq!(reopened_page, created_page);
}

#[test]
fn world_engine_exposes_the_four_operation_boundary() {
    let _: fn(OpenSpec, StorageLocation) -> Result<(WorldEngine, RecoveryReport), OpenError> =
        WorldEngine::open;
    let _: fn(&mut WorldEngine, CommitRequest) -> Result<CommitReceipt, CommitError> =
        WorldEngine::commit;
    let _: fn(&WorldEngine, ProjectionRequest) -> Result<Projection, ProjectionError> =
        WorldEngine::project;
    let _: fn(&WorldEngine, EventQuery) -> Result<EventPage, ReadError> = WorldEngine::events;
}

#[test]
fn open_rejects_non_v1_storage_without_changing_it() {
    let directory = tempfile::tempdir().expect("create temporary timeline directory");
    let path = directory.path().join("legacy.sqlite");
    let connection = rusqlite::Connection::open(&path).expect("create representative archive");
    connection
        .execute_batch(
            "CREATE TABLE legacy_events (payload BLOB NOT NULL);
             INSERT INTO legacy_events (payload) VALUES (X'4d616b697365');",
        )
        .expect("write representative archive");
    drop(connection);
    let original = std::fs::read(&path).expect("read representative archive");
    let spec = OpenSpec::new(
        WorldId::new("world-alpha").expect("valid world ID"),
        TimelineId::new("timeline-main").expect("valid timeline ID"),
    );

    let error = WorldEngine::open(spec, StorageLocation::sqlite(&path))
        .err()
        .expect("reject non-V1 storage");

    assert!(matches!(error, OpenError::IncompatibleStorage));
    assert_eq!(
        std::fs::read(path).expect("reread representative archive"),
        original
    );
}
