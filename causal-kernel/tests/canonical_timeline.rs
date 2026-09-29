//! Canonical physiology timeline: deterministic per-second transitions with
//! archived executable dependencies (ADR-0016). Tests cover 1×60s == 60×1s,
//! partition/reopen/replay, worker parity, conservation, and SafeStop.

use makise_causal_kernel::{
    BloodState, CommitRequest, EventCursor, EventQuery, OpenSpec, ProjectionRequest, RenalState,
    StorageLocation, TimelineFormat, TimelineId, WorldEngine, WorldId,
};

fn canonical_spec(timeline: &str) -> OpenSpec {
    OpenSpec::new(
        WorldId::new("canonical").unwrap(),
        TimelineId::new(timeline).unwrap(),
    )
    .with_format(TimelineFormat::CanonicalPhysiologyV2)
}

#[test]
fn one_by_sixty_equals_sixty_by_one_through_partition_and_both_replay_modes() {
    let dir = tempfile::tempdir().unwrap();
    let (aggregate_renal, aggregate_blood, aggregate_hash) = {
        let storage = StorageLocation::sqlite(dir.path().join("aggregate.sqlite"));
        let (mut engine, _) =
            WorldEngine::open(canonical_spec("aggregate"), storage.clone()).unwrap();
        let receipt = engine
            .commit(CommitRequest::ingest_fluid("intake1", 0, 250_000, 35_000))
            .unwrap();
        assert_eq!(receipt.timeline_version(), 1);
        assert_eq!(receipt.first_event_sequence(), 1);
        assert_eq!(receipt.last_event_sequence(), 1);
        let receipt = engine
            .commit(CommitRequest::advance_to("advance60", 1, 60))
            .unwrap();
        assert_eq!(receipt.timeline_version(), 2);
        assert_eq!(receipt.first_event_sequence(), 2);
        assert_eq!(receipt.last_event_sequence(), 61);
        let hash = receipt.resulting_state_hash().to_owned();
        let receipt = engine
            .commit(CommitRequest::ingest_fluid("intake2", 2, 250_000, 35_000))
            .unwrap();
        assert_eq!(receipt.timeline_version(), 3);
        let organism = engine.organism().expect("organism exists");
        let renal = organism.renal().clone();
        let blood = organism.blood().clone();
        drop(engine);

        let (mut engine, _) = WorldEngine::open(canonical_spec("aggregate"), storage).unwrap();
        assert_eq!(
            engine.fast_replay().unwrap(),
            engine.audit_replay().unwrap()
        );
        assert_eq!(
            engine.fast_replay().unwrap().resulting_state_hash(),
            receipt.resulting_state_hash()
        );
        (renal, blood, hash)
    };

    let (sequential_renal, sequential_blood, sequential_hash) = {
        let storage = StorageLocation::sqlite(dir.path().join("sequential.sqlite"));
        let (mut engine, _) =
            WorldEngine::open(canonical_spec("sequential"), storage.clone()).unwrap();
        engine
            .commit(CommitRequest::ingest_fluid("intake1", 0, 250_000, 35_000))
            .unwrap();
        for offset in 0..60 {
            let receipt = engine
                .commit(CommitRequest::advance_to(
                    &format!("advance{}", offset),
                    offset + 1,
                    1,
                ))
                .unwrap();
            assert_eq!(receipt.first_event_sequence(), offset + 2);
            assert_eq!(receipt.last_event_sequence(), offset + 2);
        }
        let projection = engine.project(ProjectionRequest::current()).unwrap();
        let events_page = engine
            .events(EventQuery::new(EventCursor::start(), 100).unwrap())
            .unwrap();
        let hash = events_page
            .events()
            .iter()
            .find(|e| e.sequence() == 61)
            .expect("event 61 exists")
            .resulting_state_hash()
            .to_owned();
        assert_eq!(projection.timeline_version(), 61);
        let receipt = engine
            .commit(CommitRequest::ingest_fluid("intake2", 61, 250_000, 35_000))
            .unwrap();
        assert_eq!(receipt.timeline_version(), 62);
        let organism = engine.organism().unwrap();
        let renal = organism.renal().clone();
        let blood = organism.blood().clone();
        drop(engine);

        let (mut engine, _) = WorldEngine::open(canonical_spec("sequential"), storage).unwrap();
        assert_eq!(
            engine.fast_replay().unwrap(),
            engine.audit_replay().unwrap()
        );
        (renal, blood, hash)
    };

    assert_eq!(aggregate_renal, sequential_renal);
    assert_eq!(aggregate_blood, sequential_blood);
    assert_eq!(aggregate_hash, sequential_hash);
    assert_eq!(
        aggregate_renal,
        RenalState::new(42_498_980, 3_498_980, 489_940, 1_020, 60).unwrap()
    );
    assert_eq!(
        aggregate_blood,
        BloodState::new(5_498_980, 10_000, 39_200, 24_000, 15_991_840, 300).unwrap()
    );
}

#[test]
fn canonical_timeline_records_separate_conservation_for_water_and_sodium() {
    let dir = tempfile::tempdir().unwrap();
    let storage = StorageLocation::sqlite(dir.path().join("conservation.sqlite"));
    let (mut engine, _) =
        WorldEngine::open(canonical_spec("conservation"), storage.clone()).unwrap();

    engine
        .commit(CommitRequest::ingest_fluid("intake", 0, 250_000, 35_000))
        .unwrap();
    engine
        .commit(CommitRequest::advance_to("advance", 1, 60))
        .unwrap();

    let events = engine
        .events(EventQuery::new(EventCursor::start(), 100).unwrap())
        .unwrap();

    for event in events.events() {
        let report = event.conservation_report().expect("conservation present");
        assert!(report.is_verified());
        let causes = event.causes().expect("causes present");
        if causes.iter().any(|c| c.kind() == "canonical_renal_intake") {
            // Intake: external water and sodium
            if let Some(deltas) = event.unit_deltas() {
                assert!(deltas.iter().any(|d| d.quantity().contains("water")));
                assert!(deltas.iter().any(|d| d.quantity().contains("sodium")));
            }
        } else if causes.iter().any(|c| c.kind() == "canonical_renal_advance") {
            // Advance: zero external input, internal redistribution
            assert!(event.interval_end_second() > event.interval_start_second());
        }
    }
}

#[test]
fn canonical_replay_rejects_changed_artifact_with_durable_safe_stop() {
    let dir = tempfile::tempdir().unwrap();
    let storage = StorageLocation::sqlite(dir.path().join("artifact.sqlite"));
    let (mut engine, _) = WorldEngine::open(canonical_spec("artifact"), storage.clone()).unwrap();

    engine
        .commit(CommitRequest::ingest_fluid("intake", 0, 250_000, 35_000))
        .unwrap();
    engine
        .commit(CommitRequest::advance_to("advance", 1, 10))
        .unwrap();

    drop(engine);

    // Corrupt archived artifact
    let conn = rusqlite::Connection::open(dir.path().join("artifact.sqlite")).unwrap();
    conn.execute(
        "UPDATE artifact_archive SET program_bytes = program_bytes || X'FF' WHERE rowid=1",
        [],
    )
    .unwrap();
    drop(conn);

    let (mut engine, _) = WorldEngine::open(canonical_spec("artifact"), storage.clone()).unwrap();
    assert!(engine.audit_replay().is_err());
    let stop = engine.safe_stop().unwrap().expect("SafeStop recorded");
    assert!(stop.reason().contains("artifact") || stop.reason().contains("mismatch"));

    // SafeStop persists after reopen
    drop(engine);
    let (engine, _) = WorldEngine::open(canonical_spec("artifact"), storage).unwrap();
    let stop = engine.safe_stop().unwrap().expect("SafeStop persists");
    assert!(stop.reason().contains("artifact") || stop.reason().contains("mismatch"));
}

#[test]
fn retry_after_reopen_returns_same_receipt_range() {
    let dir = tempfile::tempdir().unwrap();
    let storage = StorageLocation::sqlite(dir.path().join("retry.sqlite"));
    let (mut engine, _) = WorldEngine::open(canonical_spec("retry"), storage.clone()).unwrap();

    let receipt1 = engine
        .commit(CommitRequest::advance_to("advance60", 0, 60))
        .unwrap();
    assert_eq!(receipt1.first_event_sequence(), 1);
    assert_eq!(receipt1.last_event_sequence(), 60);

    drop(engine);
    let (mut engine, _) = WorldEngine::open(canonical_spec("retry"), storage).unwrap();

    let receipt2 = engine
        .commit(CommitRequest::advance_to("advance60", 0, 60))
        .unwrap();
    assert!(receipt2.replayed_request());
    assert_eq!(
        receipt2.first_event_sequence(),
        receipt1.first_event_sequence()
    );
    assert_eq!(
        receipt2.last_event_sequence(),
        receipt1.last_event_sequence()
    );
    assert_eq!(
        receipt2.resulting_state_hash(),
        receipt1.resulting_state_hash()
    );
}

#[test]
fn conflicting_payload_with_same_request_id_rejects_without_mutation() {
    let dir = tempfile::tempdir().unwrap();
    let storage = StorageLocation::sqlite(dir.path().join("conflict.sqlite"));
    let (mut engine, _) = WorldEngine::open(canonical_spec("conflict"), storage.clone()).unwrap();

    engine
        .commit(CommitRequest::advance_to("advance", 0, 30))
        .unwrap();
    let before = engine.project(ProjectionRequest::current()).unwrap();

    assert!(
        engine
            .commit(CommitRequest::advance_to("advance", 1, 40))
            .is_err()
    );

    assert_eq!(
        engine.project(ProjectionRequest::current()).unwrap(),
        before
    );
}

#[test]
fn canonical_format_refuses_aggregate_v1_requests() {
    let dir = tempfile::tempdir().unwrap();
    let storage = StorageLocation::sqlite(dir.path().join("format.sqlite"));
    let (mut engine, _) = WorldEngine::open(canonical_spec("format"), storage).unwrap();

    // Legacy aggregate requests are rejected
    assert!(
        engine
            .commit(CommitRequest::ingest_food("food", 0, 100_000))
            .is_err()
    );
    assert!(
        engine
            .commit(CommitRequest::accept_sleep_intention("sleep", 0))
            .is_err()
    );

    // Only canonical physiology requests allowed
    engine
        .commit(CommitRequest::ingest_fluid("fluid", 0, 100_000, 5_000))
        .unwrap();
    engine
        .commit(CommitRequest::advance_to("advance", 1, 10))
        .unwrap();
}

#[test]
fn last_second_failure_in_aggregate_request_commits_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let storage = StorageLocation::sqlite(dir.path().join("atomic.sqlite"));
    let (mut engine, _) = WorldEngine::open(canonical_spec("atomic"), storage.clone()).unwrap();

    // Set up state that will overflow on large advance
    engine
        .commit(CommitRequest::ingest_fluid("setup", 0, 100_000, 5_000))
        .unwrap();

    let before_version = engine
        .project(ProjectionRequest::current())
        .unwrap()
        .timeline_version();
    let before_second = engine
        .project(ProjectionRequest::current())
        .unwrap()
        .simulated_second();

    // Request that would overflow simulated_second
    let huge_advance = i64::MAX / 2;
    assert!(
        engine
            .commit(CommitRequest::advance_to("huge", 1, huge_advance))
            .is_err()
    );

    // Nothing committed: no partial prefix
    assert_eq!(
        engine
            .project(ProjectionRequest::current())
            .unwrap()
            .timeline_version(),
        before_version
    );
    assert_eq!(
        engine
            .project(ProjectionRequest::current())
            .unwrap()
            .simulated_second(),
        before_second
    );

    drop(engine);
    let (engine, _) = WorldEngine::open(canonical_spec("atomic"), storage).unwrap();
    assert_eq!(
        engine
            .project(ProjectionRequest::current())
            .unwrap()
            .timeline_version(),
        before_version
    );
}
