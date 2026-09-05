use makise_causal_kernel::{
    CommitRequest, OpenSpec, RigidBody, StorageLocation, TimelineId, WorldEngine, WorldId,
};

fn spec() -> OpenSpec {
    OpenSpec::new(
        WorldId::new("replay-basis-world").expect("valid world id"),
        TimelineId::new("replay-basis-timeline").expect("valid timeline id"),
    )
}

fn body() -> RigidBody {
    RigidBody::new(1_000_000, [0; 3], [0; 3], [0; 3], [1_000_000; 3], [0; 3]).expect("valid body")
}

#[test]
fn new_timeline_has_immutable_genesis_hash_that_anchors_first_transition() {
    let directory = tempfile::tempdir().expect("temp dir");
    let path = directory.path().join("timeline.sqlite");
    let (mut engine, _) =
        WorldEngine::open(spec(), StorageLocation::sqlite(&path)).expect("open timeline");

    let genesis = engine
        .genesis()
        .expect("read genesis")
        .expect("new timeline must have a durable genesis snapshot");
    assert_eq!(genesis.sequence(), 0);
    assert_eq!(genesis.state_hash_version(), 2);
    assert_eq!(genesis.simulated_second(), 0);

    engine
        .commit(CommitRequest::place_body("place", 0, "kettle", body()))
        .expect("commit first transition");
    let first = engine
        .events(
            makise_causal_kernel::EventQuery::new(makise_causal_kernel::EventCursor::start(), 1)
                .expect("valid query"),
        )
        .expect("read first transition");
    assert_eq!(
        first.events()[0].previous_state_hash(),
        genesis.state_hash(),
        "first transition must be anchored to exact durable initial state"
    );

    drop(engine);
    let (reopened, _) =
        WorldEngine::open(spec(), StorageLocation::sqlite(&path)).expect("reopen timeline");
    assert_eq!(
        reopened.genesis().expect("read genesis"),
        Some(genesis),
        "reopen must not reconstruct or alter genesis"
    );
}
