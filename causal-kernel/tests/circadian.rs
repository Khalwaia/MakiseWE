use makise_causal_kernel::{
    CommitRequest, OpenSpec, SleepPhase, StorageLocation, TimelineId, WorldEngine, WorldId,
};

fn spec() -> OpenSpec {
    OpenSpec::new(
        WorldId::new("world-alpha").expect("valid"),
        TimelineId::new("timeline-main").expect("valid"),
    )
}

#[test]
fn metabolic_demand_is_higher_when_awake_than_asleep() {
    let awake = makise_causal_kernel::metabolic_demand_uj_per_second(SleepPhase::Awake);
    let asleep = makise_causal_kernel::metabolic_demand_uj_per_second(SleepPhase::Asleep);

    assert!(awake > asleep);
}

#[test]
fn sleep_transition_requires_accepted_intention() {
    let directory = tempfile::tempdir().expect("temp dir");
    let (mut engine, _) = WorldEngine::open(
        spec(),
        StorageLocation::sqlite(directory.path().join("t.sqlite")),
    )
    .expect("open");

    let error = engine
        .request_sleep_without_intention()
        .expect_err("sleep without accepted intention must be rejected");

    assert!(matches!(
        error,
        makise_causal_kernel::CommitError::SleepIntentionRequired
    ));
}

#[test]
fn accepted_sleep_intention_does_not_immediately_change_phase() {
    let directory = tempfile::tempdir().expect("temp dir");
    let (mut engine, _) = WorldEngine::open(
        spec(),
        StorageLocation::sqlite(directory.path().join("t.sqlite")),
    )
    .expect("open");

    engine
        .commit(CommitRequest::accept_sleep_intention("int-1", 0))
        .expect("accepted intention");

    assert_eq!(
        engine.sleep_phase(),
        SleepPhase::Awake,
        "an accepted intention creates a condition, not an outcome"
    );
}

#[test]
fn advance_applies_sleep_metabolism_after_accepted_intention() {
    let directory = tempfile::tempdir().expect("temp dir");
    let path = directory.path().join("t.sqlite");
    let (mut engine, _) = WorldEngine::open(spec(), StorageLocation::sqlite(&path)).expect("open");
    engine
        .commit(CommitRequest::accept_sleep_intention("int-1", 0))
        .expect("accepted intention");
    engine
        .commit(CommitRequest::advance_to("adv-1", 1, 10))
        .expect("advance while asleep");

    assert_eq!(engine.sleep_phase(), SleepPhase::Asleep);

    // Reference: same 10 seconds awake burn strictly more chemical energy.
    let reference_directory = tempfile::tempdir().expect("temp dir");
    let (mut reference, _) = WorldEngine::open(
        spec(),
        StorageLocation::sqlite(reference_directory.path().join("t.sqlite")),
    )
    .expect("reference");
    reference
        .commit(CommitRequest::advance_to("adv-ref", 0, 10))
        .expect("reference advance");

    let asleep_store = engine.organism().expect("organism").chemical_store_uj();
    let awake_store = reference.organism().expect("organism").chemical_store_uj();
    assert!(asleep_store > awake_store);
}

#[test]
fn accepted_intention_and_onset_survive_reopen() {
    let directory = tempfile::tempdir().expect("temp dir");
    let path = directory.path().join("t.sqlite");
    {
        let (mut engine, _) =
            WorldEngine::open(spec(), StorageLocation::sqlite(&path)).expect("create");
        // Canonical second 0 is inside the 22:00-06:00 night window, so an
        // accepted intention leads to onset during the next advance.
        engine
            .commit(CommitRequest::accept_sleep_intention("int-1", 0))
            .expect("sleep intention");
        engine
            .commit(CommitRequest::advance_to("adv-1", 1, 5))
            .expect("advance into night window");
        assert_eq!(engine.sleep_phase(), SleepPhase::Asleep);
    }
    let (engine, _) = WorldEngine::open(spec(), StorageLocation::sqlite(&path)).expect("reopen");

    assert_eq!(engine.sleep_phase(), SleepPhase::Asleep);
}

#[test]
fn daytime_intention_without_debt_keeps_organism_awake() {
    let directory = tempfile::tempdir().expect("temp dir");
    let path = directory.path().join("t.sqlite");
    let (mut engine, _) = WorldEngine::open(spec(), StorageLocation::sqlite(&path)).expect("open");

    // Advance past the night window into the day; debt stays below the
    // onset threshold.
    engine
        .commit(CommitRequest::advance_to("morning", 0, 25_000))
        .expect("advance into daytime");

    engine
        .commit(CommitRequest::accept_sleep_intention("nap-attempt", 1))
        .expect("accepted intention");

    engine
        .commit(CommitRequest::advance_to("afternoon", 2, 3_600))
        .expect("advance during daytime");

    assert_eq!(
        engine.sleep_phase(),
        SleepPhase::Awake,
        "daytime nap must not trigger without sufficient sleep debt"
    );
}

#[test]
fn full_sleep_cycle_falls_asleep_and_wakes_in_morning_window() {
    let directory = tempfile::tempdir().expect("temp dir");
    let path = directory.path().join("t.sqlite");
    let (mut engine, _) = WorldEngine::open(spec(), StorageLocation::sqlite(&path)).expect("open");

    engine
        .commit(CommitRequest::advance_to("to-evening", 0, 79_200))
        .expect("advance to 22:00");
    // A day of waking metabolism burns most of the glycogen-scale store;
    // a solid dinner funds the overnight fast.
    engine
        .commit(CommitRequest::ingest_food("dinner", 1, 3_000_000_000_000))
        .expect("dinner");
    engine
        .commit(CommitRequest::accept_sleep_intention("bedtime", 2))
        .expect("accepted bedtime intention");

    // Inside the 22:00-06:00 night window the accepted intention leads
    // to physiological sleep onset during the next advance.
    engine
        .commit(CommitRequest::advance_to("night", 3, 7_200))
        .expect("advance through onset");
    assert_eq!(engine.sleep_phase(), SleepPhase::Asleep);

    // Debt accumulated over ~22 awake hours clears in about half that
    // time at the declared recovery rate; waking additionally requires
    // the morning window, so advancing far enough crosses both
    // conditions and the organism wakes on its own.
    engine
        .commit(CommitRequest::advance_to("until-morning", 4, 40_000))
        .expect("advance through the night");

    assert_eq!(
        engine.sleep_phase(),
        SleepPhase::Awake,
        "cleared recovery debt inside the morning window must wake the organism"
    );

    let interoception = engine.interoception().expect("interoception");
    assert!(
        interoception.sleep_debt_seconds() >= 0,
        "sleep debt is a physical counter and never negative"
    );
}
