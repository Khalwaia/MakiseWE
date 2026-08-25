//! Fluid statics over a declared incompressible Newtonian pool.
//!
//! The pool is at rest, fresh water, 15–25 °C; the slice declares the
//! measured fresh-water density and reuses the measured standard
//! gravity (both `measured` provenance per ADR-0014). Every result is
//! an exact integer product — hydrostatic pressure in nanopascals,
//! buoyant force in mg·nm/s² (= 1e-15 N) — and any column or force
//! that leaves the whole-unit envelope is typed-rejected instead of
//! silently rounded (INVARIANTS §18). The flotation verdict compares
//! body mass against displaced mass cross-multiplied in integers, so
//! neutral buoyancy resolves exactly.

use thiserror::Error;

use crate::rigid_body::GRAVITY_NM_PER_S2;

/// Measured fresh-water density at 20 °C, in mg per cubic metre
/// (998.2 kg/m³). Provenance: `measured` [CRC Handbook of Chemistry
/// and Physics]; validity range is the declared pool temperature band.
pub const WATER_DENSITY_MG_PER_M3: i64 = 998_200_000;

#[derive(Clone, Copy, Debug, Error, Eq, PartialEq)]
pub enum FluidError {
    #[error("fluid density, depth or volume must be positive")]
    InvalidParameters,
    #[error("hydrostatic column is not representable in whole nanopascals")]
    NonRepresentablePressure,
    #[error("buoyant force is not representable in whole mg·nm/s² units")]
    NonRepresentableForce,
    #[error("checked arithmetic overflow in fluid statics")]
    Overflow,
}

fn checked_density(density_mg_per_m3: i64) -> Result<i128, FluidError> {
    if density_mg_per_m3 <= 0 {
        return Err(FluidError::InvalidParameters);
    }
    Ok(i128::from(density_mg_per_m3))
}

/// Hydrostatic pressure P = ρ·g·h at `depth_nm` below the free surface,
/// reported in nanopascals (1e-9 Pa = mg/(m·s²) scaled by 1e-6):
/// P_nPa = ρ_mg/m³ · g_nm/s² · h_nm / 10¹⁵. A column that does not
/// divide into whole nanopascals leaves the declared envelope.
pub fn hydrostatic_pressure_npa(density_mg_per_m3: i64, depth_nm: i64) -> Result<i64, FluidError> {
    let density = checked_density(density_mg_per_m3)?;
    if depth_nm <= 0 {
        return Err(FluidError::InvalidParameters);
    }
    let product = density
        .checked_mul(i128::from(GRAVITY_NM_PER_S2))
        .and_then(|product| product.checked_mul(i128::from(depth_nm)))
        .ok_or(FluidError::Overflow)?;
    const DENOMINATOR: i128 = 1_000_000_000_000_000;
    if product % DENOMINATOR != 0 {
        return Err(FluidError::NonRepresentablePressure);
    }
    (product / DENOMINATOR)
        .try_into()
        .map_err(|_| FluidError::Overflow)
}

/// Buoyant force F = ρ·g·V for a fully submerged volume, in mg·nm/s²:
/// F = ρ_mg/m³ · g_nm/s² · V_mm³ / 10⁹. A volume that does not divide
/// into whole force units leaves the declared envelope.
pub fn buoyant_force_mgnm_per_s2(
    density_mg_per_m3: i64,
    submerged_volume_mm3: i64,
) -> Result<i64, FluidError> {
    let density = checked_density(density_mg_per_m3)?;
    if submerged_volume_mm3 <= 0 {
        return Err(FluidError::InvalidParameters);
    }
    let product = density
        .checked_mul(i128::from(GRAVITY_NM_PER_S2))
        .and_then(|product| product.checked_mul(i128::from(submerged_volume_mm3)))
        .ok_or(FluidError::Overflow)?;
    const DENOMINATOR: i128 = 1_000_000_000;
    if product % DENOMINATOR != 0 {
        return Err(FluidError::NonRepresentableForce);
    }
    (product / DENOMINATOR)
        .try_into()
        .map_err(|_| FluidError::Overflow)
}

/// Flotation verdict from full-submersion physics: the body floats
/// when its weight exceeds the buoyant force at complete submersion
/// only partially — equivalently when its mass is below the displaced
/// mass m_disp = ρ·V/10⁹. The comparison is cross-multiplied in exact
/// integers (mass·10⁹ versus ρ·V), so equality is true neutral
/// buoyancy, never an epsilon artefact.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImmersionVerdict {
    Floats,
    Neutral,
    Sinks,
}

pub fn immersion_verdict(
    body_mass_mg: i64,
    body_volume_mm3: i64,
    fluid_density_mg_per_m3: i64,
) -> Result<ImmersionVerdict, FluidError> {
    if body_mass_mg <= 0 || body_volume_mm3 <= 0 {
        return Err(FluidError::InvalidParameters);
    }
    let density = checked_density(fluid_density_mg_per_m3)?;
    // Displaced-mass comparison without division:
    //   m · 10⁹  ⋛  ρ · V
    let left = i128::from(body_mass_mg)
        .checked_mul(1_000_000_000)
        .ok_or(FluidError::Overflow)?;
    let right = density
        .checked_mul(i128::from(body_volume_mm3))
        .ok_or(FluidError::Overflow)?;
    Ok(match left.cmp(&right) {
        std::cmp::Ordering::Less => ImmersionVerdict::Floats,
        std::cmp::Ordering::Equal => ImmersionVerdict::Neutral,
        std::cmp::Ordering::Greater => ImmersionVerdict::Sinks,
    })
}

