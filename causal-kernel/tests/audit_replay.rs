use makise_causal_kernel::{
    CommitRequest, EventCursor, EventQuery, OpenError, OpenSpec, ProjectionRequest,
    StorageLocation, TimelineId, WorldEngine, WorldId,
};

fn spec(name: &str) -> OpenSpec {
    OpenSpec::new(
        WorldId::new(format!("{name}-world")).expect("valid"),
        TimelineId::new(format!("{name}-timeline")).expect("valid"),
    )
}

#[test]
fn committed_intervals_are_durable_and_readable_after_reopen() {
    let directory = tempfile::tempdir().expect("temp dir");
    let path = directory.path().join("t.sqlite");

    let (mut engine, _) =
        WorldEngine::open(spec("events"), StorageLocation::sqlite(&path)).expect("create");
    engine
        .commit(CommitRequest::advance_to("first", 0, 3))
        .expect("first advance");
    engine
        .commit(CommitRequest::advance_to("second", 1, 2))
        .expect("second advance");
    drop(engine);

    let (engine, recovery) =
        WorldEngine::open(spec("events"), StorageLocation::sqlite(&path)).expect("reopen");
    use makise_causal_kernel::RecoveryStatus;
    assert_eq!(recovery.status(), RecoveryStatus::Recovered);

    let query = EventQuery::new(EventCursor::start(), 10).expect("valid query");
    let page = engine.events(query).expect("read transitions");
    assert_eq!(page.events().len(), 2);
    assert_eq!(page.events()[0].sequence(), 1);
    assert_eq!(page.events()[0].interval_start_second(), 0);
    assert_eq!(page.events()[0].interval_end_second(), 3);
    assert_eq!(page.events()[1].sequence(), 2);
    assert_eq!(page.events()[1].interval_end_second(), 5);

    let projection = engine
        .project(ProjectionRequest::current())
        .expect("projection");
    assert_eq!(projection.simulated_second(), 5);
}

#[test]
fn event_pagination_is_stable() {
    let directory = tempfile::tempdir().expect("temp dir");
    let (mut engine, _) = WorldEngine::open(
        spec("pages"),
        StorageLocation::sqlite(directory.path().join("t.sqlite")),
    )
    .expect("create");
    for index in 0..3u64 {
        engine
            .commit(CommitRequest::advance_to(&format!("req-{index}"), index, 1))
            .expect("advance");
    }

    let first_query = EventQuery::new(EventCursor::start(), 2).expect("valid query");
    let first_page = engine.events(first_query).expect("first page");
    assert_eq!(first_page.events().len(), 2);
    let second_page = engine
        .events(EventQuery::new(first_page.next_cursor(), 2).expect("valid query"))
        .expect("second page");
    assert_eq!(second_page.events().len(), 1);
    assert_eq!(second_page.events()[0].sequence(), 3);
}

#[test]
fn corrupted_transition_chain_is_rejected_on_open() {
    let directory = tempfile::tempdir().expect("temp dir");
    let path = directory.path().join("t.sqlite");
    {
        let (mut engine, _) =
            WorldEngine::open(spec("corrupt"), StorageLocation::sqlite(&path)).expect("create");
        engine
            .commit(CommitRequest::advance_to("first", 0, 3))
            .expect("advance");
    }

    let connection = rusqlite::Connection::open(&path).expect("open raw database");
    connection
        .execute(
            "UPDATE causal_transitions SET interval_end_second = 99 WHERE sequence = 1",
            [],
        )
        .expect("tamper with committed transition");
    drop(connection);

    let error = WorldEngine::open(spec("corrupt"), StorageLocation::sqlite(&path))
        .err()
        .expect("corrupt chain must be rejected");
    assert!(matches!(error, OpenError::CorruptTransitionChain));
}
