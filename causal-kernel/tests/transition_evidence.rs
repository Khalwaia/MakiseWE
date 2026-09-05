use makise_causal_kernel::{
    CommitRequest, EventCursor, EventQuery, OpenSpec, RigidBody, StorageLocation, TimelineId,
    WorldEngine, WorldId,
};

fn spec(name: &str) -> OpenSpec {
    OpenSpec::new(
        WorldId::new(format!("{name}-world")).expect("valid"),
        TimelineId::new(format!("{name}-timeline")).expect("valid"),
    )
}

fn body(x_nm: i64) -> RigidBody {
    RigidBody::new(
        1_000_000,
        [x_nm, 0, 0],
        [0; 3],
        [0; 3],
        [1_000_000; 3],
        [0; 3],
    )
    .expect("valid body")
}

#[test]
fn durable_transition_exposes_canonical_evidence_and_hashes_all_body_state() {
    let first_directory = tempfile::tempdir().expect("temp dir");
    let first_path = first_directory.path().join("first.sqlite");
    let (mut first, _) =
        WorldEngine::open(spec("first"), StorageLocation::sqlite(&first_path)).expect("open first");
    first
        .commit(CommitRequest::place_body(
            "place-kettle",
            0,
            "kettle",
            body(0),
        ))
        .expect("place first body");
    drop(first);

    let (first, _) = WorldEngine::open(spec("first"), StorageLocation::sqlite(&first_path))
        .expect("reopen first");
    let first_page = first
        .events(EventQuery::new(EventCursor::start(), 1).expect("valid query"))
        .expect("read first event");
    let first_event = first_page.events().first().expect("one durable transition");

    assert_eq!(first_event.request_id(), Some("place-kettle"));
    assert_eq!(
        first_event.causes().expect("new event has causes")[0].kind(),
        "place_body"
    );
    assert!(
        first_event
            .unit_deltas()
            .expect("new event has unit-typed deltas")
            .iter()
            .any(|delta| {
                delta.quantity() == "body/kettle.position.x"
                    && delta.unit() == "nm"
                    && delta.before().is_none()
                    && delta.after() == Some(0)
            })
    );
    assert!(
        first_event
            .artifact_digests()
            .expect("artifact references are explicit even when empty")
            .is_empty()
    );
    assert!(
        first_event
            .uncertainty()
            .expect("new event has an uncertainty report")
            .is_unknown()
    );
    assert!(
        !first_event
            .conservation_report()
            .expect("new event has a conservation report")
            .is_verified()
    );

    let second_directory = tempfile::tempdir().expect("temp dir");
    let (mut second, _) = WorldEngine::open(
        spec("second"),
        StorageLocation::sqlite(second_directory.path().join("second.sqlite")),
    )
    .expect("open second");
    second
        .commit(CommitRequest::place_body(
            "place-kettle",
            0,
            "kettle",
            body(1),
        ))
        .expect("place shifted body");
    let second_event = second
        .events(EventQuery::new(EventCursor::start(), 1).expect("valid query"))
        .expect("read second event")
        .events()[0]
        .clone();

    assert_ne!(
        first_event.resulting_state_hash(),
        second_event.resulting_state_hash(),
        "every authoritative rigid-body field belongs to the state hash"
    );
}
