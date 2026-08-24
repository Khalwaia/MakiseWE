use thiserror::Error;

#[derive(Clone, Copy, Debug, Error, Eq, PartialEq)]
pub enum OrganismError {
    #[error("metabolic demand exceeds chemical store; no partial application")]
    ChemicalOverdraft,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OrganismState {
    chemical_store_uj: i64,
    core_internal_energy_uj: i64,
}

impl OrganismState {
    pub fn new(chemical_store_uj: i64, core_internal_energy_uj: i64) -> Self {
        Self {
            chemical_store_uj,
            core_internal_energy_uj,
        }
    }

    pub fn chemical_store_uj(&self) -> i64 {
        self.chemical_store_uj
    }

    pub fn core_internal_energy_uj(&self) -> i64 {
        self.core_internal_energy_uj
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
}
