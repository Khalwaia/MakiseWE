use thiserror::Error;

/// Standard gravitational acceleration in nanometres per second squared.
/// 9.80665 m/s² = 9_806_650_000 nm/s².
const GRAVITY_NM_PER_S2: i64 = 9_806_650_000;

#[derive(Clone, Copy, Debug, Error, Eq, PartialEq)]
pub enum RigidBodyError {
    #[error("state is outside declared validity range")]
    OutsideValidityRange,
    #[error("checked arithmetic overflow in rigid body dynamics")]
    Overflow,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RigidBody {
    mass_mg: i64,
    position_nm: [i64; 2],
    velocity_nm_per_s: [i64; 2],
}

impl RigidBody {
    pub fn new(
        mass_mg: i64,
        position_nm: [i64; 2],
        velocity_nm_per_s: [i64; 2],
    ) -> Result<Self, RigidBodyError> {
        if mass_mg <= 0 {
            return Err(RigidBodyError::OutsideValidityRange);
        }
        Ok(Self {
            mass_mg,
            position_nm,
            velocity_nm_per_s,
        })
    }

    pub fn mass_mg(&self) -> i64 {
        self.mass_mg
    }
    pub fn position_nm(&self) -> [i64; 2] {
        self.position_nm
    }
    pub fn velocity_nm_per_s(&self) -> [i64; 2] {
        self.velocity_nm_per_s
    }

    /// Total mechanical energy in nanojoules.
    /// KE = ½·m·v² (m in kg, v in m/s → J → ×1e9 for nJ).
    /// PE = m·g·h (same unit conversion).
    /// All arithmetic uses i128 intermediates to avoid intermediate overflow;
    /// final result must fit i64 or Overflow is returned.
    pub fn total_mechanical_energy_nj(&self) -> Result<i64, RigidBodyError> {
        // Convert to SI base for computation:
        //   m_kg = mass_mg / 1e6 (mg → kg)
        //   v_ms = v_nm_per_s / 1e9
        //   h_m  = y_nm / 1e9
        // Energy (J) = ½·(mg/1e6)·(v/1e9)² + (mg/1e6)·9.80665·(y/1e9)
        //            = ½·mg·v²/(1e6·1e18) + mg·G·y/(1e6·1e9)
        // In nanojoules: ×1e9
        // KE_nJ = ½·mg·v²/(1e6·1e9) = mg·v²/2e15
        // PE_nJ = mg·G·y/(1e6) = mg·9_806_650_000·y/1e6... too large.

        // Use exact rational arithmetic via i128:
        // KE numerator = mass_mg · v² (in nm²/s² units), denominator = 2 · 10^24
        let vx = i128::from(self.velocity_nm_per_s[0]);
        let vy = i128::from(self.velocity_nm_per_s[1]);
        let vx_squared = vx.checked_mul(vx).ok_or(RigidBodyError::Overflow)?;
        let vy_squared = vy.checked_mul(vy).ok_or(RigidBodyError::Overflow)?;
        let v_squared = vx_squared
            .checked_add(vy_squared)
            .ok_or(RigidBodyError::Overflow)?;
        let ke_num = i128::from(self.mass_mg)
            .checked_mul(v_squared)
            .ok_or(RigidBodyError::Overflow)?;
        // KE_nJ = num / (2 * 10^15) — because mg→kg is /1e6, nm²/s² → m²/s² is /1e18,
        // then J → nJ is ×1e9, so total denominator = 1e6·1e18/1e9 = 1e15, times 2.
        const KE_DENOMINATOR: i128 = 2_000_000_000_000_000;
        if ke_num % KE_DENOMINATOR != 0 {
            return Err(RigidBodyError::OutsideValidityRange);
        }
        let kinetic_energy_nj: i64 = (ke_num / KE_DENOMINATOR)
            .try_into()
            .map_err(|_| RigidBodyError::Overflow)?;

        // PE numerator = mass_mg · G · y (nm²/s²), same conversion as above but no factor ½.
        const PE_DENOMINATOR: i128 = 1_000_000_000_000_000;
        let pe_num = i128::from(self.mass_mg)
            .checked_mul(i128::from(GRAVITY_NM_PER_S2))
            .ok_or(RigidBodyError::Overflow)?
            .checked_mul(i128::from(self.position_nm[1]))
            .ok_or(RigidBodyError::Overflow)?;
        if pe_num % PE_DENOMINATOR != 0 {
            return Err(RigidBodyError::OutsideValidityRange);
        }
        let potential_energy_nj: i64 = (pe_num / PE_DENOMINATOR)
            .try_into()
            .map_err(|_| RigidBodyError::Overflow)?;

        kinetic_energy_nj
            .checked_add(potential_energy_nj)
            .ok_or(RigidBodyError::Overflow)
    }

    /// Proposes a one-second contact-free gravity step.
    /// Uses exact semi-implicit Euler with integer kinematics.
    pub fn gravity_proposal(&self) -> Result<GravityProposal, RigidBodyError> {
        let new_vy = self.velocity_nm_per_s[1]
            .checked_sub(GRAVITY_NM_PER_S2)
            .ok_or(RigidBodyError::Overflow)?;
        let new_y = i128::from(self.position_nm[1])
            .checked_add(i128::from(self.velocity_nm_per_s[1]))
            .and_then(|sum| sum.checked_sub(i128::from(GRAVITY_NM_PER_S2) / 2))
            .ok_or(RigidBodyError::Overflow)?;
        let new_y: i64 = new_y.try_into().map_err(|_| RigidBodyError::Overflow)?;
        Ok(GravityProposal {
            new_position: [self.position_nm[0], new_y],
            new_velocity: [self.velocity_nm_per_s[0], new_vy],
        })
    }
}

#[derive(Clone, Copy, Debug)]
pub struct GravityProposal {
    new_position: [i64; 2],
    new_velocity: [i64; 2],
}

impl GravityProposal {
    pub fn apply(&self, body: &RigidBody) -> RigidBody {
        RigidBody {
            mass_mg: body.mass_mg,
            position_nm: self.new_position,
            velocity_nm_per_s: self.new_velocity,
        }
    }
}
