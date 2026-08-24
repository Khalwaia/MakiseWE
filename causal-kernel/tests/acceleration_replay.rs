use makise_causal_kernel::{
    CommitRequest, OpenSpec, StorageLocation, TimelineId, WorldEngine, WorldId,
};

fn spec(name: &str) -> OpenSpec {
    OpenSpec::new(
        WorldId::new(format!("{name}-world")).expect("valid"),
        TimelineId::new(format!("{name}-timeline")).expect("valid"),
    )
}

fn organism_snapshot(engine: &WorldEngine) -> (i64, i64, i64, i64) {
    let organism = engine.organism().expect("organism initialized");
    (
        organism.chemical_store_uj(),
        organism.core_internal_energy_uj(),
        organism.ambient_internal_energy_uj(),
        engine.simulated_second(),
    )
}

#[test]
fn accelerated_second_matches_one_to_one_execution() {
    let dir = tempfile::tempdir().expect("temp");

    let one_to_one_path = dir.path().join("one-to-one.sqlite");
    let (mut one_to_one, _) =
        WorldEngine::open(spec("slow"), StorageLocation::sqlite(one_to_one_path)).expect("open");
    for second in 0..120 {
        one_to_one
            .commit(CommitRequest::advance_to(
                &format!("second-{second}"),
                second,
                1,
            ))
            .expect("advance 1s");
    }

    let accelerated_path = dir.path().join("accelerated.sqlite");
    let (mut accelerated, _) =
        WorldEngine::open(spec("fast"), StorageLocation::sqlite(accelerated_path)).expect("open");
    accelerated
        .commit(CommitRequest::advance_to("accelerate-120", 0, 120))
        .expect("accelerated advance");

    assert_eq!(
        organism_snapshot(&one_to_one),
        organism_snapshot(&accelerated),
        "acceleration must execute the same canonical per-second mechanism"
    );
}

#[test]
fn restart_inside_long_interval_matches_uninterrupted_interval() {
    let dir = tempfile::tempdir().expect("temp");
    let uninterrupted_path = dir.path().join("uninterrupted.sqlite");
    let (mut uninterrupted, _) =
        WorldEngine::open(spec("whole"), StorageLocation::sqlite(uninterrupted_path))
            .expect("open");
    uninterrupted
        .commit(CommitRequest::advance_to("whole", 0, 180))
        .expect("whole interval");

    let restarted_path = dir.path().join("restarted.sqlite");
    let (mut restarted, _) = WorldEngine::open(
        spec("split"),
        StorageLocation::sqlite(restarted_path.clone()),
    )
    .expect("open");
    restarted
        .commit(CommitRequest::advance_to("first", 0, 70))
        .expect("prefix interval");
    drop(restarted);
    let (mut restarted, _) =
        WorldEngine::open(spec("split"), StorageLocation::sqlite(restarted_path)).expect("reopen");
    restarted
        .commit(CommitRequest::advance_to("rest", 1, 110))
        .expect("suffix interval");

    let expected = organism_snapshot(&uninterrupted);
    let actual = organism_snapshot(&restarted);
    assert_eq!(expected.3, actual.3);
    assert_eq!(
        expected.2, actual.2,
        "thermal state must match after restart"
    );
    assert_eq!(expected.0, actual.0);
}
