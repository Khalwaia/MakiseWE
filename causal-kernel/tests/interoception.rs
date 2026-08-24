use makise_causal_kernel::{
    CommitRequest, InteroceptionObservables, OpenSpec, SleepPhase, StorageLocation, TimelineId,
    WorldEngine, WorldId,
};

fn spec() -> OpenSpec {
    OpenSpec::new(
        WorldId::new("world-alpha").expect("valid"),
        TimelineId::new("timeline-main").expect("valid"),
    )
}

#[test]
fn hunger_projection_decreases_with_chemical_store() {
    let full = InteroceptionObservables::of(
        makise_causal_kernel::INITIAL_CHEMICAL_STORE_UJ,
        makise_causal_kernel::INITIAL_CHEMICAL_STORE_UJ,
        0,
        SleepPhase::Awake,
    );
    let depleted = InteroceptionObservables::of(
        1_000_000_000_000,
        makise_causal_kernel::INITIAL_CHEMICAL_STORE_UJ,
        0,
        SleepPhase::Awake,
    );

    assert_eq!(full.hunger_fraction_permille(), 0);
    assert!(depleted.hunger_fraction_permille() > full.hunger_fraction_permille());
}

#[test]
fn sleep_debt_accumulates_awake_and_clears_asleep() {
    let mut debt_seconds: i64 = 0;
    // Awake for 100 canonical seconds accumulates debt second-per-second.
    for _ in 0..100 {
        debt_seconds = makise_causal_kernel::advance_sleep_debt(debt_seconds, SleepPhase::Awake);
    }
    assert_eq!(debt_seconds, 100);

    // Asleep seconds reduce debt but not below zero.
    for _ in 0..40 {
        debt_seconds = makise_causal_kernel::advance_sleep_debt(debt_seconds, SleepPhase::Asleep);
    }
    assert_eq!(debt_seconds, 60);

    for _ in 0..200 {
        debt_seconds = makise_causal_kernel::advance_sleep_debt(debt_seconds, SleepPhase::Asleep);
    }
    assert_eq!(debt_seconds, 0);
}

#[test]
fn fatigue_grows_with_sleep_debt_and_hunger_with_deficit() {
    let rested = InteroceptionObservables::of(
        makise_causal_kernel::INITIAL_CHEMICAL_STORE_UJ,
        makise_causal_kernel::INITIAL_CHEMICAL_STORE_UJ,
        0,
        SleepPhase::Awake,
    );
    let tired = InteroceptionObservables::of(
        makise_causal_kernel::INITIAL_CHEMICAL_STORE_UJ,
        makise_causal_kernel::INITIAL_CHEMICAL_STORE_UJ,
        3_600,
        SleepPhase::Awake,
    );

    assert!(tired.fatigue_fraction_permille() > rested.fatigue_fraction_permille());
}

#[test]
fn observables_exposed_by_engine_after_advance() {
    let directory = tempfile::tempdir().expect("temp dir");
    let (mut engine, _) = WorldEngine::open(
        spec(),
        StorageLocation::sqlite(directory.path().join("t.sqlite")),
    )
    .expect("open");
    engine
        .commit(CommitRequest::advance_to("adv", 0, 10))
        .expect("advance");

    let observables = engine.interoception().expect("organism must exist");
    assert!(observables.chemical_store_uj() < makise_causal_kernel::INITIAL_CHEMICAL_STORE_UJ);
    assert_eq!(observables.sleep_debt_seconds(), 10);
}
