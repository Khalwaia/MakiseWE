//! Phase 2 slice 8 evidence: point-source field propagation with
//! declared attenuation. Independent anchors are hand-derived
//! textbook values, never the production code:
//!
//! - inverse-square law: doubling the distance quarters the field,
//!   ten times the distance cuts it to one hundredth;
//! - sound intensities are rated at the standard 1 m calibration
//!   distance; the human hearing threshold at 1 kHz is
//!   10⁻¹² W/m² = 1000 fW/m² (measured, ISO 226);
//! - photometry: illuminance E = I_v/d² needs no π at all — one
//!   candela at one metre gives exactly one lux = 1000 mlx;
//! - odour transport is declared an expert_estimate inverse-square
//!   surrogate (still air, no advection), not diffusion physics.
//!
//! Free-field geometry is assumed: walls, reflections and wind are
//! outside this slice's envelope.
use makise_causal_kernel::{
    HEARING_THRESHOLD_INTENSITY_FW_PER_M2, LIGHT_MAX_DISTANCE_NM, ODOUR_MAX_DISTANCE_NM,
    PROPAGATION_REFERENCE_DISTANCE_NM, PropagationError, SOUND_MIN_DISTANCE_NM, illuminance_mlx,
    odour_concentration_mg_per_m3, sound_intensity_fw_per_m2,
};

const M_NM: i64 = PROPAGATION_REFERENCE_DISTANCE_NM; // 1 m

/// Doubling the distance quarters an inverse-square field, and the
/// reference distance reproduces the rating identically.
#[test]
fn doubling_distance_quarters_the_field() {
    // An alarm clock rated 1 W/m² at 1 m = 10¹⁵ fW/m².
    let rated = 1_000_000_000_000_000;
    assert_eq!(
        sound_intensity_fw_per_m2(rated, 2 * M_NM).expect("inside band"),
        250_000_000_000_000,
        "hand: 1 W/m² · (1/2)² = 0.25 W/m²"
    );
    assert_eq!(
        sound_intensity_fw_per_m2(rated, 10 * M_NM).expect("inside band"),
        10_000_000_000_000,
        "hand: 1 W/m² · (1/10)² = 0.01 W/m²"
    );
    assert_eq!(
        sound_intensity_fw_per_m2(12_345, M_NM).expect("inside band"),
        12_345,
        "the calibration distance returns the rating"
    );
}

/// Distances whose quotient does not land on whole femtowatts are
/// truncated by less than one unit, deterministically, exactly like
/// the temperature projection convention.
#[test]
fn sub_unit_remainders_truncate_below_one_femtowatt() {
    let rated = 1_000_000_000_000_000;
    assert_eq!(
        sound_intensity_fw_per_m2(rated, 3 * M_NM).expect("inside band"),
        111_111_111_111_111,
        "hand: 10¹⁵/9 = 111_111_111_111_111 remainder 1/9"
    );
}

/// A 4000 fW/m² source fades to exactly its quarter at 2 m — landing
/// precisely on the measured hearing threshold.
#[test]
fn quiet_source_reaches_the_measured_threshold() {
    assert_eq!(HEARING_THRESHOLD_INTENSITY_FW_PER_M2, 1_000);
    let faded = sound_intensity_fw_per_m2(4_000, 2 * M_NM).expect("inside band");
    assert_eq!(faded, HEARING_THRESHOLD_INTENSITY_FW_PER_M2);
}

/// One candela at one metre is exactly one lux = 1000 mlx by the
/// photometric law E = I_v/d²; a 100 cd bulb lights a desk 5 m away
/// with 4000 mlx = 4 lux.
#[test]
fn candle_and_bulb_follow_the_photometric_law() {
    assert_eq!(illuminance_mlx(1, M_NM).expect("inside band"), 1_000);
    assert_eq!(illuminance_mlx(1, 2 * M_NM).expect("inside band"), 250);
    assert_eq!(
        illuminance_mlx(1, 3 * M_NM).expect("inside band"),
        111,
        "hand: 1000 mlx / 9 = 111.1, truncated"
    );
    assert_eq!(illuminance_mlx(100, 5 * M_NM).expect("inside band"), 4_000);
}

/// The odour surrogate dilutes by the same geometry: 100 mg/m³ at 1 m
/// thins to exactly 4 mg/m³ at 5 m.
#[test]
fn odour_surrogate_dilutes_by_declared_geometry() {
    assert_eq!(
        odour_concentration_mg_per_m3(100, 5 * M_NM).expect("inside band"),
        4
    );
    assert_eq!(
        odour_concentration_mg_per_m3(80, 4 * M_NM).expect("inside band"),
        5
    );
}

/// Each modality declares its own validity band: a 40 m shout leaves
/// the sound envelope while the same geometry stays inside the light
/// envelope, and the odour surrogate stops at 10 m.
#[test]
fn validity_bands_are_typed_per_modality() {
    assert!(matches!(
        sound_intensity_fw_per_m2(1_000_000_000_000_000, 40 * M_NM),
        Err(PropagationError::OutsideValidityRange)
    ));
    assert!(illuminance_mlx(1, 40 * M_NM).is_ok(), "light reaches 100 m");
    assert!(matches!(
        odour_concentration_mg_per_m3(100, 20 * M_NM),
        Err(PropagationError::OutsideValidityRange)
    ));

    // Near-field boundaries are equally declared.
    assert!(matches!(
        sound_intensity_fw_per_m2(1_000, SOUND_MIN_DISTANCE_NM - 1),
        Err(PropagationError::OutsideValidityRange)
    ));
    assert!(matches!(
        illuminance_mlx(1, LIGHT_MAX_DISTANCE_NM + 1),
        Err(PropagationError::OutsideValidityRange)
    ));
    assert!(matches!(
        odour_concentration_mg_per_m3(1, ODOUR_MAX_DISTANCE_NM + 1),
        Err(PropagationError::OutsideValidityRange)
    ));
}

/// Zero or negative distances are parameter errors before any band is
/// consulted.
#[test]
fn degenerate_distances_are_parameter_errors() {
    for read in [
        |d| sound_intensity_fw_per_m2(1_000, d),
        |d| illuminance_mlx(1, d),
        |d| odour_concentration_mg_per_m3(1, d),
    ] {
        assert!(matches!(read(0), Err(PropagationError::InvalidParameters)));
        assert!(matches!(
            read(-M_NM),
            Err(PropagationError::InvalidParameters)
        ));
    }
    assert_eq!(SOUND_MIN_DISTANCE_NM, 50_000_000);
}

/// Repeated evaluation is bit-identical: pure function of declared
/// inputs, no hidden state.
#[test]
fn repeated_evaluation_is_bit_identical() {
    let first = sound_intensity_fw_per_m2(8_000, 2 * M_NM).expect("inside band");
    let second = sound_intensity_fw_per_m2(8_000, 2 * M_NM).expect("inside band");
    assert_eq!(first, second);
    assert_eq!(first, 2_000);
}
