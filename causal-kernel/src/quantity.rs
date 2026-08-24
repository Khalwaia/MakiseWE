use std::fmt;

use sha2::{Digest, Sha256};
use thiserror::Error;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Dimension {
    Energy,
    Temperature,
}

impl Dimension {
    fn canonical_name(self) -> &'static str {
        match self {
            Self::Energy => "energy",
            Self::Temperature => "thermodynamic_temperature",
        }
    }
}

impl fmt::Display for Dimension {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.canonical_name())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UnitScale {
    Nano,
    Micro,
    Milli,
}

impl UnitScale {
    fn multiplier(self) -> i64 {
        match self {
            Self::Nano => 1,
            Self::Micro => 1_000,
            Self::Milli => 1_000_000,
        }
    }

    fn canonical_name(self) -> &'static str {
        match self {
            Self::Nano => "nano",
            Self::Micro => "micro",
            Self::Milli => "milli",
        }
    }
}

impl fmt::Display for UnitScale {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.canonical_name())
    }
}

#[derive(Clone, Copy, Debug, Error, Eq, PartialEq)]
pub enum QuantityError {
    #[error("quantity magnitude is out of representable range")]
    Overflow,
    #[error("arithmetic requires equal dimensions: {expected} vs {actual}")]
    DimensionMismatch {
        expected: Dimension,
        actual: Dimension,
    },
    #[error("arithmetic requires equal unit scales: {expected} vs {actual}")]
    ScaleMismatch {
        expected: UnitScale,
        actual: UnitScale,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Quantity {
    magnitude_nano: i64,
    dimension: Dimension,
    scale: UnitScale,
}

impl Quantity {
    pub fn new(
        magnitude: i64,
        dimension: Dimension,
        scale: UnitScale,
    ) -> Result<Self, QuantityError> {
        let scaled = i128::from(magnitude) * i128::from(scale.multiplier());
        if scaled < i128::from(i64::MIN) || scaled > i128::from(i64::MAX) {
            return Err(QuantityError::Overflow);
        }
        Ok(Self {
            magnitude_nano: scaled as i64,
            dimension,
            scale,
        })
    }

    pub fn dimension(&self) -> Dimension {
        self.dimension
    }

    pub fn scale(&self) -> UnitScale {
        self.scale
    }

    pub fn checked_add(&self, other: &Self) -> Result<Self, QuantityError> {
        if self.dimension != other.dimension {
            return Err(QuantityError::DimensionMismatch {
                expected: self.dimension,
                actual: other.dimension,
            });
        }
        if self.scale != other.scale {
            return Err(QuantityError::ScaleMismatch {
                expected: self.scale,
                actual: other.scale,
            });
        }
        self.magnitude_nano
            .checked_add(other.magnitude_nano)
            .map(|magnitude_nano| Self {
                magnitude_nano,
                dimension: self.dimension,
                scale: self.scale,
            })
            .ok_or(QuantityError::Overflow)
    }
}

pub struct ReservoirState {
    internal_energy_microjoule: i64,
    heat_capacity_microjoule_per_millikelvin: i64,
}

impl ReservoirState {
    pub fn internal_energy_microjoule(&self) -> i64 {
        self.internal_energy_microjoule
    }

    pub fn heat_capacity_microjoule_per_millikelvin(&self) -> i64 {
        self.heat_capacity_microjoule_per_millikelvin
    }

    pub fn new(
        internal_energy_microjoule: i64,
        heat_capacity_microjoule_per_millikelvin: i64,
    ) -> Self {
        Self {
            internal_energy_microjoule,
            heat_capacity_microjoule_per_millikelvin,
        }
    }

    fn canonical_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"reservoir-v1");
        bytes.extend_from_slice(&self.internal_energy_microjoule.to_be_bytes());
        bytes.extend_from_slice(&self.heat_capacity_microjoule_per_millikelvin.to_be_bytes());
        bytes
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StateHash([u8; 32]);

impl StateHash {
    pub fn of(state: &ReservoirState) -> Self {
        let digest = Sha256::digest(state.canonical_bytes());
        Self(digest.into())
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}
