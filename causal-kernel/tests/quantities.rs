use makise_causal_kernel::{
    Dimension, Quantity, QuantityError, ReservoirState, StateHash, UnitScale,
};

#[test]
fn quantity_addition_rejects_dimension_mismatch() {
    let energy = Quantity::new(1_000, Dimension::Energy, UnitScale::Micro)
        .expect("valid microjoule quantity");
    let temperature = Quantity::new(310_000, Dimension::Temperature, UnitScale::Milli)
        .expect("valid millikelvin quantity");

    let error = energy
        .checked_add(&temperature)
        .expect_err("dimension mismatch must be rejected");

    assert!(matches!(error, QuantityError::DimensionMismatch { .. }));
}

#[test]
fn quantity_addition_requires_matching_scale() {
    let first = Quantity::new(1_000, Dimension::Energy, UnitScale::Micro)
        .expect("valid microjoule quantity");
    let second =
        Quantity::new(2, Dimension::Energy, UnitScale::Milli).expect("valid millijoule quantity");

    let error = first
        .checked_add(&second)
        .expect_err("scale mismatch must be rejected before commit");

    assert!(matches!(error, QuantityError::ScaleMismatch { .. }));
}

#[test]
fn quantity_overflow_returns_typed_failure_without_wrapping() {
    let near_max = Quantity::new(i64::MAX, Dimension::Energy, UnitScale::Nano)
        .expect("max magnitude is representable");

    let error = near_max
        .checked_add(&near_max)
        .expect_err("overflow must be typed failure");

    assert!(matches!(error, QuantityError::Overflow));
}

#[test]
fn identical_logical_state_has_identical_hash() {
    let state = ReservoirState::new(5_000_000_000_000, 4_200);

    let first = StateHash::of(&state);
    let second = StateHash::of(&state);

    assert_eq!(first, second);
}

#[test]
fn changing_energy_or_capacity_changes_state_hash() {
    let baseline = ReservoirState::new(5_000_000_000_000, 4_200);
    let other_energy = ReservoirState::new(5_000_000_000_001, 4_200);
    let other_capacity = ReservoirState::new(5_000_000_000_000, 4_201);

    let baseline_hash = StateHash::of(&baseline);

    assert_ne!(baseline_hash, StateHash::of(&other_energy));
    assert_ne!(baseline_hash, StateHash::of(&other_capacity));
}
