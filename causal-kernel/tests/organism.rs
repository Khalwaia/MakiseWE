use makise_causal_kernel::{
    CommitRequest, OpenSpec, OrganismState, StorageLocation, TimelineId, WorldId,
};

fn spec() -> OpenSpec {
    OpenSpec::new(
        WorldId::new("world-alpha").expect("valid"),
        TimelineId::new("timeline-main").expect("valid"),
    )
}

#[test]
fn metabolism_burns_chemical_exactly_into_thermal_energy() {
    let mut organism = OrganismState::new(1_000_000_000_000, 20_000_000_000_000);
    let before = organism.total_accounted_uj();

    organism
        .apply_metabolism(500_000_000)
        .expect("in-store metabolism must succeed");

    assert_eq!(organism.total_accounted_uj(), before);
}

#[test]
fn metabolism_rejects_overdraft_without_partial_application() {
    let mut organism = OrganismState::new(100_000, 20_000_000_000_000);

    let error = organism
        .apply_metabolism(200_000)
        .expect_err("overdraft must be typed failure");

    assert_eq!(organism.chemical_store_uj(), 100_000);
    assert!(matches!(
        error,
        makise_causal_kernel::OrganismError::ChemicalOverdraft
    ));
}

#[test]
fn reopen_restores_organism_energies_exactly() {
    let directory = tempfile::tempdir().expect("temp dir");
    let path = directory.path().join("t.sqlite");

    {
        let (mut engine, _) =
            makise_causal_kernel::WorldEngine::open(spec(), StorageLocation::sqlite(&path))
                .expect("create");
        engine
            .commit(CommitRequest::advance_to("req-1", 0, 3))
            .expect("advance to heat reservoirs");
        drop(engine);
    }

    let (engine, _) =
        makise_causal_kernel::WorldEngine::open(spec(), StorageLocation::sqlite(&path))
            .expect("reopen");

    // Reference: uninterrupted run in a second timeline reaches the same state.
    let reference_directory = tempfile::tempdir().expect("temp dir");
    let (mut reference, _) = makise_causal_kernel::WorldEngine::open(
        spec(),
        StorageLocation::sqlite(reference_directory.path().join("t.sqlite")),
    )
    .expect("reference");
    reference
        .commit(CommitRequest::advance_to("req-1", 0, 3))
        .expect("reference advance");

    assert_eq!(
        engine.organism().map(OrganismState::total_accounted_uj),
        reference.organism().map(OrganismState::total_accounted_uj),
    );
}
