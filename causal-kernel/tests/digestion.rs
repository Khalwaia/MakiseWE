use makise_causal_kernel::{
    CommitRequest, OpenSpec, StorageLocation, TimelineId, WorldEngine, WorldId,
};

fn spec() -> OpenSpec {
    OpenSpec::new(
        WorldId::new("world-alpha").expect("valid"),
        TimelineId::new("timeline-main").expect("valid"),
    )
}

#[test]
fn ingestion_fills_digestion_buffer_without_immediate_store_credit() {
    let directory = tempfile::tempdir().expect("temp dir");
    let (mut engine, _) = WorldEngine::open(
        spec(),
        StorageLocation::sqlite(directory.path().join("t.sqlite")),
    )
    .expect("open");
    engine
        .commit(CommitRequest::advance_to("warmup", 0, 1))
        .expect("initialize organism");

    let before = engine.organism().expect("organism").chemical_store_uj();
    engine
        .commit(CommitRequest::ingest_food("meal-1", 1, 2_000_000))
        .expect("ingestion");

    let organism = engine.organism().expect("organism");
    assert_eq!(
        organism.digestion_buffer_uj(),
        2_000_000,
        "ingested energy must enter the digestive buffer, not the store"
    );
    assert_eq!(
        organism.chemical_store_uj(),
        before,
        "absorption must not be instantaneous"
    );
}

#[test]
fn absorption_transfers_declared_flux_per_canonical_second() {
    let directory = tempfile::tempdir().expect("temp dir");
    let path = directory.path().join("t.sqlite");
    let (mut engine, _) = WorldEngine::open(spec(), StorageLocation::sqlite(&path)).expect("open");

    // Spend an hour of waking metabolism so the chemical store has
    // headroom below its declared capacity for the buffered meal.
    engine
        .commit(CommitRequest::advance_to("spend", 0, 3_600))
        .expect("hour of waking metabolism");

    // A large buffered meal so the flux runs at full declared rate for
    // every advanced second despite waking metabolism drawing the store.
    let meal_uj = 10 * makise_causal_kernel::ABSORPTION_RATE_UJ_PER_SECOND;
    engine
        .commit(CommitRequest::ingest_food("meal", 1, meal_uj))
        .expect("ingestion");

    let buffered_before_absorption = engine.organism().unwrap().digestion_buffer_uj();
    engine
        .commit(CommitRequest::advance_to("absorb", 2, 4))
        .expect("four canonical seconds");

    let organism = engine.organism().unwrap();
    assert_eq!(
        organism.digestion_buffer_uj(),
        buffered_before_absorption - 4 * makise_causal_kernel::ABSORPTION_RATE_UJ_PER_SECOND,
        "each canonical second must move exactly the declared flux out of the buffer"
    );
}

#[test]
fn buffer_and_store_sum_changes_only_by_declared_flux_and_metabolism() {
    let directory = tempfile::tempdir().expect("temp dir");
    let path = directory.path().join("t.sqlite");
    let (mut engine, _) = WorldEngine::open(spec(), StorageLocation::sqlite(&path)).expect("open");

    engine
        .commit(CommitRequest::advance_to("warmup", 0, 3_600))
        .expect("create metabolic deficit");
    let meal_uj = 5_000_000;
    engine
        .commit(CommitRequest::ingest_food("meal", 1, meal_uj))
        .expect("ingestion");
    let sum_before = engine.organism().unwrap().chemical_store_uj()
        + engine.organism().unwrap().digestion_buffer_uj();

    engine
        .commit(CommitRequest::advance_to("drain", 2, 30))
        .expect("thirty seconds of absorption and metabolism");

    // Buffer drains by the declared flux, but a small meal fully
    // absorbs within a single canonical second (meal < rate); the store
    // changes only by that absorbed flux plus waking metabolism.
    let organism = engine.organism().unwrap();
    let absorbed_total = meal_uj.min(makise_causal_kernel::ABSORPTION_RATE_UJ_PER_SECOND);
    let expected_burn: i64 = (3_600..3_630)
        .map(makise_causal_kernel::awake_metabolism_for_second)
        .sum();
    assert_eq!(
        organism.digestion_buffer_uj(),
        meal_uj - absorbed_total,
        "buffer must retain only the unabsorbed remainder"
    );
    assert_eq!(
        organism.chemical_store_uj(),
        sum_before - organism.digestion_buffer_uj() - expected_burn,
        "store must change only by absorbed flux minus burned energy"
    );
}

#[test]
fn ingestion_beyond_chemical_capacity_is_typed_rejection() {
    let directory = tempfile::tempdir().expect("temp dir");
    let (mut engine, _) = WorldEngine::open(
        spec(),
        StorageLocation::sqlite(directory.path().join("t.sqlite")),
    )
    .expect("open");
    engine
        .commit(CommitRequest::advance_to("warmup", 0, 1))
        .expect("initialize organism");

    let overflow = makise_causal_kernel::INITIAL_CHEMICAL_STORE_UJ + 1;
    let error = engine
        .commit(CommitRequest::ingest_food("gluttony", 1, overflow))
        .expect_err("meal beyond chemical capacity must be rejected");

    assert!(matches!(
        error,
        makise_causal_kernel::CommitError::DigestiveCapacityExceeded
    ));
    assert_eq!(
        engine.organism().unwrap().digestion_buffer_uj(),
        0,
        "rejected ingestion must not mutate state"
    );
}

#[test]
fn ingestion_rejects_nonpositive_amount() {
    let directory = tempfile::tempdir().expect("temp dir");
    let (mut engine, _) = WorldEngine::open(
        spec(),
        StorageLocation::sqlite(directory.path().join("t.sqlite")),
    )
    .expect("open");

    let error = engine
        .commit(CommitRequest::ingest_food("bad", 0, 0))
        .expect_err("nonpositive ingestion must be rejected");

    assert!(matches!(
        error,
        makise_causal_kernel::CommitError::InvalidIngestion
    ));
}

#[test]
fn awake_night_demand_is_lower_than_awake_day_demand() {
    // Canonical day starts at simulated second 0; night is seconds 0..=21600.
    let directory_night = tempfile::tempdir().expect("temp dir");
    let (mut night, _) = WorldEngine::open(
        spec(),
        StorageLocation::sqlite(directory_night.path().join("t.sqlite")),
    )
    .expect("night");
    night
        .commit(CommitRequest::advance_to("n", 0, 10))
        .expect("night advance");

    let night_burn = makise_causal_kernel::awake_metabolism_for_second(100);
    let day_burn = makise_causal_kernel::awake_metabolism_for_second(50_000);
    assert!(night_burn < day_burn);
    let _ = night;
}

#[test]
fn ten_seconds_of_advance_use_circadian_modulated_demand() {
    let directory = tempfile::tempdir().expect("temp dir");
    let path = directory.path().join("t.sqlite");
    let (mut engine, _) = WorldEngine::open(spec(), StorageLocation::sqlite(&path)).expect("open");
    engine
        .commit(CommitRequest::advance_to("adv", 0, 5))
        .expect("advance");

    let expected_total: i64 = (0..5)
        .map(makise_causal_kernel::awake_metabolism_for_second)
        .sum();
    let store = engine.organism().expect("organism").chemical_store_uj();
    assert_eq!(
        store,
        makise_causal_kernel::INITIAL_CHEMICAL_STORE_UJ - expected_total
    );
}
