use thiserror::Error;

use crate::blood::{BloodError, BloodState};
use crate::morphotype::Morphotype;
use crate::quantity::ReservoirState;
use crate::renal::{RenalError, RenalState};
use crate::thermal::{ReservoirPair, ThermalProposal};

#[derive(Clone, Copy, Debug, Error, Eq, PartialEq)]
pub enum OrganismError {
    #[error("metabolic demand exceeds chemical store; no partial application")]
    ChemicalOverdraft,
    #[error("thermal exchange rejected: {0}")]
    Thermal(#[from] crate::thermal::ThermalError),
    #[error("ingestion exceeds declared chemical capacity; no partial application")]
    DigestiveCapacityExceeded,
    #[error("checked arithmetic overflow in organism state")]
    Overflow,
    #[error("arterial O₂ overdraft; no partial metabolism")]
    OxygenOverdraft,
    #[error("blood state invalid")]
    Blood(#[from] BloodError),
    #[error("renal state rejected: {0}")]
    Renal(#[from] RenalError),
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
    digestion_buffer_uj: i64,
    core_internal_energy_uj: i64,
    ambient_reservoir: ReservoirState,
    morphotype: Morphotype,
    blood: BloodState,
    renal: RenalState,
    // Phase 3.3 substrate compartments
    plasma_glucose_mmol: i64,
    liver_glycogen_mmol: i64,
    fecal_dry_mass_mg: i64,
}

impl OrganismState {
    pub fn new(chemical_store_uj: i64, core_internal_energy_uj: i64) -> Self {
        let morphotype = Morphotype::human();
        Self {
            chemical_store_uj,
            digestion_buffer_uj: 0,
            core_internal_energy_uj,
            ambient_reservoir: ReservoirState::new(
                BASELINE_AMBIENT_INTERNAL_ENERGY_UJ,
                AMBIENT_HEAT_CAPACITY_UJ_PER_MK,
            ),
            morphotype: morphotype.clone(),
            blood: BloodState::from_morphotype(&morphotype),
            renal: RenalState::baseline(),
            plasma_glucose_mmol: 15_000, // 5.0 mmol/L × 3L plasma = 15 mmol
            liver_glycogen_mmol: 400_000, // ~400 mmol glucose equivalents
            fecal_dry_mass_mg: 0,
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
            digestion_buffer_uj: 0,
            core_internal_energy_uj,
            ambient_reservoir,
            morphotype: morphotype.clone(),
            blood: BloodState::from_morphotype(morphotype),
            renal: RenalState::baseline(),
            plasma_glucose_mmol: 15_000,
            liver_glycogen_mmol: 400_000,
            fecal_dry_mass_mg: 0,
        }
    }

    pub fn with_ambient(
        chemical_store_uj: i64,
        core_internal_energy_uj: i64,
        ambient_reservoir: ReservoirState,
    ) -> Self {
        let morphotype = Morphotype::human();
        Self {
            chemical_store_uj,
            digestion_buffer_uj: 0,
            core_internal_energy_uj,
            ambient_reservoir,
            morphotype: morphotype.clone(),
            blood: BloodState::from_morphotype(&morphotype),
            renal: RenalState::baseline(),
            plasma_glucose_mmol: 15_000,
            liver_glycogen_mmol: 400_000,
            fecal_dry_mass_mg: 0,
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
            digestion_buffer_uj: 0,
            core_internal_energy_uj,
            ambient_reservoir: ReservoirState::new(
                BASELINE_AMBIENT_INTERNAL_ENERGY_UJ,
                AMBIENT_HEAT_CAPACITY_UJ_PER_MK,
            ),
            morphotype: morphotype.clone(),
            blood: BloodState::from_morphotype(morphotype),
            renal: RenalState::baseline(),
            plasma_glucose_mmol: 15_000,
            liver_glycogen_mmol: 400_000,
            fecal_dry_mass_mg: 0,
        }
    }

    pub(crate) fn with_ambient_from_row(
        chemical_store_uj: i64,
        digestion_buffer_uj: i64,
        core_internal_energy_uj: i64,
        ambient_energy_uj: i64,
        ambient_capacity_uj_per_mk: i64,
    ) -> Self {
        let mut organism = Self::with_ambient(
            chemical_store_uj,
            core_internal_energy_uj,
            ReservoirState::new(ambient_energy_uj, ambient_capacity_uj_per_mk),
        );
        organism.digestion_buffer_uj = digestion_buffer_uj;
        organism.plasma_glucose_mmol = 15_000;
        organism.liver_glycogen_mmol = 400_000;
        organism.fecal_dry_mass_mg = 0;
        organism
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn with_blood_from_row(
        chemical_store_uj: i64,
        digestion_buffer_uj: i64,
        core_internal_energy_uj: i64,
        ambient_energy_uj: i64,
        ambient_capacity_uj_per_mk: i64,
        blood_volume_mm3: i64,
        hb_tetramer_umol: i64,
        arterial_o2_umol: i64,
        venous_co2_umol: i64,
        map_mpa: i64,
        lung_diffusion_umol_per_s: i64,
        total_body_water_mm3: i64,
        plasma_mm3: i64,
        plasma_sodium_umol: i64,
        urine_water_mm3: i64,
        urine_sodium_umol: i64,
        plasma_glucose_mmol: i64,
        liver_glycogen_mmol: i64,
        fecal_dry_mass_mg: i64,
    ) -> Self {
        // Derive morphotype from stored diffusion/blood params by matching known morphotypes.
        // Fallback to human if unknown — migration preserves exact amounts anyway.
        let morphotype = if blood_volume_mm3 == Morphotype::neko().blood_volume_mm3()
            && hb_tetramer_umol == Morphotype::neko().hb_tetramer_umol()
        {
            Morphotype::neko()
        } else {
            Morphotype::human()
        };
        let blood = BloodState::new(
            blood_volume_mm3,
            hb_tetramer_umol,
            arterial_o2_umol,
            venous_co2_umol,
            map_mpa,
            lung_diffusion_umol_per_s,
        )
        .unwrap_or_else(|_| BloodState::from_morphotype(&morphotype));
        let renal = RenalState::new(
            total_body_water_mm3,
            plasma_mm3,
            plasma_sodium_umol,
            urine_water_mm3,
            urine_sodium_umol,
        )
        .unwrap_or_else(|_| RenalState::baseline());
        Self {
            chemical_store_uj,
            digestion_buffer_uj,
            core_internal_energy_uj,
            ambient_reservoir: ReservoirState::new(ambient_energy_uj, ambient_capacity_uj_per_mk),
            morphotype: morphotype.clone(),
            blood,
            renal,
            plasma_glucose_mmol,
            liver_glycogen_mmol,
            fecal_dry_mass_mg,
        }
    }

    pub fn chemical_store_uj(&self) -> i64 {
        self.chemical_store_uj
    }

    /// Energy currently held in the digestive tract awaiting absorption.
    pub fn digestion_buffer_uj(&self) -> i64 {
        self.digestion_buffer_uj
    }

    /// Declared chemical capacity of the organism; ingestion beyond
    /// store + buffer headroom is a typed rejection.
    pub fn chemical_capacity_uj(&self) -> i64 {
        crate::interoception::INITIAL_CHEMICAL_STORE_UJ
    }

    /// Buffers ingested energy without crediting the store. The caller is
    /// responsible for capacity validation before mutation.
    pub fn stage_ingestion(&mut self, energy_uj: i64) -> Result<(), OrganismError> {
        let total = self
            .chemical_store_uj
            .checked_add(self.digestion_buffer_uj)
            .ok_or(OrganismError::Overflow)?;
        let new_total = total
            .checked_add(energy_uj)
            .ok_or(OrganismError::Overflow)?;
        if new_total > self.chemical_capacity_uj() {
            return Err(OrganismError::DigestiveCapacityExceeded);
        }
        self.digestion_buffer_uj = self
            .digestion_buffer_uj
            .checked_add(energy_uj)
            .ok_or(OrganismError::Overflow)?;
        Ok(())
    }

    /// Consumes absorbed flux from the buffer. Exact by construction;
    /// the caller guarantees `energy_uj` does not exceed the buffer.
    pub fn consume_digestion_buffer(&mut self, energy_uj: i64) {
        debug_assert!(self.digestion_buffer_uj >= energy_uj);
        self.digestion_buffer_uj -= energy_uj;
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

    // --- Blood gas proxies (Phase 3) ---

    pub fn blood(&self) -> &BloodState {
        &self.blood
    }

    pub fn blood_volume_mm3(&self) -> i64 {
        self.blood.blood_volume_mm3()
    }

    pub fn blood_o2_capacity_umol(&self) -> i64 {
        self.blood.o2_capacity_umol()
    }

    pub fn arterial_o2_umol(&self) -> i64 {
        self.blood.arterial_o2_umol()
    }

    pub fn venous_co2_umol(&self) -> i64 {
        self.blood.venous_co2_umol()
    }

    pub fn arterial_saturation_permille(&self) -> i64 {
        self.blood.arterial_saturation_permille()
    }

    pub fn mean_arterial_pressure_mpa(&self) -> i64 {
        self.blood.map_mpa()
    }

    pub fn total_body_water_mm3(&self) -> i64 {
        self.renal.total_body_water_mm3()
    }
    pub fn plasma_sodium_mmol_per_l_milli(&self) -> i64 {
        self.renal.plasma_sodium_mmol_per_l_milli()
    }
    pub fn urine_water_mm3(&self) -> i64 {
        self.renal.urine_water_mm3()
    }
    pub fn renal(&self) -> &RenalState {
        &self.renal
    }

    // --- Phase 3.3 substrate compartments ---

    pub fn plasma_glucose_mmol(&self) -> i64 {
        self.plasma_glucose_mmol
    }

    pub fn liver_glycogen_mmol(&self) -> i64 {
        self.liver_glycogen_mmol
    }

    pub fn fecal_dry_mass_mg(&self) -> i64 {
        self.fecal_dry_mass_mg
    }

    pub(crate) fn absorb_glucose(&mut self, mmol: i64) {
        self.plasma_glucose_mmol = self.plasma_glucose_mmol.saturating_add(mmol);
    }

    pub(crate) fn consume_plasma_glucose(&mut self, mmol: i64) {
        debug_assert!(self.plasma_glucose_mmol >= mmol);
        self.plasma_glucose_mmol = self.plasma_glucose_mmol.saturating_sub(mmol);
    }

    pub(crate) fn synthesize_glycogen(&mut self, mmol: i64) {
        self.liver_glycogen_mmol = self.liver_glycogen_mmol.saturating_add(mmol);
    }

    pub(crate) fn breakdown_glycogen(&mut self, mmol: i64) {
        debug_assert!(self.liver_glycogen_mmol >= mmol);
        self.liver_glycogen_mmol = self.liver_glycogen_mmol.saturating_sub(mmol);
    }

    pub(crate) fn accumulate_fecal_mass(&mut self, mg: i64) {
        self.fecal_dry_mass_mg = self.fecal_dry_mass_mg.saturating_add(mg);
    }

    pub(crate) fn clear_fecal_mass(&mut self) {
        self.fecal_dry_mass_mg = 0;
    }

    pub fn stage_fluid_intake(
        &mut self,
        water_mm3: i64,
        sodium_umol: i64,
    ) -> Result<(), RenalError> {
        let mut next_renal = self.renal.clone();
        let mut next_blood = self.blood.clone();
        next_renal.stage_intake(water_mm3, sodium_umol)?;
        next_blood
            .adjust_volume_and_map(water_mm3)
            .map_err(|_| RenalError::OutsideValidityRange)?;
        self.renal = next_renal;
        self.blood = next_blood;
        Ok(())
    }

    pub fn apply_renal_for_second(&mut self) -> Result<(), RenalError> {
        let mut next_renal = self.renal.clone();
        let mut next_blood = self.blood.clone();
        let water_loss = next_renal.excrete_one_second()?;
        next_blood
            .adjust_volume_and_map(-water_loss)
            .map_err(|_| RenalError::OutsideValidityRange)?;
        self.renal = next_renal;
        self.blood = next_blood;
        Ok(())
    }

    /// Applies already validated renal and blood candidates as one writer
    /// operation.  The candidates are produced by an admitted executable
    /// artifact; this seam deliberately performs no additional derivation.
    pub(crate) fn apply_renal_blood_candidates(&mut self, renal: RenalState, blood: BloodState) {
        self.renal = renal;
        self.blood = blood;
    }

    /// One second of gas exchange at given metabolic demand. Typed rejection
    /// without partial burn if O₂ insufficient. Caller must have already
    /// validated chemical store availability if coupling to metabolism.
    pub fn apply_gas_exchange_for_second(&mut self, demand_uj: i64) -> Result<(), OrganismError> {
        self.blood.apply_one_second(demand_uj).map_err(|e| match e {
            BloodError::OxygenOverdraft => OrganismError::OxygenOverdraft,
            other => OrganismError::Blood(other),
        })
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
        self.core_internal_energy_uj = self
            .core_internal_energy_uj
            .checked_add(transfer.delta_hot_uj())
            .ok_or(OrganismError::Overflow)?;
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
        self.core_internal_energy_uj = self
            .core_internal_energy_uj
            .checked_add(demand_uj)
            .ok_or(OrganismError::Overflow)?;
        Ok(())
    }

    pub fn total_accounted_uj(&self) -> i64 {
        self.chemical_store_uj + self.core_internal_energy_uj
    }

    /// Adds absorbed chemical energy from digestion. Exact by construction.
    pub fn absorb_chemical_energy(&mut self, energy_uj: i64) {
        self.chemical_store_uj = self
            .chemical_store_uj
            .checked_add(energy_uj)
            .expect("absorb_chemical_energy overflow is typed at stage_ingestion");
    }
}
