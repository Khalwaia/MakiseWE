use thiserror::Error;

use crate::quantity::ReservoirState;

#[derive(Clone, Copy, Debug, Error, Eq, PartialEq)]
pub enum ThermalError {
    #[error("state is outside declared validity range")]
    OutsideValidityRange,
    #[error("checked arithmetic overflow in thermal transfer")]
    Overflow,
}

#[derive(Clone, Debug)]
pub struct ReservoirPair {
    hot: ReservoirState,
    cold: ReservoirState,
}

impl ReservoirPair {
    pub fn new(hot: ReservoirState, cold: ReservoirState) -> Self {
        Self { hot, cold }
    }

    pub fn hot(&self) -> &ReservoirState {
        &self.hot
    }

    pub fn cold(&self) -> &ReservoirState {
        &self.cold
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ThermalTransfer {
    delta_hot_uj: i64,
    delta_cold_uj: i64,
}

impl ThermalTransfer {
    pub fn delta_hot_uj(&self) -> i64 {
        self.delta_hot_uj
    }

    pub fn delta_cold_uj(&self) -> i64 {
        self.delta_cold_uj
    }

    fn conserves_energy(&self) -> bool {
        self.delta_hot_uj == -self.delta_cold_uj
    }
}

pub struct ThermalProposal {
    transfer: ThermalTransfer,
}

impl ThermalProposal {
    pub fn one_second(
        pair: &ReservoirPair,
        conductance_uj_per_mk_s: i64,
    ) -> Result<Self, ThermalError> {
        let hot_capacity = pair.hot.heat_capacity_microjoule_per_millikelvin();
        let cold_capacity = pair.cold.heat_capacity_microjoule_per_millikelvin();
        if hot_capacity <= 0 || cold_capacity <= 0 || conductance_uj_per_mk_s < 0 {
            return Err(ThermalError::OutsideValidityRange);
        }

        // Temperature projection: T_mK = E_uJ / C_uJ_per_mK.
        let hot_temperature_mk =
            i128::from(pair.hot.internal_energy_microjoule()).checked_div(i128::from(hot_capacity));
        let cold_temperature_mk = i128::from(pair.cold.internal_energy_microjoule())
            .checked_div(i128::from(cold_capacity));
        let hot_temperature_mk = hot_temperature_mk.ok_or(ThermalError::Overflow)?;
        let cold_temperature_mk = cold_temperature_mk.ok_or(ThermalError::Overflow)?;

        let delta_t_mk = hot_temperature_mk
            .checked_sub(cold_temperature_mk)
            .ok_or(ThermalError::Overflow)?;
        let magnitude = i128::from(conductance_uj_per_mk_s)
            .checked_mul(delta_t_mk.abs())
            .ok_or(ThermalError::Overflow)?;
        if magnitude > i128::from(i64::MAX) {
            return Err(ThermalError::Overflow);
        }
        let magnitude_i64 = magnitude as i64;

        let (delta_hot, delta_cold) = if delta_t_mk >= 0 {
            (-magnitude_i64, magnitude_i64)
        } else {
            (magnitude_i64, -magnitude_i64)
        };
        let transfer = ThermalTransfer {
            delta_hot_uj: delta_hot,
            delta_cold_uj: delta_cold,
        };
        debug_assert!(transfer.conserves_energy());

        Ok(Self { transfer })
    }

    pub fn transfer(&self) -> &ThermalTransfer {
        &self.transfer
    }
}
