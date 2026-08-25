//! Point-source field propagation over declared inverse-square
//! attenuation.
//!
//! All three modalities of this slice share one geometry kernel: a
//! field emitted by a point source thins with the square of distance,
//! I(d) = I(d_ref)·(d_ref/d)², exact integer arithmetic throughout.
//! Sources are rated at the standard 1 m calibration distance; the
//! reading at that distance is the rating itself.
//!
//! Envelope per ADR-0014:
//! - sound intensity is rated in fW/m² at 1 m; the hearing-threshold
//!   constant is `measured` [ISO 226, 20 µPa at 1 kHz ↔ 10⁻¹² W/m²];
//! - illuminance follows the photometric law E = I_v/d² from a
//!   candela rating — `measured` SI base unit, no π required;
//! - odour concentration is an `expert_estimate` inverse-square
//!   surrogate for still air; real plume advection/diffusion stays
//!   outside this envelope;
//! - free-field geometry only: walls, reflections and wind are
//!   declared gaps; each modality carries its own validity band.
//!
//! Readings are observational projections: quotients truncate below
//! one output unit deterministically (the established temperature-
//! projection convention), while band violations and degenerate
//! inputs are typed rejections (INVARIANTS §18).

use thiserror::Error;

#[derive(Clone, Copy, Debug, Error, Eq, PartialEq)]
pub enum PropagationError {
    #[error("distance must be positive")]
    InvalidParameters,
    #[error("distance is outside the modality's declared validity range")]
    OutsideValidityRange,
    #[error("checked arithmetic overflow in propagation")]
    Overflow,
}

/// Standard calibration distance for source ratings, in nanometres
/// (1 m). Provenance: `declared` convention shared by all modalities.
pub const PROPAGATION_REFERENCE_DISTANCE_NM: i64 = 1_000_000_000;

/// Sound validity band: 5 cm..30 m. Provenance: `expert_estimate`
/// apartment-scale free-field envelope; below it the point-source
/// near field invalidates the model.
pub const SOUND_MIN_DISTANCE_NM: i64 = 50_000_000;
pub const SOUND_MAX_DISTANCE_NM: i64 = 30_000_000_000;

/// Light validity band: 5 cm..100 m. Provenance: `expert_estimate`
/// free-field photometric envelope.
pub const LIGHT_MIN_DISTANCE_NM: i64 = SOUND_MIN_DISTANCE_NM;
pub const LIGHT_MAX_DISTANCE_NM: i64 = 100_000_000_000;

/// Odour surrogate validity band: 5 cm..10 m. Provenance:
/// `expert_estimate`; the still-air assumption degrades quickly
/// beyond room scale.
pub const ODOUR_MIN_DISTANCE_NM: i64 = SOUND_MIN_DISTANCE_NM;
pub const ODOUR_MAX_DISTANCE_NM: i64 = 10_000_000_000;

/// Measured human hearing threshold at 1 kHz: 10⁻¹² W/m² expressed
/// in fW/m². Provenance: `measured` [ISO 226].
pub const HEARING_THRESHOLD_INTENSITY_FW_PER_M2: i64 = 1_000;

const REFERENCE_SQUARED_I128: i128 =
    PROPAGATION_REFERENCE_DISTANCE_NM as i128 * PROPAGATION_REFERENCE_DISTANCE_NM as i128;

fn attenuate(
    numerator: i128,
    distance_nm: i64,
    min_distance_nm: i64,
    max_distance_nm: i64,
) -> Result<i64, PropagationError> {
    if distance_nm <= 0 {
        return Err(PropagationError::InvalidParameters);
    }
    if !(min_distance_nm..=max_distance_nm).contains(&distance_nm) {
        return Err(PropagationError::OutsideValidityRange);
    }
    let squared = i128::from(distance_nm)
        .checked_mul(i128::from(distance_nm))
        .ok_or(PropagationError::Overflow)?;
    // Observational projection: truncation error is below one output
    // unit by construction.
    (numerator / squared)
        .try_into()
        .map_err(|_| PropagationError::Overflow)
}

/// Sound intensity at `distance_nm` from a source rated
/// `rated_intensity_fw_per_m2` at the 1 m calibration distance,
/// in femtowatts per square metre.
pub fn sound_intensity_fw_per_m2(
    rated_intensity_fw_per_m2: i64,
    distance_nm: i64,
) -> Result<i64, PropagationError> {
    if rated_intensity_fw_per_m2 <= 0 {
        return Err(PropagationError::InvalidParameters);
    }
    let numerator = i128::from(rated_intensity_fw_per_m2)
        .checked_mul(REFERENCE_SQUARED_I128)
        .ok_or(PropagationError::Overflow)?;
    attenuate(
        numerator,
        distance_nm,
        SOUND_MIN_DISTANCE_NM,
        SOUND_MAX_DISTANCE_NM,
    )
}

/// Illuminance at `distance_nm` from a point source of
/// `luminous_intensity_cd` candela, in millilux: E = I_v/d² scaled so
/// the 1 m reading is exactly 1000 mlx per candela.
pub fn illuminance_mlx(
    luminous_intensity_cd: i64,
    distance_nm: i64,
) -> Result<i64, PropagationError> {
    if luminous_intensity_cd <= 0 {
        return Err(PropagationError::InvalidParameters);
    }
    // E_mlx = cd · 10³ lx/cd · d_ref² / d².
    let numerator = i128::from(luminous_intensity_cd)
        .checked_mul(1_000)
        .and_then(|scaled| scaled.checked_mul(REFERENCE_SQUARED_I128))
        .ok_or(PropagationError::Overflow)?;
    attenuate(
        numerator,
        distance_nm,
        LIGHT_MIN_DISTANCE_NM,
        LIGHT_MAX_DISTANCE_NM,
    )
}

/// Odour concentration at `distance_nm` from a source rated
/// `rated_concentration_mg_per_m3` at 1 m, in mg/m³. Declared
/// still-air inverse-square surrogate (`expert_estimate`), not
/// diffusion physics.
pub fn odour_concentration_mg_per_m3(
    rated_concentration_mg_per_m3: i64,
    distance_nm: i64,
) -> Result<i64, PropagationError> {
    if rated_concentration_mg_per_m3 <= 0 {
        return Err(PropagationError::InvalidParameters);
    }
    let numerator = i128::from(rated_concentration_mg_per_m3)
        .checked_mul(REFERENCE_SQUARED_I128)
        .ok_or(PropagationError::Overflow)?;
    attenuate(
        numerator,
        distance_nm,
        ODOUR_MIN_DISTANCE_NM,
        ODOUR_MAX_DISTANCE_NM,
    )
}
