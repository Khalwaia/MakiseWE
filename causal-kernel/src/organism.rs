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
            ambient_reservoir: ReservoirState::new(20_000_000_000_000, 1_000_000),
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

    pub(crate) fn with_ambient_from_row(
        chemical_store_uj: i64,
        core_internal_energy_uj: i64,
        ambient_energy_uj: i64,
    ) -> Self {
        Self::with_ambient(
            chemical_store_uj,
            core_internal_energy_uj,
            ReservoirState::new(ambient_energy_uj, 1_000_000),
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
