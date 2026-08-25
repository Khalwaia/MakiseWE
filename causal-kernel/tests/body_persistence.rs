//! Phase 2 evidence: metric rigid bodies become durable timeline
//! state. The public seam owns every mutation (`WorldEngine::commit`),
//! retries are idempotent, conflicting payloads are typed-rejected,
//! stale versions never mutate, and a reopened timeline restores
//! every staged body bit-exact — the restart half of the phase 2
//! gate's partition/restart/replay parity.

use makise_causal_kernel::{
    CommitError, CommitRequest, OpenSpec, ReadError, RigidBody, StorageLocation, TimelineId,
    WorldEngine, WorldId,
};

const TIMELINE_PATH: &str = "bodies.sqlite";

fn spec(world: &str, timeline: &str) -> OpenSpec {
    OpenSpec::new(
        WorldId::new(world).expect("valid"),
        TimelineId::new(timeline).expect("valid"),
    )
}

fn kettle(position_nm: [i64; 3], velocity_nm_per_s: [i64; 3]) -> RigidBody {
    RigidBody::new(
        1_200_000, // 1.2 kg
        position_nm,
        velocity_nm_per_s,
        [0, -50_000_000, 0],
        [2_000_000, 3_000_000, 2_500_000],
        [0, 0, 0],
    )
    .expect("valid body")
}

fn open(directory: &std::path::Path, world: &str, timeline: &str) -> WorldEngine {
    let (engine, _) = WorldEngine::open(
        spec(world, timeline),
        StorageLocation::sqlite(directory.join(TIMELINE_PATH)),
    )
    .expect("open");
    engine
}

/// A placed body survives engine drop and reopen bit-exact: every
/// declared field returns identically, including negative velocity
/// components and off-axis centre-of-mass offsets.
#[test]
fn placed_body_survives_restart_bit_exact() {
    let directory = tempfile::tempdir().expect("temp dir");
    let original = kettle(
        [10_000_000_000, 900_000_000, -4_000_000_000],
        [-250_000_000, 0, 750_000_000],
    );

    {
        let mut engine = open(directory.path(), "physics-world", "physics-timeline");
        engine
            .commit(CommitRequest::place_body(
                "stage-kettle",
                0,
                "kettle",
                original,
            ))
            .expect("staged");
    }

    let reopened = open(directory.path(), "physics-world", "physics-timeline");
    let restored = reopened.body("kettle").expect("readable").expect("present");
    assert_eq!(restored, original);
    assert_eq!(restored.mass_mg(), 1_200_000);
    assert_eq!(
        restored.position_nm(),
        [10_000_000_000, 900_000_000, -4_000_000_000]
    );
    assert_eq!(restored.velocity_nm_per_s(), [-250_000_000, 0, 750_000_000]);
}

/// Retrying the identical placement request replays the original
/// receipt instead of mutating again.
#[test]
fn body_placement_retry_is_idempotent() {
    let directory = tempfile::tempdir().expect("temp dir");
    let mut engine = open(directory.path(), "physics-world", "physics-timeline");
    let request = CommitRequest::place_body("stage-once", 0, "kettle", kettle([0; 3], [0; 3]));

    let first = engine.commit(request.clone()).expect("first");
    let second = engine.commit(request).expect("retry");

    assert!(second.replayed_request());
    assert_eq!(first.timeline_version(), second.timeline_version());
}

/// The same request id carrying a different pose is a conflicting
/// payload and must be rejected, not silently applied.
#[test]
fn conflicting_pose_for_same_request_is_rejected() {
    let directory = tempfile::tempdir().expect("temp dir");
    let mut engine = open(directory.path(), "physics-world", "physics-timeline");
    engine
        .commit(CommitRequest::place_body(
            "stage-dual",
            0,
            "kettle",
            kettle([0; 3], [0; 3]),
        ))
        .expect("first");

    let error = engine
        .commit(CommitRequest::place_body(
            "stage-dual",
            1,
            "kettle",
            kettle([5_000_000_000, 0, 0], [0; 3]),
        ))
        .expect_err("conflicting payload");
    assert!(matches!(error, CommitError::IdempotencyConflict));
}

/// A stale expected version is rejected before anything is written;
/// the previously committed body stays untouched.
#[test]
fn stale_version_rejection_leaves_the_body_untouched() {
    let directory = tempfile::tempdir().expect("temp dir");
    let mut engine = open(directory.path(), "physics-world", "physics-timeline");
    engine
        .commit(CommitRequest::place_body(
            "first-body",
            0,
            "kettle",
            kettle([1_000_000_000, 0, 0], [0; 3]),
        ))
        .expect("first");

    let error = engine
        .commit(CommitRequest::place_body(
            "second-body",
            0,
            "pot",
            kettle([2_000_000_000, 0, 0], [0; 3]),
        ))
        .expect_err("stale version");
    assert!(matches!(error, CommitError::ExpectedVersionConflict));
    assert!(engine.body("pot").expect("readable").is_none());
}

/// An empty body identifier is rejected at the commit boundary.
#[test]
fn empty_body_identifier_is_typed() {
    let directory = tempfile::tempdir().expect("temp dir");
    let mut engine = open(directory.path(), "physics-world", "physics-timeline");
    let error = engine
        .commit(CommitRequest::place_body(
            "blank-id",
            0,
            "   ",
            kettle([0; 3], [0; 3]),
        ))
        .expect_err("empty identifier");
    assert!(matches!(error, CommitError::InvalidBodyId));
}

/// Placing a body under an existing identifier moves it forward as an
/// authoritative delta: the record reflects the newest committed pose.
#[test]
fn replacing_a_body_advances_its_state() {
    let directory = tempfile::tempdir().expect("temp dir");
    let mut engine = open(directory.path(), "physics-world", "physics-timeline");
    engine
        .commit(CommitRequest::place_body(
            "v1",
            0,
            "kettle",
            kettle([0; 3], [0; 3]),
        ))
        .expect("v1");
    engine
        .commit(CommitRequest::place_body(
            "v2",
            1,
            "kettle",
            kettle([7_000_000_000, 0, 0], [0; 3]),
        ))
        .expect("v2");

    let moved = engine.body("kettle").expect("readable").expect("present");
    assert_eq!(moved.position_nm(), [7_000_000_000, 0, 0]);
}

/// Every accepted placement appends one canonical zero-width
/// transition; the event stream enumerates bodies in a stable order
/// across restart.
#[test]
fn placements_extend_the_canonical_event_stream() {
    let directory = tempfile::tempdir().expect("temp dir");
    {
        let mut engine = open(directory.path(), "physics-world", "physics-timeline");
        engine
            .commit(CommitRequest::place_body(
                "a",
                0,
                "pot",
                kettle([0; 3], [0; 3]),
            ))
            .expect("pot");
        engine
            .commit(CommitRequest::place_body(
                "b",
                1,
                "kettle",
                kettle([0; 3], [0; 3]),
            ))
            .expect("kettle");
    }

    let reopened = open(directory.path(), "physics-world", "physics-timeline");
    let page = reopened
        .events(
            makise_causal_kernel::EventQuery::new(makise_causal_kernel::EventCursor::start(), 10)
                .expect("valid query"),
        )
        .expect("readable events");
    assert_eq!(page.events().len(), 2, "one transition per placement");
    assert_eq!(page.next_cursor().offset(), 2);

    let ids = reopened.body_ids().expect("listing readable");
    assert_eq!(ids.len(), 2);
    assert!(ids.contains(&"pot".to_owned()) && ids.contains(&"kettle".to_owned()));
}

/// Externally corrupted body rows are typed rejections, not silent
/// garbage reconstruction (INVARIANTS §42): a non-positive mass cannot
/// come back as a body.
#[test]
fn corrupted_row_is_typed_on_read() {
    let directory = tempfile::tempdir().expect("temp dir");
    let path = directory.path().join(TIMELINE_PATH);
    {
        let mut engine = open(directory.path(), "physics-world", "physics-timeline");
        engine
            .commit(CommitRequest::place_body(
                "victim",
                0,
                "kettle",
                kettle([0; 3], [0; 3]),
            ))
            .expect("staged");
    }
    // Tamper outside the engine: no host process may repair silently.
    let connection = rusqlite::Connection::open(&path).expect("raw open");
    connection
        .execute(
            "UPDATE body_state SET mass_mg = 0 WHERE body_id = 'kettle'",
            [],
        )
        .expect("tampered");
    drop(connection);

    let reopened = open(directory.path(), "physics-world", "physics-timeline");
    let error = reopened
        .body("kettle")
        .expect_err("corrupted row must be typed");
    assert!(matches!(error, ReadError::CorruptBodyState));
}
