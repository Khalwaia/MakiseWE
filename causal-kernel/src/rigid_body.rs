use thiserror::Error;

/// Standard gravitational acceleration in nanometres per second squared.
/// 9.80665 m/s² = 9_806_650_000 nm/s².
pub(crate) const GRAVITY_NM_PER_S2: i64 = 9_806_650_000;

#[derive(Clone, Copy, Debug, Error, Eq, PartialEq)]
pub enum RigidBodyError {
    #[error("state is outside declared validity range")]
    OutsideValidityRange,
    #[error("checked arithmetic overflow in rigid body dynamics")]
    Overflow,
}

/// Authoritative metric rigid-body state.
///
/// Unit conventions (exact integer representations):
/// - mass in milligrams (`mg`, 1e-6 kg);
/// - length in nanometres (`nm`, 1e-9 m), time in seconds, y is up;
/// - principal inertia components in milligram square metres
///   (`mg·m²`, 1e-6 kg·m²), diagonal of the inertia tensor about the
///   centre of mass with the body frame axis-aligned to the world frame;
/// - angular velocity in microradians per second (`µrad/s`, 1e-6 rad/s).
///
/// The centre-of-mass offset is the fixed vector from the body-frame
/// origin to the centre of mass; gravity acts through it, so free fall
/// exerts zero torque and angular momentum is conserved exactly.
/// Gyroscopic coupling between non-principal axes stays outside this
/// slice's declared validity range until articulated bodies arrive.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RigidBody {
    mass_mg: i64,
    position_nm: [i64; 3],
    velocity_nm_per_s: [i64; 3],
    center_of_mass_offset_nm: [i64; 3],
    principal_inertia_mgm2: [i64; 3],
    angular_velocity_urad_per_s: [i64; 3],
}

impl RigidBody {
    pub fn new(
        mass_mg: i64,
        position_nm: [i64; 3],
        velocity_nm_per_s: [i64; 3],
        center_of_mass_offset_nm: [i64; 3],
        principal_inertia_mgm2: [i64; 3],
        angular_velocity_urad_per_s: [i64; 3],
    ) -> Result<Self, RigidBodyError> {
        if mass_mg <= 0 || principal_inertia_mgm2.iter().any(|moment| *moment <= 0) {
            return Err(RigidBodyError::OutsideValidityRange);
        }
        Ok(Self {
            mass_mg,
            position_nm,
            velocity_nm_per_s,
            center_of_mass_offset_nm,
            principal_inertia_mgm2,
            angular_velocity_urad_per_s,
        })
    }

    pub fn mass_mg(&self) -> i64 {
        self.mass_mg
    }
    pub fn position_nm(&self) -> [i64; 3] {
        self.position_nm
    }
    pub fn velocity_nm_per_s(&self) -> [i64; 3] {
        self.velocity_nm_per_s
    }
    pub fn center_of_mass_offset_nm(&self) -> [i64; 3] {
        self.center_of_mass_offset_nm
    }
    pub fn principal_inertia_mgm2(&self) -> [i64; 3] {
        self.principal_inertia_mgm2
    }
    pub fn angular_velocity_urad_per_s(&self) -> [i64; 3] {
        self.angular_velocity_urad_per_s
    }

    /// Centre-of-mass position in world coordinates, in nanometres.
    pub fn world_center_of_mass_nm(&self) -> Result<[i64; 3], RigidBodyError> {
        let mut world = [0i64; 3];
        for ((slot, position), offset) in world
            .iter_mut()
            .zip(self.position_nm)
            .zip(self.center_of_mass_offset_nm)
        {
            *slot = i128::from(position)
                .checked_add(i128::from(offset))
                .ok_or(RigidBodyError::Overflow)?
                .try_into()
                .map_err(|_| RigidBodyError::Overflow)?;
        }
        Ok(world)
    }

    /// Linear momentum p = m·v in mg·nm/s (= 1e-15 kg·m/s).
    pub fn linear_momentum_mg_nm_per_s(&self) -> Result<[i64; 3], RigidBodyError> {
        let mut momentum = [0i64; 3];
        for (slot, velocity) in momentum.iter_mut().zip(self.velocity_nm_per_s) {
            *slot = i128::from(self.mass_mg)
                .checked_mul(i128::from(velocity))
                .ok_or(RigidBodyError::Overflow)?
                .try_into()
                .map_err(|_| RigidBodyError::Overflow)?;
        }
        Ok(momentum)
    }

    /// Angular momentum about the centre of mass, L = I·ω per principal
    /// axis, in mg·m²·µrad/s (= 1e-12 kg·m²/s).
    pub fn angular_momentum_mg_m2_urad_per_s(&self) -> Result<[i64; 3], RigidBodyError> {
        let mut momentum = [0i64; 3];
        for ((slot, inertia), spin) in momentum
            .iter_mut()
            .zip(self.principal_inertia_mgm2)
            .zip(self.angular_velocity_urad_per_s)
        {
            *slot = i128::from(inertia)
                .checked_mul(i128::from(spin))
                .ok_or(RigidBodyError::Overflow)?
                .try_into()
                .map_err(|_| RigidBodyError::Overflow)?;
        }
        Ok(momentum)
    }

    /// Total mechanical energy in nanojoules:
    /// KE_trans = Σ m·vᵢ²/2, KE_rot = Σ Iᵢ·ωᵢ²/2, PE = m·g·y.
    /// All arithmetic uses i128 intermediates; a result that is not a
    /// whole number of nanojoules is outside the declared validity range
    /// of the integer model, and any intermediate overflow is typed.
    pub fn total_mechanical_energy_nj(&self) -> Result<i64, RigidBodyError> {
        // KE_trans_nJ = mass_mg · v_nm² / (2 · 10^15):
        // mg→kg ÷1e6, nm²/s²→m²/s² ÷1e18, J→nJ ×1e9.
        const TRANSLATIONAL_DENOMINATOR: i128 = 2_000_000_000_000_000;
        let mut velocity_squared_sum = 0i128;
        for velocity in self.velocity_nm_per_s {
            let squared = i128::from(velocity)
                .checked_mul(i128::from(velocity))
                .ok_or(RigidBodyError::Overflow)?;
            velocity_squared_sum = velocity_squared_sum
                .checked_add(squared)
                .ok_or(RigidBodyError::Overflow)?;
        }
        let translational_num = i128::from(self.mass_mg)
            .checked_mul(velocity_squared_sum)
            .ok_or(RigidBodyError::Overflow)?;
        if translational_num % TRANSLATIONAL_DENOMINATOR != 0 {
            return Err(RigidBodyError::OutsideValidityRange);
        }
        let translational_energy_nj: i64 = (translational_num / TRANSLATIONAL_DENOMINATOR)
            .try_into()
            .map_err(|_| RigidBodyError::Overflow)?;

        // KE_rot_nJ = I_mgm2 · ω_urad² / (2 · 10^9):
        // mg·m²→kg·m² ÷1e6, µrad²/s²→rad²/s² ÷1e12, J→nJ ×1e9.
        const ROTATIONAL_DENOMINATOR: i128 = 2_000_000_000;
        let mut rotational_num = 0i128;
        for axis in 0..3 {
            let squared = i128::from(self.angular_velocity_urad_per_s[axis])
                .checked_mul(i128::from(self.angular_velocity_urad_per_s[axis]))
                .ok_or(RigidBodyError::Overflow)?;
            let product = i128::from(self.principal_inertia_mgm2[axis])
                .checked_mul(squared)
                .ok_or(RigidBodyError::Overflow)?;
            rotational_num = rotational_num
                .checked_add(product)
                .ok_or(RigidBodyError::Overflow)?;
        }
        if rotational_num % ROTATIONAL_DENOMINATOR != 0 {
            return Err(RigidBodyError::OutsideValidityRange);
        }
        let rotational_energy_nj: i64 = (rotational_num / ROTATIONAL_DENOMINATOR)
            .try_into()
            .map_err(|_| RigidBodyError::Overflow)?;

        // PE_nJ = mass_mg · G_nm · y_nm / 10^15:
        // mg→kg ÷1e6, nm→m ÷1e9 twice, J→nJ ×1e9.
        const POTENTIAL_DENOMINATOR: i128 = 1_000_000_000_000_000;
        let potential_num = i128::from(self.mass_mg)
            .checked_mul(i128::from(GRAVITY_NM_PER_S2))
            .ok_or(RigidBodyError::Overflow)?
            .checked_mul(i128::from(self.position_nm[1]))
            .ok_or(RigidBodyError::Overflow)?;
        if potential_num % POTENTIAL_DENOMINATOR != 0 {
            return Err(RigidBodyError::OutsideValidityRange);
        }
        let potential_energy_nj: i64 = (potential_num / POTENTIAL_DENOMINATOR)
            .try_into()
            .map_err(|_| RigidBodyError::Overflow)?;

        translational_energy_nj
            .checked_add(rotational_energy_nj)
            .and_then(|sum| sum.checked_add(potential_energy_nj))
            .ok_or(RigidBodyError::Overflow)
    }

    /// Proposes a one-second contact-free gravity step using exact
    /// constant-acceleration kinematics over the declared interval.
    /// Torque-free by contract: angular state is carried through
    /// unchanged.
    pub fn gravity_proposal(&self) -> Result<GravityProposal, RigidBodyError> {
        let integrate = |position: i64, velocity: i64| -> Result<i64, RigidBodyError> {
            let sum = i128::from(position)
                .checked_add(i128::from(velocity))
                .ok_or(RigidBodyError::Overflow)?;
            sum.try_into().map_err(|_| RigidBodyError::Overflow)
        };
        let new_x = integrate(self.position_nm[0], self.velocity_nm_per_s[0])?;
        let new_vy = self.velocity_nm_per_s[1]
            .checked_sub(GRAVITY_NM_PER_S2)
            .ok_or(RigidBodyError::Overflow)?;
        let new_y = i128::from(self.position_nm[1])
            .checked_add(i128::from(self.velocity_nm_per_s[1]))
            .and_then(|sum| sum.checked_sub(i128::from(GRAVITY_NM_PER_S2) / 2))
            .ok_or(RigidBodyError::Overflow)?;
        let new_y: i64 = new_y.try_into().map_err(|_| RigidBodyError::Overflow)?;
        let new_z = integrate(self.position_nm[2], self.velocity_nm_per_s[2])?;
        Ok(GravityProposal {
            new_position: [new_x, new_y, new_z],
            new_velocity: [self.velocity_nm_per_s[0], new_vy, self.velocity_nm_per_s[2]],
        })
    }
}

#[derive(Clone, Copy, Debug)]
pub struct GravityProposal {
    new_position: [i64; 3],
    new_velocity: [i64; 3],
}

impl GravityProposal {
    pub fn apply(&self, body: &RigidBody) -> RigidBody {
        RigidBody {
            mass_mg: body.mass_mg,
            position_nm: self.new_position,
            velocity_nm_per_s: self.new_velocity,
            center_of_mass_offset_nm: body.center_of_mass_offset_nm,
            principal_inertia_mgm2: body.principal_inertia_mgm2,
            angular_velocity_urad_per_s: body.angular_velocity_urad_per_s,
        }
    }
}
