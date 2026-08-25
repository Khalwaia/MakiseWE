//! Phase 2 slice 13 evidence: fluid statics over an incompressible
//! Newtonian pool at rest. Independent anchors are hand-derived
//! textbook values, never the production code:
//!
//! - hydrostatic law P = ρ·g·h. With measured fresh-water density
//!   ρ = 998_200_000 mg/m³ (998.2 kg/m³ at 20 °C, CRC Handbook) and
//!   g = 9_806_650_000 nm/s², one metre of depth gives
//!   998_200_000 · 9_806_650_000 · 10⁹ / 10¹⁵ =
//!   9_788_998_030_000 nanopascals ≈ 9789 Pa;
//! - Archimedes' law F = ρ·g·V. One litre displaced gives
//!   9_788_998_030_000_000 mg·nm/s² ≈ 9.789 N;
//! - flotation verdict compares body mass against displaced mass
//!   m_disp = ρ·V/10⁹ cross-multiplied in integers (a litre of this
//!   water displaces 998_200 mg), never via rounded division.

use makise_causal_kernel::{
    FluidError, ImmersionVerdict, WATER_DENSITY_MG_PER_M3, buoyant_force_mgnm_per_s2,
    hydrostatic_pressure_npa, immersion_verdict,
};

const MM: i64 = 1_000_000;

/// One metre below the surface: the hand-multiplied anchor above,
/// exactly, in nanopascals.
#[test]
fn one_metre_of_water_presses_the_hand_derived_column() {
    let pressure = hydrostatic_pressure_npa(WATER_DENSITY_MG_PER_M3, 1_000_000_000)
        .expect("representable column");
    assert_eq!(pressure, 9_788_998_030_000);
}

/// The hydrostatic law is linear in depth: doubling the depth doubles
/// the column exactly, and ten centimetres give one tenth.
#[test]
fn hydrostatic_columns_scale_exactly_with_depth() {
    let one_metre =
        hydrostatic_pressure_npa(WATER_DENSITY_MG_PER_M3, 1_000_000_000).expect("representable");
    let two_metre =
        hydrostatic_pressure_npa(WATER_DENSITY_MG_PER_M3, 2_000_000_000).expect("representable");
    assert_eq!(two_metre, one_metre * 2);

    let decimetre =
        hydrostatic_pressure_npa(WATER_DENSITY_MG_PER_M3, 100 * MM).expect("representable");
    assert_eq!(decimetre * 10, one_metre);
}

/// Archimedes: one fully submerged litre displaces one litre of pool
/// water, whose weight is the hand-derived buoyant force above.
#[test]
fn one_submerged_litre_displaces_the_textbook_force() {
    let force = buoyant_force_mgnm_per_s2(WATER_DENSITY_MG_PER_M3, 1_000_000)
        .expect("representable volume");
    assert_eq!(force, 9_788_998_030_000_000);
}

/// A 500 g wooden block of 700 cm³ feels more upward force when fully
/// submerged than its own weight, so it bobs: it floats. An 8 kg iron
/// lump of 1000 cm³ feels far less and sinks.
#[test]
fn wood_floats_and_iron_sinks_by_exact_displaced_mass() {
    let verdict = immersion_verdict(
        500_000, // 500 g
        700_000, // 700 cm³
        WATER_DENSITY_MG_PER_M3,
    )
    .expect("valid geometry");
    assert_eq!(verdict, ImmersionVerdict::Floats);

    let verdict = immersion_verdict(
        8_000_000, // 8 kg
        1_000_000, // 1000 cm³
        WATER_DENSITY_MG_PER_M3,
    )
    .expect("valid geometry");
    assert_eq!(verdict, ImmersionVerdict::Sinks);
}

/// A litre-sized body of exactly the displaced mass (998_200 mg) sits
/// neutrally buoyant: the exact integer comparison resolves the
/// boundary without any epsilon.
#[test]
fn neutrally_buoyant_body_is_an_exact_boundary() {
    let verdict =
        immersion_verdict(998_200, 1_000_000, WATER_DENSITY_MG_PER_M3).expect("valid geometry");
    assert_eq!(verdict, ImmersionVerdict::Neutral);

    let heavier =
        immersion_verdict(998_201, 1_000_000, WATER_DENSITY_MG_PER_M3).expect("valid geometry");
    assert_eq!(heavier, ImmersionVerdict::Sinks);
}

/// Non-positive density, depth or volume are outside the declared
/// validity range and rejected at the call boundary.
#[test]
fn degenerate_inputs_are_typed() {
    assert!(matches!(
        hydrostatic_pressure_npa(0, 1_000),
        Err(FluidError::InvalidParameters)
    ));
    assert!(matches!(
        hydrostatic_pressure_npa(WATER_DENSITY_MG_PER_M3, -1),
        Err(FluidError::InvalidParameters)
    ));
    assert!(matches!(
        buoyant_force_mgnm_per_s2(WATER_DENSITY_MG_PER_M3, 0),
        Err(FluidError::InvalidParameters)
    ));
    assert!(matches!(
        immersion_verdict(0, 1_000_000, WATER_DENSITY_MG_PER_M3),
        Err(FluidError::InvalidParameters)
    ));
}

/// A depth whose pressure column leaves the whole-nanopascal envelope
/// is typed-rejected instead of silently rounded.
#[test]
fn fractional_pressure_column_is_typed() {
    // Depth 1 nm makes the column fractional: the 10¹⁵ divisor does
    // not divide the product exactly.
    let error = hydrostatic_pressure_npa(WATER_DENSITY_MG_PER_M3, 1)
        .expect_err("one-nanometre column is fractional");
    assert!(matches!(error, FluidError::NonRepresentablePressure));
}
