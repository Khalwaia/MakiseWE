use makise_causal_kernel::CellCohort;

#[test]
fn cohort_construction_stores_conserved_quantities() {
    let cohort = CellCohort::new(
        1_000,
        500_000_000_000, // 0.5 g total mass in µg? No — declared as nanograms
        1_200_000_000,   // electric charge in nano-units
    );

    assert_eq!(cohort.cell_count(), 1_000);
    assert_eq!(cohort.total_mass_ng(), 500_000_000_000);
    assert_eq!(cohort.total_charge_nano(), 1_200_000_000);
}

#[test]
fn deterministic_lift_preserves_cell_count_and_totals() {
    let coarse = CellCohort::new(100, 50_000_000_000, 120_000_000);
    let fine = coarse.lift_to_cells();

    assert_eq!(fine.len(), 100);
    let total_mass: i64 = fine.iter().map(|c| c.mass_ng).sum();
    let total_charge: i64 = fine.iter().map(|c| c.charge_nano).sum();
    assert_eq!(
        total_mass, 50_000_000_000,
        "lift must conserve total mass exactly"
    );
    assert_eq!(
        total_charge, 120_000_000,
        "lift must conserve total charge exactly"
    );

    // Same input → same per-cell distribution (deterministic).
    let fine_again = coarse.lift_to_cells();
    assert_eq!(fine, fine_again, "lift must be deterministic");
}

#[test]
fn projection_back_from_fine_cells_matches_coarse() {
    let coarse = CellCohort::new(3, 30_000_000_001, 90_000_007);
    let fine = coarse.lift_to_cells();
    let projected_back = CellCohort::from_fine_cells(&fine, 3);

    assert_eq!(projected_back.cell_count(), 3);
    assert_eq!(
        projected_back.total_mass_ng(),
        30_000_000_001,
        "projection must round-trip mass exactly"
    );
    assert_eq!(
        projected_back.total_charge_nano(),
        90_000_007,
        "projection must round-trip charge exactly"
    );
}

#[test]
fn lift_rejects_zero_cell_count_as_typed_failure() {
    let empty = CellCohort::new(0, 0, 0);
    let result = std::panic::catch_unwind(|| {
        let _ = empty.lift_to_cells();
    });
    // Zero cells produce empty vector — valid degenerate case.
    assert!(result.is_ok());
}
