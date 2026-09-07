use thiserror::Error;

/// Coarse renal/fluid representation. Amounts use mm³ and µmol; a
/// 420 000 µmol plasma sodium amount in 3 L gives 140 mmol/L.
/// Provenance: expert_estimate/synthetic fixture; valid only at resting
/// baseline, with a nephron-segment resolution upgrade required for disease,
/// hormone control, or sustained volume disturbance.
pub const BASELINE_TOTAL_BODY_WATER_MM3: i64 = 42_000_000;
pub const BASELINE_PLASMA_MM3: i64 = 3_000_000;
pub const BASELINE_PLASMA_SODIUM_UMOL: i64 = 420_000;
const MIN_TOTAL_BODY_WATER_MM3: i64 = 40_000_000;
const MAX_TOTAL_BODY_WATER_MM3: i64 = 45_000_000;
const MIN_PLASMA_MM3: i64 = 2_000_000;
const MAX_PLASMA_MM3: i64 = 5_000_000;
const MIN_PLASMA_SODIUM_UMOL: i64 = 400_000;
const MAX_PLASMA_SODIUM_UMOL: i64 = 510_000;
pub const MAX_CORRECTIVE_URINE_WATER_MM3_PER_SECOND: i64 = 17;
pub const MAX_CORRECTIVE_URINE_SODIUM_UMOL_PER_SECOND: i64 = 1;

#[derive(Clone, Copy, Debug, Error, Eq, PartialEq)]
pub enum RenalError {
    #[error("fluid intake must have non-negative water and sodium, with at least one positive")]
    InvalidInput,
    #[error("renal state would leave its declared resting validity range")]
    OutsideValidityRange,
    #[error("renal amount overflow")]
    Overflow,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RenalState {
    total_body_water_mm3: i64,
    plasma_mm3: i64,
    plasma_sodium_umol: i64,
    urine_water_mm3: i64,
    urine_sodium_umol: i64,
}

impl RenalState {
    pub fn baseline() -> Self {
        Self {
            total_body_water_mm3: BASELINE_TOTAL_BODY_WATER_MM3,
            plasma_mm3: BASELINE_PLASMA_MM3,
            plasma_sodium_umol: BASELINE_PLASMA_SODIUM_UMOL,
            urine_water_mm3: 0,
            urine_sodium_umol: 0,
        }
    }

    pub fn new(
        total_body_water_mm3: i64,
        plasma_mm3: i64,
        plasma_sodium_umol: i64,
        urine_water_mm3: i64,
        urine_sodium_umol: i64,
    ) -> Result<Self, RenalError> {
        if !(MIN_TOTAL_BODY_WATER_MM3..=MAX_TOTAL_BODY_WATER_MM3).contains(&total_body_water_mm3)
            || !(MIN_PLASMA_MM3..=MAX_PLASMA_MM3).contains(&plasma_mm3)
            || plasma_mm3 > total_body_water_mm3
            || !(MIN_PLASMA_SODIUM_UMOL..=MAX_PLASMA_SODIUM_UMOL).contains(&plasma_sodium_umol)
            || urine_water_mm3 < 0
            || urine_sodium_umol < 0
        {
            return Err(RenalError::OutsideValidityRange);
        }
        Ok(Self {
            total_body_water_mm3,
            plasma_mm3,
            plasma_sodium_umol,
            urine_water_mm3,
            urine_sodium_umol,
        })
    }

    pub fn total_body_water_mm3(&self) -> i64 {
        self.total_body_water_mm3
    }
    pub fn plasma_mm3(&self) -> i64 {
        self.plasma_mm3
    }
    pub fn plasma_sodium_umol(&self) -> i64 {
        self.plasma_sodium_umol
    }
    pub fn urine_water_mm3(&self) -> i64 {
        self.urine_water_mm3
    }
    pub fn urine_sodium_umol(&self) -> i64 {
        self.urine_sodium_umol
    }

    /// mmol/L × 1000; integer error is below 0.001 mmol/L.
    pub fn plasma_sodium_mmol_per_l_milli(&self) -> i64 {
        self.plasma_sodium_umol * 1_000_000 / self.plasma_mm3
    }

    pub fn stage_intake(&mut self, water_mm3: i64, sodium_umol: i64) -> Result<(), RenalError> {
        if water_mm3 < 0 || sodium_umol < 0 || (water_mm3 == 0 && sodium_umol == 0) {
            return Err(RenalError::InvalidInput);
        }
        let total_body_water_mm3 = self
            .total_body_water_mm3
            .checked_add(water_mm3)
            .ok_or(RenalError::Overflow)?;
        let plasma_mm3 = self
            .plasma_mm3
            .checked_add(water_mm3)
            .ok_or(RenalError::Overflow)?;
        let plasma_sodium_umol = self
            .plasma_sodium_umol
            .checked_add(sodium_umol)
            .ok_or(RenalError::Overflow)?;
        let next = Self::new(
            total_body_water_mm3,
            plasma_mm3,
            plasma_sodium_umol,
            self.urine_water_mm3,
            self.urine_sodium_umol,
        )?;
        *self = next;
        Ok(())
    }

    /// Removes only excess over the declared resting baseline. This coarse
    /// model has no insensible-loss or endocrine port yet, so it never
    /// invents an unpaired baseline outflow.
    pub fn excrete_one_second(&mut self) -> Result<i64, RenalError> {
        let water_loss = (self.total_body_water_mm3 - BASELINE_TOTAL_BODY_WATER_MM3)
            .clamp(0, MAX_CORRECTIVE_URINE_WATER_MM3_PER_SECOND);
        let sodium_loss = (self.plasma_sodium_umol - BASELINE_PLASMA_SODIUM_UMOL)
            .clamp(0, MAX_CORRECTIVE_URINE_SODIUM_UMOL_PER_SECOND);
        self.total_body_water_mm3 -= water_loss;
        self.plasma_mm3 -= water_loss;
        self.plasma_sodium_umol -= sodium_loss;
        self.urine_water_mm3 = self
            .urine_water_mm3
            .checked_add(water_loss)
            .ok_or(RenalError::Overflow)?;
        self.urine_sodium_umol = self
            .urine_sodium_umol
            .checked_add(sodium_loss)
            .ok_or(RenalError::Overflow)?;
        Ok(water_loss)
    }
}
