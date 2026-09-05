use makise_causal_kernel::{
    CommitRequest, EventCursor, EventQuery, OpenError, OpenSpec, ProjectionRequest,
    StorageLocation, TimelineFormat, TimelineId, WorldEngine, WorldId,
};

fn spec() -> OpenSpec {
    OpenSpec::new(
        WorldId::new("format-world").expect("world id"),
        TimelineId::new("format-timeline").expect("timeline id"),
    )
}

#[test]
fn canonical_requirement_cannot_reinterpret_an_aggregate_timeline() {
    let directory = tempfile::tempdir().expect("temp dir");
    let path = directory.path().join("timeline.sqlite");
    let storage = StorageLocation::sqlite(&path);
    let request = CommitRequest::advance_to("three-seconds", 0, 3);
    let query = EventQuery::new(EventCursor::start(), 10).expect("event query");
    let (mut engine, _) = WorldEngine::open(spec(), storage.clone()).expect("create");
    let receipt = engine.commit(request.clone()).expect("commit");
    let events = engine.events(query.clone()).expect("events");
    assert_eq!(events.events().len(), 1);
    let projection = engine
        .project(ProjectionRequest::current())
        .expect("project");
    drop(engine);
    let original = std::fs::read(&path).expect("original archive bytes");

    let error = WorldEngine::open(
        spec().with_format(TimelineFormat::CanonicalPhysiologyV2),
        storage.clone(),
    )
    .err()
    .expect("reject incompatible format requirement");
    assert!(matches!(
        error,
        OpenError::FormatMismatch {
            requested: TimelineFormat::CanonicalPhysiologyV2,
            stored: TimelineFormat::AggregateV1,
        }
    ));
    assert_eq!(std::fs::read(&path).expect("archive bytes"), original);

    let (mut reopened, _) = WorldEngine::open(spec(), storage).expect("legacy reopen");
    assert_eq!(reopened.events(query).expect("same events"), events);
    assert_eq!(
        reopened
            .project(ProjectionRequest::current())
            .expect("same projection"),
        projection
    );
    let retried = reopened.commit(request).expect("retry original request");
    assert!(retried.replayed_request());
    assert_eq!(retried.timeline_version(), receipt.timeline_version());
}

#[test]
fn unavailable_canonical_creation_does_not_leave_an_aggregate_database() {
    let directory = tempfile::tempdir().expect("temp dir");
    let path = directory.path().join("timeline.sqlite");
    let result = WorldEngine::open(
        spec().with_format(TimelineFormat::CanonicalPhysiologyV2),
        StorageLocation::sqlite(&path),
    );
    assert!(matches!(
        result,
        Err(OpenError::UnsupportedTimelineFormat(
            TimelineFormat::CanonicalPhysiologyV2
        ))
    ));
    assert!(!path.exists(), "rejected creation must not create storage");
}

#[test]
fn unavailable_canonical_creation_preserves_existing_empty_storage() {
    let directory = tempfile::tempdir().expect("temp dir");
    let path = directory.path().join("timeline.sqlite");
    std::fs::write(&path, []).expect("empty storage");
    let result = WorldEngine::open(
        spec().with_format(TimelineFormat::CanonicalPhysiologyV2),
        StorageLocation::sqlite(&path),
    );
    assert!(matches!(
        result,
        Err(OpenError::UnsupportedTimelineFormat(
            TimelineFormat::CanonicalPhysiologyV2
        ))
    ));
    assert!(std::fs::read(&path).expect("unchanged storage").is_empty());
}

#[test]
fn pre_format_metadata_reopens_without_rewriting_the_archive() {
    let directory = tempfile::tempdir().expect("temp dir");
    let path = directory.path().join("timeline.sqlite");
    let storage = StorageLocation::sqlite(&path);
    let (engine, _) = WorldEngine::open(spec(), storage.clone()).expect("create");
    let genesis = engine.genesis().expect("genesis");
    drop(engine);
    // Compatibility fixture: the metadata schema used before ADR-0016.
    let connection = rusqlite::Connection::open(&path).expect("fixture storage");
    connection
        .execute_batch(
            "DROP TABLE timeline_metadata;
         CREATE TABLE timeline_metadata (
             singleton INTEGER PRIMARY KEY CHECK (singleton = 1),
             world_id TEXT NOT NULL,
             timeline_id TEXT NOT NULL
         );
         INSERT INTO timeline_metadata VALUES (1, 'format-world', 'format-timeline');",
        )
        .expect("pre-format metadata");
    drop(connection);
    let original = std::fs::read(&path).expect("original bytes");
    let (engine, _) = WorldEngine::open(spec().with_format(TimelineFormat::AggregateV1), storage)
        .expect("pre-format reopen");
    assert_eq!(engine.genesis().expect("same genesis"), genesis);
    drop(engine);
    assert_eq!(std::fs::read(&path).expect("archive bytes"), original);
}

#[test]
fn unknown_durable_format_is_rejected_before_any_recovery_write() {
    let directory = tempfile::tempdir().expect("temp dir");
    let path = directory.path().join("timeline.sqlite");
    let storage = StorageLocation::sqlite(&path);
    let (engine, _) = WorldEngine::open(spec(), storage.clone()).expect("create");
    drop(engine);
    // Fault injection: an unrecognized format cannot fall back to aggregate semantics.
    let connection = rusqlite::Connection::open(&path).expect("fault injection");
    connection
        .execute(
            "UPDATE timeline_metadata SET timeline_format='unknown-format'",
            [],
        )
        .expect("replace format");
    drop(connection);
    let original = std::fs::read(&path).expect("original bytes");
    for open_spec in [spec(), spec().with_format(TimelineFormat::AggregateV1)] {
        assert!(matches!(
            WorldEngine::open(open_spec, storage.clone()),
            Err(OpenError::IncompatibleStorage)
        ));
        assert_eq!(std::fs::read(&path).expect("unchanged bytes"), original);
    }
}
