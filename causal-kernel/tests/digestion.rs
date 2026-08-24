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
fn ingestion_adds_chemical_energy_exactly() {
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

    let after = engine.organism().expect("organism").chemical_store_uj();
    assert_eq!(after - before, 2_000_000);
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
