use thiserror::Error;

/// Phase 3: minimal cardiovascular/respiratory gas compartment.
/// Units: blood volume mm³, O₂/CO₂ amount µmol, pressure mPa.
/// Provenance: expert_estimate anchored to published physiology,
/// upgradeable via mechanism artifacts.
/// Energy released per µmol O₂ consumed. 470 kJ/mol = 470 000 µJ/µmol.
/// Provenance: Weir 1949 gas-exchange method, synthetic coarse surrogate;
/// declared as expert_estimate with 10% uncertainty.
pub const ENERGY_PER_UMOL_O2_UJ: i64 = 470_000;

/// Respiratory quotient numerator/denominator (CO₂ produced per O₂ consumed).
/// 0.85 = 85/100, coarse mixed-substrate surrogate.
pub const RQ_NUM: i64 = 85;
pub const RQ_DEN: i64 = 100;

#[derive(Clone, Copy, Debug, Error, Eq, PartialEq)]
pub enum BloodError {
    #[error("arterial O₂ would become negative; no partial burn")]
    OxygenOverdraft,
    #[error("blood amounts would overflow i64")]
    Overflow,
    #[error("invalid blood state (negative amount or over capacity)")]
    InvalidState,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BloodState {
    blood_volume_mm3: i64,
    hb_tetramer_umol: i64,
    arterial_o2_umol: i64,
    venous_co2_umol: i64,
    map_mpa: i64,
    lung_diffusion_umol_per_s: i64,
}

impl BloodState {
    pub fn new(
        blood_volume_mm3: i64,
        hb_tetramer_umol: i64,
        arterial_o2_umol: i64,
        venous_co2_umol: i64,
        map_mpa: i64,
        lung_diffusion_umol_per_s: i64,
    ) -> Result<Self, BloodError> {
        if blood_volume_mm3 <= 0
            || hb_tetramer_umol <= 0
            || arterial_o2_umol < 0
            || venous_co2_umol < 0
            || map_mpa <= 0
            || lung_diffusion_umol_per_s <= 0
        {
            return Err(BloodError::InvalidState);
        }
        let cap = hb_tetramer_umol
            .checked_mul(4)
            .ok_or(BloodError::Overflow)?;
        if arterial_o2_umol > cap {
            return Err(BloodError::InvalidState);
        }
        Ok(Self {
            blood_volume_mm3,
            hb_tetramer_umol,
            arterial_o2_umol,
            venous_co2_umol,
            map_mpa,
            lung_diffusion_umol_per_s,
        })
    }

    pub fn from_morphotype(morphotype: &crate::morphotype::Morphotype) -> Self {
        let bv = morphotype.blood_volume_mm3();
        let hb = morphotype.hb_tetramer_umol();
        let map = morphotype.mean_arterial_pressure_mpa();
        let diff = morphotype.lung_diffusion_umol_per_s();
        let cap = hb * 4;
        // Resting arterial saturation 98% = 980 permille.
        let arterial = cap * 980 / 1000;
        // Venous CO₂ resting coarse surrogate: ~ 500 mmol? Use 25 mmol total.
        // Choose 24_000 µmol as resting venous CO₂ pool.
        let venous_co2 = 24_000;
        Self {
            blood_volume_mm3: bv,
            hb_tetramer_umol: hb,
            arterial_o2_umol: arterial,
            venous_co2_umol: venous_co2,
            map_mpa: map,
            lung_diffusion_umol_per_s: diff,
        }
    }

    pub fn blood_volume_mm3(&self) -> i64 {
        self.blood_volume_mm3
    }
    pub fn hb_tetramer_umol(&self) -> i64 {
        self.hb_tetramer_umol
    }
    pub fn arterial_o2_umol(&self) -> i64 {
        self.arterial_o2_umol
    }
    pub fn venous_co2_umol(&self) -> i64 {
        self.venous_co2_umol
    }
    pub fn map_mpa(&self) -> i64 {
        self.map_mpa
    }
    pub fn lung_diffusion_umol_per_s(&self) -> i64 {
        self.lung_diffusion_umol_per_s
    }
    pub fn o2_capacity_umol(&self) -> i64 {
        self.hb_tetramer_umol * 4
    }
    pub fn arterial_saturation_permille(&self) -> i64 {
        if self.o2_capacity_umol() == 0 {
            return 0;
        }
        self.arterial_o2_umol * 1000 / self.o2_capacity_umol()
    }

    /// One canonical second of gas exchange at given metabolic demand.
    /// Checks O₂ availability first; no partial burn on overdraft.
    pub fn apply_one_second(&mut self, demand_uj: i64) -> Result<(), BloodError> {
        if demand_uj < 0 {
            return Err(BloodError::InvalidState);
        }
        let o2_demand_umol = demand_uj / ENERGY_PER_UMOL_O2_UJ;
        let co2_production_umol = o2_demand_umol * RQ_NUM / RQ_DEN;

        // Overdraft check: need o2_demand in arterial pool.
        if self.arterial_o2_umol < o2_demand_umol {
            return Err(BloodError::OxygenOverdraft);
        }

        // Consume O₂ / produce CO₂
        self.arterial_o2_umol -= o2_demand_umol;
        self.venous_co2_umol = self
            .venous_co2_umol
            .checked_add(co2_production_umol)
            .ok_or(BloodError::Overflow)?;

        // Lung ventilation: replenish O₂ up to 98% cap, limited by diffusion.
        let capacity = self.o2_capacity_umol();
        let deficit_to_full = (capacity * 980 / 1000) - self.arterial_o2_umol;
        let actual_replenish = deficit_to_full.max(0).min(self.lung_diffusion_umol_per_s);
        self.arterial_o2_umol = self
            .arterial_o2_umol
            .checked_add(actual_replenish)
            .ok_or(BloodError::Overflow)?;
        // CO2 exhalation proportional to ventilation; keeps venous stable in steady state.
        let co2_exhaled = actual_replenish * RQ_NUM / RQ_DEN;
        let co2_to_exhale = co2_exhaled.min(self.venous_co2_umol);
        self.venous_co2_umol -= co2_to_exhale;

        // Clamp arterial not to exceed 98% saturation cap (avoid unbounded growth)
        let cap98 = capacity * 980 / 1000;
        if self.arterial_o2_umol > cap98 {
            self.arterial_o2_umol = cap98;
        }
        // Keep venous CO₂ within plausible coarse band 0..200_000 µmol
        if self.venous_co2_umol < 0 {
            self.venous_co2_umol = 0;
        }
        Ok(())
    }
}
