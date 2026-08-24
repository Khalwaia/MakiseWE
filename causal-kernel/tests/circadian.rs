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
fn sleep_phase_survives_reopen() {
    let directory = tempfile::tempdir().expect("temp dir");
    let path = directory.path().join("t.sqlite");
    {
        let (mut engine, _) =
            WorldEngine::open(spec(), StorageLocation::sqlite(&path)).expect("create");
        engine
            .commit(CommitRequest::accept_sleep_intention("int-1", 0))
            .expect("sleep intention");
    }
    let (engine, _) = WorldEngine::open(spec(), StorageLocation::sqlite(&path)).expect("reopen");

    assert_eq!(engine.sleep_phase(), SleepPhase::Asleep);
}
