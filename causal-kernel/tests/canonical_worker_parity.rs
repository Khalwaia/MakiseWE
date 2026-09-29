//! Worker parity for canonical physiology: independent engines running the same
//! stimulus sequence must produce identical canonical transitions, hashes, and
//! conservation evidence (ADR-0016, INVARIANTS §32-34).

use makise_causal_kernel::{
    CommitRequest, EventCursor, EventQuery, OpenSpec, ProjectionRequest, StorageLocation,
    TimelineFormat, TimelineId, WorldEngine, WorldId,
};

fn canonical_spec(timeline: &str) -> OpenSpec {
    OpenSpec::new(
        WorldId::new("worker").unwrap(),
        TimelineId::new(timeline).unwrap(),
    )
    .with_format(TimelineFormat::CanonicalPhysiologyV2)
}

#[test]
fn independent_engines_produce_identical_canonical_transitions_and_state_hashes() {
    let dir = tempfile::tempdir().unwrap();

    let worker1_state = {
        let storage = StorageLocation::sqlite(dir.path().join("worker1.sqlite"));
        let (mut engine, _) = WorldEngine::open(canonical_spec("w1"), storage.clone()).unwrap();

        engine
            .commit(CommitRequest::ingest_fluid("intake1", 0, 250_000, 35_000))
            .unwrap();
        engine
            .commit(CommitRequest::advance_to("advance30", 1, 30))
            .unwrap();
        engine
            .commit(CommitRequest::ingest_fluid("intake2", 2, 100_000, 10_000))
            .unwrap();
        engine
            .commit(CommitRequest::advance_to("advance20", 3, 20))
            .unwrap();

        let projection = engine.project(ProjectionRequest::current()).unwrap();
        let events = engine
            .events(EventQuery::new(EventCursor::start(), 200).unwrap())
            .unwrap();

        (
            projection,
            events.events().to_vec(),
            engine.organism().unwrap().clone(),
        )
    };

    let worker2_state = {
        let storage = StorageLocation::sqlite(dir.path().join("worker2.sqlite"));
        let (mut engine, _) = WorldEngine::open(canonical_spec("w2"), storage).unwrap();

        engine
            .commit(CommitRequest::ingest_fluid("intake1", 0, 250_000, 35_000))
            .unwrap();
        engine
            .commit(CommitRequest::advance_to("advance30", 1, 30))
            .unwrap();
        engine
            .commit(CommitRequest::ingest_fluid("intake2", 2, 100_000, 10_000))
            .unwrap();
        engine
            .commit(CommitRequest::advance_to("advance20", 3, 20))
            .unwrap();

        let projection = engine.project(ProjectionRequest::current()).unwrap();
        let events = engine
            .events(EventQuery::new(EventCursor::start(), 200).unwrap())
            .unwrap();

        (
            projection,
            events.events().to_vec(),
            engine.organism().unwrap().clone(),
        )
    };

    // Projections match
    assert_eq!(
        worker1_state.0.timeline_version(),
        worker2_state.0.timeline_version()
    );
    assert_eq!(
        worker1_state.0.simulated_second(),
        worker2_state.0.simulated_second()
    );

    // Event count matches
    assert_eq!(worker1_state.1.len(), worker2_state.1.len());

    // Every transition matches: interval, hashes, deltas, conservation, artifacts
    for (e1, e2) in worker1_state.1.iter().zip(worker2_state.1.iter()) {
        assert_eq!(e1.sequence(), e2.sequence());
        assert_eq!(e1.interval_start_second(), e2.interval_start_second());
        assert_eq!(e1.interval_end_second(), e2.interval_end_second());
        assert_eq!(e1.previous_state_hash(), e2.previous_state_hash());
        assert_eq!(e1.resulting_state_hash(), e2.resulting_state_hash());
        assert_eq!(e1.state_hash_version(), e2.state_hash_version());

        let deltas1 = e1.unit_deltas().expect("deltas present");
        let deltas2 = e2.unit_deltas().expect("deltas present");
        assert_eq!(deltas1.len(), deltas2.len());
        for (d1, d2) in deltas1.iter().zip(deltas2.iter()) {
            assert_eq!(d1.quantity(), d2.quantity());
            assert_eq!(d1.unit(), d2.unit());
            assert_eq!(d1.before(), d2.before());
            assert_eq!(d1.after(), d2.after());
        }

        assert_eq!(e1.artifact_digests(), e2.artifact_digests());
        assert_eq!(e1.causes(), e2.causes());
    }

    // Organism state identical
    assert_eq!(worker1_state.2, worker2_state.2);
}

#[test]
fn worker_parity_holds_across_partition_boundaries() {
    let dir = tempfile::tempdir().unwrap();

    let single_session = {
        let storage = StorageLocation::sqlite(dir.path().join("single.sqlite"));
        let (mut engine, _) = WorldEngine::open(canonical_spec("single"), storage.clone()).unwrap();

        engine
            .commit(CommitRequest::ingest_fluid("intake", 0, 200_000, 30_000))
            .unwrap();
        engine
            .commit(CommitRequest::advance_to("advance", 1, 40))
            .unwrap();

        let events = engine
            .events(EventQuery::new(EventCursor::start(), 200).unwrap())
            .unwrap();
        events.events().to_vec()
    };

    let multi_partition = {
        let storage = StorageLocation::sqlite(dir.path().join("multi.sqlite"));
        let (mut engine, _) = WorldEngine::open(canonical_spec("multi"), storage.clone()).unwrap();

        engine
            .commit(CommitRequest::ingest_fluid("intake", 0, 200_000, 30_000))
            .unwrap();

        drop(engine);
        let (mut engine, _) = WorldEngine::open(canonical_spec("multi"), storage.clone()).unwrap();

        engine
            .commit(CommitRequest::advance_to("advance", 1, 40))
            .unwrap();

        drop(engine);
        let (engine, _) = WorldEngine::open(canonical_spec("multi"), storage).unwrap();

        let events = engine
            .events(EventQuery::new(EventCursor::start(), 200).unwrap())
            .unwrap();
        events.events().to_vec()
    };

    assert_eq!(single_session.len(), multi_partition.len());

    for (s, m) in single_session.iter().zip(multi_partition.iter()) {
        assert_eq!(s.interval_start_second(), m.interval_start_second());
        assert_eq!(s.interval_end_second(), m.interval_end_second());
        assert_eq!(s.resulting_state_hash(), m.resulting_state_hash());
        assert_eq!(s.artifact_digests(), m.artifact_digests());
        assert_eq!(s.unit_deltas(), m.unit_deltas());
    }
}

#[test]
fn fast_and_audit_replay_agree_on_mixed_intake_and_advance_sequence() {
    let dir = tempfile::tempdir().unwrap();
    let storage = StorageLocation::sqlite(dir.path().join("mixed.sqlite"));
    let (mut engine, _) = WorldEngine::open(canonical_spec("mixed"), storage.clone()).unwrap();

    engine
        .commit(CommitRequest::ingest_fluid("i1", 0, 100_000, 15_000))
        .unwrap();
    engine
        .commit(CommitRequest::advance_to("a1", 1, 10))
        .unwrap();
    engine
        .commit(CommitRequest::ingest_fluid("i2", 2, 50_000, 5_000))
        .unwrap();
    engine
        .commit(CommitRequest::advance_to("a2", 3, 20))
        .unwrap();
    engine
        .commit(CommitRequest::ingest_fluid("i3", 4, 150_000, 20_000))
        .unwrap();
    engine
        .commit(CommitRequest::advance_to("a3", 5, 15))
        .unwrap();

    let fast = engine.fast_replay().unwrap();
    let audit = engine.audit_replay().unwrap();

    assert_eq!(fast.timeline_version(), audit.timeline_version());
    assert_eq!(fast.resulting_state_hash(), audit.resulting_state_hash());

    drop(engine);
    let (mut engine, _) = WorldEngine::open(canonical_spec("mixed"), storage).unwrap();

    let fast2 = engine.fast_replay().unwrap();
    let audit2 = engine.audit_replay().unwrap();

    assert_eq!(fast.resulting_state_hash(), fast2.resulting_state_hash());
    assert_eq!(audit.resulting_state_hash(), audit2.resulting_state_hash());
}

#[test]
fn determinism_holds_for_zero_advance_intake_only_sequence() {
    let dir = tempfile::tempdir().unwrap();

    let run1 = {
        let storage = StorageLocation::sqlite(dir.path().join("run1.sqlite"));
        let (mut engine, _) = WorldEngine::open(canonical_spec("r1"), storage).unwrap();

        engine
            .commit(CommitRequest::ingest_fluid("i1", 0, 100_000, 10_000))
            .unwrap();
        engine
            .commit(CommitRequest::ingest_fluid("i2", 1, 200_000, 20_000))
            .unwrap();
        engine
            .commit(CommitRequest::ingest_fluid("i3", 2, 50_000, 5_000))
            .unwrap();

        engine.organism().unwrap().clone()
    };

    let run2 = {
        let storage = StorageLocation::sqlite(dir.path().join("run2.sqlite"));
        let (mut engine, _) = WorldEngine::open(canonical_spec("r2"), storage).unwrap();

        engine
            .commit(CommitRequest::ingest_fluid("i1", 0, 100_000, 10_000))
            .unwrap();
        engine
            .commit(CommitRequest::ingest_fluid("i2", 1, 200_000, 20_000))
            .unwrap();
        engine
            .commit(CommitRequest::ingest_fluid("i3", 2, 50_000, 5_000))
            .unwrap();

        engine.organism().unwrap().clone()
    };

    assert_eq!(run1, run2);
}
