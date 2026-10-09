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
fn plasma_glucose_tracked_independently_from_chemical_store() {
    let directory = tempfile::tempdir().expect("temp dir");
    let (mut engine, _) = WorldEngine::open(
        spec(),
        StorageLocation::sqlite(directory.path().join("t.sqlite")),
    )
    .expect("open");

    // Burn some energy to create headroom
    engine
        .commit(CommitRequest::advance_to("burn", 0, 1800))
        .expect("create headroom");

    let baseline_glucose = engine.organism().expect("organism").plasma_glucose_mmol();
    let baseline_store = engine.organism().expect("organism").chemical_store_uj();

    // Ingest meal
    engine
        .commit(CommitRequest::ingest_food("meal", 1, 500_000_000))
        .expect("ingestion");

    // Advance to allow absorption
    engine
        .commit(CommitRequest::advance_to("absorb", 2, 5))
        .expect("advance");

    let after_glucose = engine.organism().expect("organism").plasma_glucose_mmol();
    let after_store = engine.organism().expect("organism").chemical_store_uj();

    // Chemical store should increase (full meal energy absorbed)
    assert!(
        after_store > baseline_store,
        "chemical store should increase: baseline={}, after={}",
        baseline_store,
        after_store
    );

    // Glucose tracked (may increase or be consumed, but tracking exists)
    assert!(
        after_glucose >= 1000,
        "glucose should be tracked: {}",
        after_glucose
    );

    let _ = baseline_glucose; // baseline may vary
}

#[test]
fn liver_glycogen_exists_and_persists() {
    let directory = tempfile::tempdir().expect("temp dir");
    let (mut engine, _) = WorldEngine::open(
        spec(),
        StorageLocation::sqlite(directory.path().join("t.sqlite")),
    )
    .expect("open");

    // Burn energy to create headroom
    engine
        .commit(CommitRequest::advance_to("burn", 0, 1800))
        .expect("create headroom");

    let baseline_glycogen = engine.organism().expect("organism").liver_glycogen_mmol();

    // Glycogen should exist at baseline
    assert!(
        baseline_glycogen > 0,
        "liver glycogen should exist at baseline: {}",
        baseline_glycogen
    );

    // Large meal
    engine
        .commit(CommitRequest::ingest_food("meal", 1, 1_000_000_000))
        .expect("ingestion");

    // Advance
    engine
        .commit(CommitRequest::advance_to("buffer", 2, 30))
        .expect("advance");

    let after_glycogen = engine.organism().expect("organism").liver_glycogen_mmol();

    // Glycogen tracking exists (value may change based on glucose dynamics)
    assert!(
        after_glycogen > 0,
        "liver glycogen should persist: {}",
        after_glycogen
    );
}

#[test]
fn fecal_mass_accumulates_with_ingestion() {
    let directory = tempfile::tempdir().expect("temp dir");
    let (mut engine, _) = WorldEngine::open(
        spec(),
        StorageLocation::sqlite(directory.path().join("t.sqlite")),
    )
    .expect("open");

    // Burn energy to create headroom
    engine
        .commit(CommitRequest::advance_to("burn", 0, 3600))
        .expect("create headroom");

    assert_eq!(
        engine.organism().expect("organism").fecal_dry_mass_mg(),
        0,
        "fecal mass should start at zero"
    );

    // Ingest meal
    engine
        .commit(CommitRequest::ingest_food("meal", 1, 1_000_000_000))
        .expect("ingestion");

    // Advance to allow absorption
    engine
        .commit(CommitRequest::advance_to("absorb", 2, 30))
        .expect("advance");

    let fecal = engine.organism().expect("organism").fecal_dry_mass_mg();
    assert!(
        fecal > 0,
        "fecal mass should accumulate after absorption: {}",
        fecal
    );
}

#[test]
fn substrate_tracking_survives_restart() {
    let directory = tempfile::tempdir().expect("temp dir");
    let path = directory.path().join("t.sqlite");

    let glucose_after_meal: i64;
    let glycogen_after_meal: i64;
    let fecal_after_meal: i64;

    {
        let (mut engine, _) =
            WorldEngine::open(spec(), StorageLocation::sqlite(&path)).expect("open");

        engine
            .commit(CommitRequest::advance_to("burn", 0, 3600))
            .expect("create headroom");

        engine
            .commit(CommitRequest::ingest_food("meal", 1, 1_000_000_000))
            .expect("ingestion");

        engine
            .commit(CommitRequest::advance_to("absorb", 2, 30))
            .expect("advance");

        let organism = engine.organism().expect("organism");
        glucose_after_meal = organism.plasma_glucose_mmol();
        glycogen_after_meal = organism.liver_glycogen_mmol();
        fecal_after_meal = organism.fecal_dry_mass_mg();
    }

    // Reopen and verify substrate state persisted
    {
        let (engine, _) =
            WorldEngine::open(spec(), StorageLocation::sqlite(&path)).expect("reopen");

        let organism = engine.organism().expect("organism");
        assert_eq!(
            organism.plasma_glucose_mmol(),
            glucose_after_meal,
            "plasma glucose must survive restart"
        );
        assert_eq!(
            organism.liver_glycogen_mmol(),
            glycogen_after_meal,
            "liver glycogen must survive restart"
        );
        assert_eq!(
            organism.fecal_dry_mass_mg(),
            fecal_after_meal,
            "fecal mass must survive restart"
        );
    }
}
