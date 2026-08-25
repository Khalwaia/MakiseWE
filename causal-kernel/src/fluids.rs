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
    #[error("puddle depth is not representable in whole nanometres")]
    NonRepresentableDepth,
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

/// Declared liquid container: rigid, open-topped, with an exact
/// capacity and current content in cubic millimetres (1 ml = 1000
/// mm³). Pure accounting state — no pressure model inside this type;
/// hydrostatics above covers depth-dependent quantities.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LiquidContainer {
    capacity_mm3: i64,
    content_mm3: i64,
}

impl LiquidContainer {
    pub fn new(capacity_mm3: i64, content_mm3: i64) -> Result<Self, FluidError> {
        if capacity_mm3 <= 0 || content_mm3 < 0 || content_mm3 > capacity_mm3 {
            return Err(FluidError::InvalidParameters);
        }
        Ok(Self {
            capacity_mm3,
            content_mm3,
        })
    }

    pub fn capacity_mm3(&self) -> i64 {
        self.capacity_mm3
    }
    pub fn content_mm3(&self) -> i64 {
        self.content_mm3
    }
    pub fn free_space_mm3(&self) -> i64 {
        self.capacity_mm3 - self.content_mm3
    }
    pub fn is_full(&self) -> bool {
        self.content_mm3 == self.capacity_mm3
    }
    pub fn is_empty(&self) -> bool {
        self.content_mm3 == 0
    }
}

/// Proposed pour cause: a positive volume bound in cubic millimetres.
/// The request caps the transfer; it never invents liquid that the
/// source does not hold (INVARIANTS §57).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PourRequest {
    requested_mm3: i64,
}

impl PourRequest {
    pub fn new(requested_mm3: i64) -> Result<Self, FluidError> {
        if requested_mm3 <= 0 {
            return Err(FluidError::InvalidParameters);
        }
        Ok(Self { requested_mm3 })
    }

    pub fn requested_mm3(&self) -> i64 {
        self.requested_mm3
    }
}

/// Exact outcome of one evaluated pour. Volume is conserved bit-exact
/// across the boundary: source + target contents before equal next
/// source + next target contents plus the spill. Spilling past the
/// target rim is a first-class physical outcome — never an error and
/// never a silent clamp.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PourOutcome {
    next_source: LiquidContainer,
    next_target: LiquidContainer,
    transferred_mm3: i64,
    spilled_mm3: i64,
}

impl PourOutcome {
    pub fn next_source(&self) -> &LiquidContainer {
        &self.next_source
    }
    pub fn next_target(&self) -> &LiquidContainer {
        &self.next_target
    }
    pub fn transferred_mm3(&self) -> i64 {
        self.transferred_mm3
    }
    /// Liquid that left the source but did not fit the target.
    pub fn spilled_mm3(&self) -> i64 {
        self.spilled_mm3
    }
}

impl LiquidContainer {
    /// Evaluates pouring `request` from this container into `target`.
    /// Pure function of both containers and the request; repeated
    /// evaluation yields identical outcomes.
    pub fn pour_into(
        &self,
        target: &Self,
        request: &PourRequest,
    ) -> Result<PourOutcome, FluidError> {
        if std::ptr::eq(self, target) {
            return Err(FluidError::InvalidParameters);
        }
        let given = self.content_mm3.min(request.requested_mm3);
        let accepted = given.min(target.free_space_mm3());
        let spilled = given - accepted;

        let next_source_content = self.content_mm3 - given;
        let next_target_content = target
            .content_mm3
            .checked_add(accepted)
            .ok_or(FluidError::Overflow)?;

        Ok(PourOutcome {
            next_source: Self {
                capacity_mm3: self.capacity_mm3,
                content_mm3: next_source_content,
            },
            next_target: Self {
                capacity_mm3: target.capacity_mm3,
                content_mm3: next_target_content,
            },
            transferred_mm3: accepted,
            spilled_mm3: spilled,
        })
    }
}

/// Free-surface depth of a spilled puddle standing on a level floor
/// footprint, in nanometres: h_nm = V_mm³ / A_mm² · 10⁶. A quotient
/// that does not land on whole nanometres leaves the declared envelope
/// instead of silently tilting or rounding the surface.
pub fn puddle_depth_nm(volume_mm3: i64, footprint_area_mm2: i64) -> Result<i64, FluidError> {
    if volume_mm3 <= 0 || footprint_area_mm2 <= 0 {
        return Err(FluidError::InvalidParameters);
    }
    let scaled = i128::from(volume_mm3)
        .checked_mul(1_000_000)
        .ok_or(FluidError::Overflow)?;
    if scaled % i128::from(footprint_area_mm2) != 0 {
        return Err(FluidError::NonRepresentableDepth);
    }
    (scaled / i128::from(footprint_area_mm2))
        .try_into()
        .map_err(|_| FluidError::Overflow)
}
