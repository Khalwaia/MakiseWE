//! Apartment electricity and water infrastructure as unit-typed
//! flow-conservation mechanisms.
//!
//! Each network is an admission-limited delivery channel: loads are
//! admitted against declared capacity with exact integer accounting,
//! delivered amounts are exact products of rate and time, and a
//! cumulative meter records everything that left the network. A load
//! that is not currently admitted can deliver nothing — disconnection
//! stops the physical process without any promised outcome (plan 0003
//! §4 negative test).
//!
//! Envelope per ADR-0014: the branch breaker is `derived` from the
//! typical EU residential circuit (230 V × 16 A = 3680 W [IEC
//! 60898]); the kitchen tap flow is `expert_estimate` inside the
//! published 6–12 l/min mixer band. Sources are modelled as capacity
//! limits only — grid generation and mains pressure dynamics stay
//! outside this envelope; storage (battery, tank) is a separate
//! resolution upgrade. Delivered energy becomes a physical delta when
//! the caller applies it through existing thermal ports; delivered
//! water enters containers through the existing pour/spill
//! conservation.

use thiserror::Error;

#[derive(Clone, Copy, Debug, Error, Eq, PartialEq)]
pub enum InfrastructureError {
    #[error("capacity, load or duration must be positive")]
    InvalidParameters,
    #[error("request exceeds the network's remaining capacity")]
    CapacityExceeded,
    #[error("nothing is delivered through a load that is not admitted")]
    LoadNotAdmitted,
    #[error("checked arithmetic overflow in infrastructure accounting")]
    Overflow,
}

/// Declared apartment branch breaker capacity in watts: 230 V × 16 A.
/// Provenance: `derived` from the standard EU residential circuit
/// [IEC 60898]; callers declare their own network where the building
/// differs.
pub const BRANCH_BREAKER_W: i64 = 3_680;

/// Declared full-open kitchen tap flow in mm³/s (9 l/min). Provenance:
/// `expert_estimate`, midpoint of the published 6–12 l/min mixer
/// tap band [EN 817].
pub const KITCHEN_TAP_FLOW_MM3_PER_S: i64 = 150_000;

/// Electric power network: admission against a declared capacity,
/// exact energy delivery, cumulative metering.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PowerNetwork {
    capacity_w: i64,
    load_w: i64,
    delivered_uj: i64,
}

impl PowerNetwork {
    pub fn new(capacity_w: i64) -> Result<Self, InfrastructureError> {
        if capacity_w <= 0 {
            return Err(InfrastructureError::InvalidParameters);
        }
        Ok(Self {
            capacity_w,
            load_w: 0,
            delivered_uj: 0,
        })
    }

    pub fn available_watts(&self) -> i64 {
        self.capacity_w - self.load_w
    }

    pub fn load_watts(&self) -> i64 {
        self.load_w
    }

    pub fn cumulative_delivered_uj(&self) -> i64 {
        self.delivered_uj
    }

    /// Admits a constant draw against the remaining capacity. Rejected
    /// draws never partially load the network.
    pub fn admit_load(&mut self, watts: i64) -> Result<(), InfrastructureError> {
        if watts <= 0 {
            return Err(InfrastructureError::InvalidParameters);
        }
        let load = self
            .load_w
            .checked_add(watts)
            .ok_or(InfrastructureError::Overflow)?;
        if load > self.capacity_w {
            return Err(InfrastructureError::CapacityExceeded);
        }
        self.load_w = load;
        Ok(())
    }

    /// Drops an admitted draw, restoring its headroom exactly.
    pub fn drop_load(&mut self, watts: i64) -> Result<(), InfrastructureError> {
        if watts <= 0 || watts > self.load_w {
            return Err(InfrastructureError::InvalidParameters);
        }
        self.load_w -= watts;
        Ok(())
    }

    /// Delivers exactly one second of `watts` for an admitted load:
    /// E = P·1 s in microjoules, metered cumulatively. The caller
    /// applies the returned delta through existing mechanisms.
    pub fn deliver_one_second(&mut self, watts: i64) -> Result<i64, InfrastructureError> {
        if watts <= 0 || watts > self.load_w {
            return Err(InfrastructureError::LoadNotAdmitted);
        }
        let energy_uj = i128::from(watts)
            .checked_mul(1_000_000)
            .ok_or(InfrastructureError::Overflow)?;
        let energy_uj = energy_uj
            .try_into()
            .map_err(|_| InfrastructureError::Overflow)?;
        self.delivered_uj = self
            .delivered_uj
            .checked_add(energy_uj)
            .ok_or(InfrastructureError::Overflow)?;
        Ok(energy_uj)
    }
}

/// Water network: admission against a declared maximum flow, exact
/// volume delivery, cumulative metering.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WaterNetwork {
    capacity_mm3_per_s: i64,
    open_flow_mm3_per_s: i64,
    delivered_mm3: i64,
}

impl WaterNetwork {
    pub fn new(capacity_mm3_per_s: i64) -> Result<Self, InfrastructureError> {
        if capacity_mm3_per_s <= 0 {
            return Err(InfrastructureError::InvalidParameters);
        }
        Ok(Self {
            capacity_mm3_per_s,
            open_flow_mm3_per_s: 0,
            delivered_mm3: 0,
        })
    }

    pub fn open_flow_mm3_per_s(&self) -> i64 {
        self.open_flow_mm3_per_s
    }

    pub fn cumulative_delivered_mm3(&self) -> i64 {
        self.delivered_mm3
    }

    /// Admits a constant flow rate against the remaining main
    /// capacity. Rejected requests never partially open the tap.
    pub fn admit_flow(&mut self, mm3_per_s: i64) -> Result<(), InfrastructureError> {
        if mm3_per_s <= 0 {
            return Err(InfrastructureError::InvalidParameters);
        }
        let flow = self
            .open_flow_mm3_per_s
            .checked_add(mm3_per_s)
            .ok_or(InfrastructureError::Overflow)?;
        if flow > self.capacity_mm3_per_s {
            return Err(InfrastructureError::CapacityExceeded);
        }
        self.open_flow_mm3_per_s = flow;
        Ok(())
    }

    /// Closes an admitted flow exactly.
    pub fn close_flow(&mut self, mm3_per_s: i64) -> Result<(), InfrastructureError> {
        if mm3_per_s <= 0 || mm3_per_s > self.open_flow_mm3_per_s {
            return Err(InfrastructureError::InvalidParameters);
        }
        self.open_flow_mm3_per_s -= mm3_per_s;
        Ok(())
    }

    /// Delivers exactly `seconds` of an admitted flow: V = rate·t in
    /// cubic millimetres, metered cumulatively. The caller pours the
    /// returned volume into containers through the existing spill
    /// conservation.
    pub fn draw_mm3(&mut self, mm3_per_s: i64, seconds: i64) -> Result<i64, InfrastructureError> {
        if seconds <= 0 || mm3_per_s <= 0 || mm3_per_s > self.open_flow_mm3_per_s {
            return Err(InfrastructureError::LoadNotAdmitted);
        }
        let volume = i128::from(mm3_per_s)
            .checked_mul(i128::from(seconds))
            .ok_or(InfrastructureError::Overflow)?;
        let volume = volume
            .try_into()
            .map_err(|_| InfrastructureError::Overflow)?;
        self.delivered_mm3 = self
            .delivered_mm3
            .checked_add(volume)
            .ok_or(InfrastructureError::Overflow)?;
        Ok(volume)
    }
}
