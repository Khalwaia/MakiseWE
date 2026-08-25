//! Room atmosphere compartment over the shared thermal port.
//!
//! The room is a single air Compartment: one declared volume of dry
//! air at rest, homogeneous within the declared uncertainty, plus an
//! exact water-vapour inventory. The air itself is a plain
//! [`ReservoirState`] derived from measured dry-air properties, so a
//! stove, a pot or an organism ambient reservoir couples to it through
//! the existing [`ThermalProposal`] discipline with no new transfer
//! code (INVARIANTS §46).
//!
//! Envelope per ADR-0014: dry-air density and specific heat are
//! `measured` [CRC Handbook, 20 °C, 1 atm]; the free-convection
//! coefficient is `expert_estimate` inside the published 5–25 W/(m²·K)
//! band for free convection in air; room temperature validity is the
//! declared −10..70 °C apartment band. Evaporation transfers mass only;
//! latent-heat coupling and a temperature-dependent saturation curve
//! are declared gaps outside this slice. Every result is exact integer
//! arithmetic; anything that does not land on whole declared units is
//! typed-rejected instead of rounded (INVARIANTS §18).

use thiserror::Error;

use crate::fluids::WATER_DENSITY_MG_PER_M3;
use crate::quantity::ReservoirState;

/// Measured dry-air density at 20 °C and 1 atm, in mg per cubic metre
/// (1.204 kg/m³). Provenance: `measured` [CRC Handbook of Chemistry
/// and Physics].
pub const DRY_AIR_DENSITY_MG_PER_M3: i64 = 1_204_000;

/// Measured constant-volume specific heat of dry air near 300 K, in
/// J/(kg·K). Provenance: `measured` [CRC Handbook of Chemistry and
/// Physics]. Constant-volume applies to the declared sealed-room
/// envelope; a leaky-room isobaric upgrade is a separate resolution.
pub const DRY_AIR_SPECIFIC_HEAT_CV_J_PER_KG_K: i64 = 718;

/// Declared free-convection coefficient for quiescent air along a
/// warmed surface, in W/(m²·K). Provenance: `expert_estimate`, midpoint
/// of the published 5–25 W/(m²·K) band for natural convection in air
/// [Incropera, Fundamentals of Heat and Mass Transfer]; callers pass
/// their own coefficient when a surface declares a different regime.
pub const FREE_CONVECTION_H_W_PER_M2_K: i64 = 10;

/// Declared apartment-envelope validity range for room air, in
/// millikelvin: −10 °C..70 °C. Provenance: `expert_estimate` scenario
/// envelope; outside it the compartment must be re-declared.
pub const MIN_ROOM_TEMPERATURE_MK: i64 = 263_150;
pub const MAX_ROOM_TEMPERATURE_MK: i64 = 343_150;

#[derive(Clone, Copy, Debug, Error, Eq, PartialEq)]
pub enum AtmosphereError {
    #[error("volume, power or duration must be positive")]
    InvalidParameters,
    #[error("state is outside the declared apartment validity range")]
    OutsideValidityRange,
    #[error("air mass is not representable in whole milligrams for this volume")]
    NonRepresentableAirMass,
    #[error("heat capacity is not representable in whole µJ/mK for this volume")]
    NonRepresentableHeatCapacity,
    #[error("convective conductance is not representable in whole µJ/(mK·s)")]
    NonRepresentableConductance,
    #[error("evaporated amount is not representable in whole milligrams")]
    NonRepresentableVapourGain,
    #[error("absolute humidity is not representable in whole µg/m³")]
    NonRepresentableHumidity,
    #[error("checked arithmetic overflow in atmosphere accounting")]
    Overflow,
}

/// Convective conductance G = h·A between a surface and the room air,
/// in µJ/(mK·s): G = h_W/(m²K) · A_mm² / 1000. An area that does not
/// divide into whole units leaves the declared envelope.
pub fn convective_conductance_uj_per_mk_s(
    h_w_per_m2k: i64,
    contact_area_mm2: i64,
) -> Result<i64, AtmosphereError> {
    if h_w_per_m2k < 0 || contact_area_mm2 <= 0 {
        return Err(AtmosphereError::InvalidParameters);
    }
    let product = i128::from(h_w_per_m2k)
        .checked_mul(i128::from(contact_area_mm2))
        .ok_or(AtmosphereError::Overflow)?;
    if product % 1000 != 0 {
        return Err(AtmosphereError::NonRepresentableConductance);
    }
    (product / 1000)
        .try_into()
        .map_err(|_| AtmosphereError::Overflow)
}

/// Exact energy delivered by a burner of `power_w` watts over
/// `seconds`: E_uJ = P·t·10⁶.
pub fn heater_energy_uj(power_w: i64, seconds: i64) -> Result<i64, AtmosphereError> {
    if power_w <= 0 || seconds <= 0 {
        return Err(AtmosphereError::InvalidParameters);
    }
    let microjoules = i128::from(power_w)
        .checked_mul(i128::from(seconds))
        .and_then(|product| product.checked_mul(1_000_000))
        .ok_or(AtmosphereError::Overflow)?;
    microjoules
        .try_into()
        .map_err(|_| AtmosphereError::Overflow)
}

fn checked_air_mass_mg(volume_mm3: i64) -> Result<i64, AtmosphereError> {
    if volume_mm3 <= 0 {
        return Err(AtmosphereError::InvalidParameters);
    }
    // m_mg = ρ_mg/m³ · V_mm³ / 10⁹.
    let scaled = i128::from(DRY_AIR_DENSITY_MG_PER_M3)
        .checked_mul(i128::from(volume_mm3))
        .ok_or(AtmosphereError::Overflow)?;
    if scaled % 1_000_000_000 != 0 {
        return Err(AtmosphereError::NonRepresentableAirMass);
    }
    let mass_mg = scaled / 1_000_000_000;
    if mass_mg % 1000 != 0 {
        return Err(AtmosphereError::NonRepresentableHeatCapacity);
    }
    mass_mg.try_into().map_err(|_| AtmosphereError::Overflow)
}

fn heat_capacity_uj_per_mk(air_mass_mg: i64) -> Result<i64, AtmosphereError> {
    // C_uJ/mK = m_kg · c_v · 1000 = m_mg · c_v / 1000; divisibility is
    // guaranteed by the air-mass admission check.
    let capacity = i128::from(air_mass_mg)
        .checked_mul(i128::from(DRY_AIR_SPECIFIC_HEAT_CV_J_PER_KG_K))
        .ok_or(AtmosphereError::Overflow)?
        / 1000;
    capacity.try_into().map_err(|_| AtmosphereError::Overflow)
}

/// One room's air compartment: declared volume, the derived thermal
/// reservoir, and the exact water-vapour inventory.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RoomAtmosphere {
    volume_mm3: i64,
    air_mass_mg: i64,
    reservoir: ReservoirState,
    vapour_mass_mg: i64,
}

impl RoomAtmosphere {
    /// Declares a sealed room whose air reservoir is built from the
    /// measured dry-air properties at `initial_temperature_mk`.
    pub fn new(volume_mm3: i64, initial_temperature_mk: i64) -> Result<Self, AtmosphereError> {
        if !(MIN_ROOM_TEMPERATURE_MK..=MAX_ROOM_TEMPERATURE_MK).contains(&initial_temperature_mk) {
            return Err(AtmosphereError::OutsideValidityRange);
        }
        let air_mass_mg = checked_air_mass_mg(volume_mm3)?;
        let capacity = heat_capacity_uj_per_mk(air_mass_mg)?;
        let energy = i128::from(capacity)
            .checked_mul(i128::from(initial_temperature_mk))
            .ok_or(AtmosphereError::Overflow)?;
        let energy = energy.try_into().map_err(|_| AtmosphereError::Overflow)?;
        Ok(Self {
            volume_mm3,
            air_mass_mg,
            reservoir: ReservoirState::new(energy, capacity),
            vapour_mass_mg: 0,
        })
    }

    pub fn volume_mm3(&self) -> i64 {
        self.volume_mm3
    }

    pub fn air_mass_mg(&self) -> i64 {
        self.air_mass_mg
    }

    /// The shared causal port: pair this reservoir with any stove, pot
    /// or organism reservoir through [`ThermalProposal`].
    pub fn air_reservoir(&self) -> &ReservoirState {
        &self.reservoir
    }

    /// Declared observable projection in millikelvin. Integer division;
    /// truncation error is below 1 mK by construction.
    pub fn temperature_mk(&self) -> i64 {
        self.reservoir.internal_energy_microjoule()
            / self.reservoir.heat_capacity_microjoule_per_millikelvin()
    }

    pub fn vapour_mass_mg(&self) -> i64 {
        self.vapour_mass_mg
    }

    pub fn total_gas_mass_mg(&self) -> i64 {
        self.air_mass_mg + self.vapour_mass_mg
    }

    /// Absolute humidity in µg/m³: m_vap_mg · 10¹² / V_mm³. A quotient
    /// that does not land on whole units leaves the declared envelope.
    pub fn absolute_humidity_ug_per_m3(&self) -> Result<i64, AtmosphereError> {
        let scaled = i128::from(self.vapour_mass_mg)
            .checked_mul(1_000_000_000_000)
            .ok_or(AtmosphereError::Overflow)?;
        if scaled % i128::from(self.volume_mm3) != 0 {
            return Err(AtmosphereError::NonRepresentableHumidity);
        }
        (scaled / i128::from(self.volume_mm3))
            .try_into()
            .map_err(|_| AtmosphereError::Overflow)
    }

    /// Applies signed heater or transfer energy atomically: the next
    /// state is validated against the declared envelope before it
    /// replaces the current one, so a rejected application leaves the
    /// compartment untouched.
    pub fn apply_heating(&mut self, energy_uj: i64) -> Result<(), AtmosphereError> {
        let capacity = self.reservoir.heat_capacity_microjoule_per_millikelvin();
        let next_energy = self
            .reservoir
            .internal_energy_microjoule()
            .checked_add(energy_uj)
            .ok_or(AtmosphereError::Overflow)?;
        let next_temperature_mks = i128::from(next_energy) / i128::from(capacity);
        if !(i128::from(MIN_ROOM_TEMPERATURE_MK)..=i128::from(MAX_ROOM_TEMPERATURE_MK))
            .contains(&next_temperature_mks)
        {
            return Err(AtmosphereError::OutsideValidityRange);
        }
        self.reservoir = ReservoirState::new(next_energy, capacity);
        Ok(())
    }

    /// Moves liquid water into the vapour inventory using the measured
    /// fresh-water density shared with the fluids mechanism:
    /// m_vap_mg = V_mm³ · ρ / 10⁹, exact or rejected. Mass-only by
    /// declaration — latent-heat coupling stays outside this envelope.
    /// Returns the exact vapour mass gained.
    pub fn evaporate_in(&mut self, liquid_volume_mm3: i64) -> Result<i64, AtmosphereError> {
        if liquid_volume_mm3 <= 0 {
            return Err(AtmosphereError::InvalidParameters);
        }
        let scaled = i128::from(WATER_DENSITY_MG_PER_M3)
            .checked_mul(i128::from(liquid_volume_mm3))
            .ok_or(AtmosphereError::Overflow)?;
        if scaled % 1_000_000_000 != 0 {
            return Err(AtmosphereError::NonRepresentableVapourGain);
        }
        let gained_mg = (scaled / 1_000_000_000)
            .try_into()
            .map_err(|_| AtmosphereError::Overflow)?;
        self.vapour_mass_mg = self
            .vapour_mass_mg
            .checked_add(gained_mg)
            .ok_or(AtmosphereError::Overflow)?;
        Ok(gained_mg)
    }
}
