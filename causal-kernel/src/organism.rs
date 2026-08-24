use thiserror::Error;

use crate::morphotype::Morphotype;
use crate::quantity::ReservoirState;
use crate::thermal::{ReservoirPair, ThermalProposal};

#[derive(Clone, Copy, Debug, Error, Eq, PartialEq)]
pub enum OrganismError {
    #[error("metabolic demand exceeds chemical store; no partial application")]
    ChemicalOverdraft,
    #[error("thermal exchange rejected: {0}")]
    Thermal(#[from] crate::thermal::ThermalError),
    #[error("checked arithmetic overflow in organism state")]
    Overflow,
}

/// Declared reference temperatures for baseline state construction and
/// validation bands. Provenance: `expert_estimate` anchored to published
/// human physiology (Mackowiak 1992 circadian band around 36.4–37.4 °C;
/// nominal apartment surrogate 20 °C) as summarized in
/// docs/research/biology-realism.md.
pub const REFERENCE_CORE_TEMPERATURE_MK: i64 = 310_150;
pub const REFERENCE_AMBIENT_TEMPERATURE_MK: i64 = 293_150;

/// Room-sized thermal surrogate capacity: 1e7 J/K expressed in µJ/mK.
/// Chosen so a full day of metabolic heat input shifts ambient by < 1 K.
pub const AMBIENT_HEAT_CAPACITY_UJ_PER_MK: i64 = 10_000_000_000;

/// Baseline reservoir internal energies: heat capacity × reference
/// temperature, exact integer products.
pub const BASELINE_CORE_INTERNAL_ENERGY_UJ: i64 = 67_110_257_000_000;
pub const BASELINE_AMBIENT_INTERNAL_ENERGY_UJ: i64 = 2_931_500_000_000_000;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OrganismState {
    chemical_store_uj: i64,
    core_internal_energy_uj: i64,
    ambient_reservoir: ReservoirState,
    morphotype: Morphotype,
}

impl OrganismState {
    pub fn new(chemical_store_uj: i64, core_internal_energy_uj: i64) -> Self {
        Self {
            chemical_store_uj,
            core_internal_energy_uj,
            ambient_reservoir: ReservoirState::new(
                BASELINE_AMBIENT_INTERNAL_ENERGY_UJ,
                AMBIENT_HEAT_CAPACITY_UJ_PER_MK,
            ),
            morphotype: Morphotype::human(),
        }
    }

    pub fn with_morphotype(
        morphotype: &Morphotype,
        chemical_store_uj: i64,
        core_internal_energy_uj: i64,
        ambient_reservoir: ReservoirState,
    ) -> Self {
        Self {
            chemical_store_uj,
            core_internal_energy_uj,
            ambient_reservoir,
            morphotype: morphotype.clone(),
        }
    }

    pub fn with_ambient(
        chemical_store_uj: i64,
        core_internal_energy_uj: i64,
        ambient_reservoir: ReservoirState,
    ) -> Self {
        Self {
            chemical_store_uj,
            core_internal_energy_uj,
            ambient_reservoir,
            morphotype: Morphotype::human(),
        }
    }

    /// Builds a baseline organism whose core internal energy equals the
    /// declared morphotype heat capacity × reference core temperature,
    /// with the room surrogate at reference ambient temperature. Using a
    /// shared raw energy across morphotypes would silently encode
    /// different temperatures.
    pub fn physiological_baseline(morphotype: &Morphotype) -> Self {
        let core_internal_energy_uj = morphotype
            .core_heat_capacity_uj_per_mk()
            .checked_mul(REFERENCE_CORE_TEMPERATURE_MK)
            .expect("baseline core energy fits i64");
        Self {
            chemical_store_uj: crate::interoception::INITIAL_CHEMICAL_STORE_UJ,
            core_internal_energy_uj,
            ambient_reservoir: ReservoirState::new(
                BASELINE_AMBIENT_INTERNAL_ENERGY_UJ,
                AMBIENT_HEAT_CAPACITY_UJ_PER_MK,
            ),
            morphotype: morphotype.clone(),
        }
    }

    pub(crate) fn with_ambient_from_row(
        chemical_store_uj: i64,
        core_internal_energy_uj: i64,
        ambient_energy_uj: i64,
        ambient_capacity_uj_per_mk: i64,
    ) -> Self {
        Self::with_ambient(
            chemical_store_uj,
            core_internal_energy_uj,
            ReservoirState::new(ambient_energy_uj, ambient_capacity_uj_per_mk),
        )
    }

    pub fn chemical_store_uj(&self) -> i64 {
        self.chemical_store_uj
    }

    pub fn core_internal_energy_uj(&self) -> i64 {
        self.core_internal_energy_uj
    }

    pub fn ambient_reservoir(&self) -> &ReservoirState {
        &self.ambient_reservoir
    }

    pub fn ambient_internal_energy_uj(&self) -> i64 {
        self.ambient_reservoir.internal_energy_microjoule()
    }

    pub fn morphotype(&self) -> &Morphotype {
        &self.morphotype
    }

    /// Declared observable projection: core temperature in millikelvin.
    /// Integer division; truncation error is below 1 mK by construction.
    pub fn core_temperature_mk(&self) -> i64 {
        self.core_internal_energy_uj / self.morphotype.core_heat_capacity_uj_per_mk()
    }

    /// Declared observable projection: ambient reservoir temperature in
    /// millikelvin. Integer division; truncation error is below 1 mK.
    pub fn ambient_temperature_mk(&self) -> i64 {
        self.ambient_reservoir.internal_energy_microjoule()
            / self
                .ambient_reservoir
                .heat_capacity_microjoule_per_millikelvin()
    }

    /// Exchanges exactly one second of thermal energy between organism core
    /// and ambient environment using the shared thermal mechanism. Total
    /// accounted energy is conserved without tolerance.
    pub fn apply_ambient_exchange(&mut self) -> Result<(), OrganismError> {
        let core = ReservoirState::new(
            self.core_internal_energy_uj,
            self.morphotype.core_heat_capacity_uj_per_mk(),
        );
        let pair = ReservoirPair::new(core, self.ambient_reservoir);
        let proposal =
            ThermalProposal::one_second(&pair, self.morphotype.ambient_conductance_uj_per_mk_s())
                .map_err(OrganismError::Thermal)?;
        let transfer = proposal.transfer();
        self.core_internal_energy_uj += transfer.delta_hot_uj();
        let new_ambient_energy = self
            .ambient_reservoir
            .internal_energy_microjoule()
            .checked_add(transfer.delta_cold_uj())
            .ok_or(OrganismError::Overflow)?;
        self.ambient_reservoir = ReservoirState::new(
            new_ambient_energy,
            self.ambient_reservoir
                .heat_capacity_microjoule_per_millikelvin(),
        );
        Ok(())
    }

    /// Converts exactly `demand_uj` of chemical store into core thermal energy.
    /// Total accounted energy is conserved without tolerance.
    pub fn apply_metabolism(&mut self, demand_uj: i64) -> Result<(), OrganismError> {
        if demand_uj < 0 || self.chemical_store_uj < demand_uj {
            return Err(OrganismError::ChemicalOverdraft);
        }
        self.chemical_store_uj -= demand_uj;
        self.core_internal_energy_uj += demand_uj;
        Ok(())
    }

    pub fn total_accounted_uj(&self) -> i64 {
        self.chemical_store_uj + self.core_internal_energy_uj
    }

    /// Adds absorbed chemical energy from digestion. Exact by construction.
    pub fn absorb_chemical_energy(&mut self, energy_uj: i64) {
        self.chemical_store_uj += energy_uj;
    }
}
