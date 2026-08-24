use thiserror::Error;

use crate::rigid_body::{GRAVITY_NM_PER_S2, RigidBody};

#[derive(Clone, Copy, Debug, Error, Eq, PartialEq)]
pub enum ContactError {
    #[error("grasp requires an active contact")]
    GraspRequiresContact,
    #[error("grip force cannot sustain the held weight within the friction cone")]
    FrictionInfeasible,
    #[error("collider or grasp parameters are outside declared validity range")]
    InvalidParameters,
    #[error("friction product is not a whole number of force units")]
    NonRepresentableFriction,
    #[error("collision impulse quotient is not a whole velocity unit")]
    NonRepresentableResponse,
    #[error("checked arithmetic overflow in contact mechanics")]
    Overflow,
}

/// Axis-aligned bounding-box collider centred on the body origin, in
/// nanometres. Axis-aligned extents keep every contact normal an exact
/// ±unit axis vector under integer arithmetic.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BoxCollider {
    half_extents_nm: [i64; 3],
}

impl BoxCollider {
    pub fn new(half_extents_nm: [i64; 3]) -> Result<Self, ContactError> {
        if half_extents_nm.iter().any(|extent| *extent <= 0) {
            return Err(ContactError::InvalidParameters);
        }
        Ok(Self { half_extents_nm })
    }

    pub fn half_extents_nm(&self) -> [i64; 3] {
        self.half_extents_nm
    }
}

/// Typed proposal output of a body-body overlap: the minimum-translation
/// axis carries the exact ±unit normal and the penetration depth; all
/// three axis overlaps are reported as the full manifold.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ContactManifold {
    normal: [i64; 3],
    penetration_nm: i64,
    overlaps_nm: [i64; 3],
}

impl ContactManifold {
    pub fn normal(&self) -> [i64; 3] {
        self.normal
    }
    pub fn penetration_nm(&self) -> i64 {
        self.penetration_nm
    }
    pub fn overlaps_nm(&self) -> [i64; 3] {
        self.overlaps_nm
    }
}

/// Proposes a contact between two origin-centred box colliders.
/// `Ok(None)` means genuinely separated bodies; `Ok(Some(..))` carries
/// the deterministic minimum-penetration manifold (ties break toward
/// the lowest axis index, normal points from `b` toward `a`).
pub fn contact_proposal(
    a: &crate::rigid_body::RigidBody,
    a_collider: &BoxCollider,
    b: &crate::rigid_body::RigidBody,
    b_collider: &BoxCollider,
) -> Result<Option<ContactManifold>, ContactError> {
    let a_position = a.position_nm();
    let b_position = b.position_nm();
    let mut overlaps = [0i64; 3];
    for axis in 0..3 {
        let a_max = i128::from(a_position[axis]) + i128::from(a_collider.half_extents_nm[axis]);
        let a_min = i128::from(a_position[axis]) - i128::from(a_collider.half_extents_nm[axis]);
        let b_max = i128::from(b_position[axis]) + i128::from(b_collider.half_extents_nm[axis]);
        let b_min = i128::from(b_position[axis]) - i128::from(b_collider.half_extents_nm[axis]);
        let overlap = a_max.min(b_max) - a_min.max(b_min);
        overlaps[axis] = overlap.try_into().map_err(|_| ContactError::Overflow)?;
    }
    if overlaps.iter().any(|overlap| *overlap <= 0) {
        return Ok(None);
    }
    let mut normal_axis = 0;
    for axis in 1..3 {
        if overlaps[axis] < overlaps[normal_axis] {
            normal_axis = axis;
        }
    }
    let sign = if a_position[normal_axis] >= b_position[normal_axis] {
        1
    } else {
        -1
    };
    let mut normal = [0i64; 3];
    normal[normal_axis] = sign;
    Ok(Some(ContactManifold {
        normal,
        penetration_nm: overlaps[normal_axis],
        overlaps_nm: overlaps,
    }))
}

/// Proposed grip cause with declared quantities: the normal pressing
/// force in mg·nm/s² (= 1e-15 N) and the dimensionless pad friction
/// coefficient in micro fixed point (µ stored as µ·1e6).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GraspRequest {
    normal_force_mgnm_per_s2: i64,
    friction_coefficient_micro: i64,
}

impl GraspRequest {
    pub fn new(
        normal_force_mgnm_per_s2: i64,
        friction_coefficient_micro: i64,
    ) -> Result<Self, ContactError> {
        if normal_force_mgnm_per_s2 < 0 || friction_coefficient_micro < 0 {
            return Err(ContactError::InvalidParameters);
        }
        Ok(Self {
            normal_force_mgnm_per_s2,
            friction_coefficient_micro,
        })
    }

    pub fn normal_force_mgnm_per_s2(&self) -> i64 {
        self.normal_force_mgnm_per_s2
    }
    pub fn friction_coefficient_micro(&self) -> i64 {
        self.friction_coefficient_micro
    }
}

/// Exact feasibility report of a proposed grasp: supported weight versus
/// the maximum friction force the cone allows, both in mg·nm/s².
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GraspAssessment {
    weight_force_mgnm_per_s2: i64,
    max_friction_force_mgnm_per_s2: i64,
}

impl GraspAssessment {
    pub fn weight_force_mgnm_per_s2(&self) -> i64 {
        self.weight_force_mgnm_per_s2
    }
    pub fn max_friction_force_mgnm_per_s2(&self) -> i64 {
        self.max_friction_force_mgnm_per_s2
    }
}

/// Validates a proposed grasp against the actual contact and the held
/// body: a positive-overlap manifold must exist, and the friction cone
/// µ·N ≥ m·g must close exactly (cross-multiplied in integers, never
/// rounded).
pub fn grasp_proposal(
    contact: Option<&ContactManifold>,
    held: &crate::rigid_body::RigidBody,
    request: &GraspRequest,
) -> Result<GraspAssessment, ContactError> {
    match contact {
        Some(manifold) if manifold.penetration_nm > 0 => {}
        _ => return Err(ContactError::GraspRequiresContact),
    }
    let weight_force = i128::from(held.mass_mg())
        .checked_mul(i128::from(GRAVITY_NM_PER_S2))
        .ok_or(ContactError::Overflow)?;
    let weight_force: i64 = weight_force
        .try_into()
        .map_err(|_| ContactError::Overflow)?;

    let available_scaled = i128::from(request.friction_coefficient_micro)
        .checked_mul(i128::from(request.normal_force_mgnm_per_s2))
        .ok_or(ContactError::Overflow)?;
    // Cone closure is decided by exact cross-multiplied comparison;
    // rounding never participates in the verdict.
    if available_scaled < i128::from(weight_force) * 1_000_000 {
        return Err(ContactError::FrictionInfeasible);
    }
    // The assessment reports whole force units; a feasible but fractional
    // µ·N product is outside the declared reporting validity range.
    if available_scaled % 1_000_000 != 0 {
        return Err(ContactError::NonRepresentableFriction);
    }
    let max_friction_force: i64 = (available_scaled / 1_000_000)
        .try_into()
        .map_err(|_| ContactError::Overflow)?;

    Ok(GraspAssessment {
        weight_force_mgnm_per_s2: weight_force,
        max_friction_force_mgnm_per_s2: max_friction_force,
    })
}

/// Physical holding projection: `Held` only while the current contact
/// persists and the last assessment keeps the cone closed. This is a
/// stateless read of facts, never a stored flag; institutional
/// possession and title claims are separate state kinds (INVARIANTS §68)
/// and are not derived here.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HoldState {
    Held,
    Released,
}

pub fn hold_projection(
    contact: Option<&ContactManifold>,
    assessment: &GraspAssessment,
) -> HoldState {
    match contact {
        Some(manifold)
            if manifold.penetration_nm > 0
                && assessment.max_friction_force_mgnm_per_s2
                    >= assessment.weight_force_mgnm_per_s2 =>
        {
            HoldState::Held
        }
        _ => HoldState::Released,
    }
}

/// Proposed collision-response cause: the restitution coefficient in
/// micro fixed point, where 0 is perfectly plastic and 1_000_000 is
/// perfectly elastic.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CollisionResponseProposal {
    restitution_micro: i64,
}

impl CollisionResponseProposal {
    pub fn new(restitution_micro: i64) -> Result<Self, ContactError> {
        if !(0..=1_000_000).contains(&restitution_micro) {
            return Err(ContactError::InvalidParameters);
        }
        Ok(Self { restitution_micro })
    }

    pub fn restitution_micro(&self) -> i64 {
        self.restitution_micro
    }
}

/// Validated outcome of one collision resolution: impulse magnitude with
/// the resulting body states plus exact kinetic energies on both sides
/// of the interval boundary. Pair momentum is conserved bit-exact for
/// every restitution; kinetic energy is conserved exactly at e = 1 and
/// monotonically decreases otherwise.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CollisionResolution {
    normal: [i64; 3],
    impulse_mg_nm_per_s: i64,
    next_a: RigidBody,
    next_b: RigidBody,
    kinetic_energy_before_nj: i64,
    kinetic_energy_after_nj: i64,
}

impl CollisionResolution {
    pub fn normal(&self) -> [i64; 3] {
        self.normal
    }
    pub fn impulse_mg_nm_per_s(&self) -> i64 {
        self.impulse_mg_nm_per_s
    }
    pub fn next_a(&self) -> &RigidBody {
        &self.next_a
    }
    pub fn next_b(&self) -> &RigidBody {
        &self.next_b
    }
    pub fn kinetic_energy_before_nj(&self) -> i64 {
        self.kinetic_energy_before_nj
    }
    pub fn kinetic_energy_after_nj(&self) -> i64 {
        self.kinetic_energy_after_nj
    }
}

fn translational_kinetic_energy_nj(body: &RigidBody) -> Result<i64, ContactError> {
    const DENOMINATOR: i128 = 2_000_000_000_000_000;
    let mut squared_sum = 0i128;
    for velocity in body.velocity_nm_per_s() {
        squared_sum += i128::from(velocity) * i128::from(velocity);
    }
    let numerator = squared_sum * i128::from(body.mass_mg());
    if numerator % DENOMINATOR != 0 {
        return Err(ContactError::NonRepresentableResponse);
    }
    (numerator / DENOMINATOR)
        .try_into()
        .map_err(|_| ContactError::Overflow)
}

/// Resolves one overlapping contact: an impulse along the manifold normal
/// when the bodies approach each other, then a mass-split positional
/// correction. The correction gives `a` floor(d·m_b/(m_a+m_b)) of the
/// depth and `b` the exact complement, so the split stays deterministic
/// with at most one nanometre of declared bias instead of a silent
/// rounding rule. Angular state is untouched — contacts act through the
/// centre of mass this slice, mirroring torque-free gravity.
pub fn resolve_collision(
    a: &RigidBody,
    a_collider: &BoxCollider,
    b: &RigidBody,
    b_collider: &BoxCollider,
    proposal: &CollisionResponseProposal,
) -> Result<Option<CollisionResolution>, ContactError> {
    let Some(manifold) = contact_proposal(a, a_collider, b, b_collider)? else {
        return Ok(None);
    };
    let axis = manifold
        .normal()
        .iter()
        .position(|&component| component != 0)
        .ok_or(ContactError::Overflow)?;
    let sign = i128::from(manifold.normal()[axis]);
    let mass_a = i128::from(a.mass_mg());
    let mass_b = i128::from(b.mass_mg());

    let relative_normal =
        (i128::from(a.velocity_nm_per_s()[axis]) - i128::from(b.velocity_nm_per_s()[axis])) * sign;
    let mut velocity_delta_a = 0i64;
    let mut velocity_delta_b = 0i64;
    let mut impulse = 0i64;
    if relative_normal < 0 {
        // Δv_a = s·(1+e)·(−v_rel)·m_b / (1e6·(m_a+m_b)) in mg-based
        // units, where an impulse divided by mass already carries nm/s;
        // fractional quotients are typed rejections, never rounds.
        let scale = i128::from(1_000_000 + proposal.restitution_micro);
        let denominator = i128::from(1_000_000) * (mass_a + mass_b);
        let closing = -relative_normal;
        let numerator_a = scale
            .checked_mul(closing)
            .and_then(|product| product.checked_mul(mass_b))
            .ok_or(ContactError::Overflow)?;
        let numerator_b = scale
            .checked_mul(closing)
            .and_then(|product| product.checked_mul(mass_a))
            .ok_or(ContactError::Overflow)?;
        if numerator_a % denominator != 0 || numerator_b % denominator != 0 {
            return Err(ContactError::NonRepresentableResponse);
        }
        let magnitude_a: i64 = (numerator_a / denominator)
            .try_into()
            .map_err(|_| ContactError::Overflow)?;
        let magnitude_b: i64 = (numerator_b / denominator)
            .try_into()
            .map_err(|_| ContactError::Overflow)?;
        velocity_delta_a =
            i64::try_from(sign * i128::from(magnitude_a)).map_err(|_| ContactError::Overflow)?;
        velocity_delta_b =
            i64::try_from(-sign * i128::from(magnitude_b)).map_err(|_| ContactError::Overflow)?;
        impulse = i128::from(magnitude_a.abs())
            .checked_mul(mass_a)
            .and_then(|product| product.try_into().ok())
            .ok_or(ContactError::Overflow)?;
    }

    let energy_before = translational_kinetic_energy_nj(a)?
        .checked_add(translational_kinetic_energy_nj(b)?)
        .ok_or(ContactError::Overflow)?;

    let mut position_a = a.position_nm();
    let mut position_b = b.position_nm();
    let mut velocity_a = a.velocity_nm_per_s();
    let mut velocity_b = b.velocity_nm_per_s();

    velocity_a[axis] = i64::try_from(i128::from(velocity_a[axis]) + i128::from(velocity_delta_a))
        .map_err(|_| ContactError::Overflow)?;
    velocity_b[axis] = i64::try_from(i128::from(velocity_b[axis]) + i128::from(velocity_delta_b))
        .map_err(|_| ContactError::Overflow)?;

    // Mass-split de-penetration along the same axis.
    let depth = i128::from(manifold.penetration_nm());
    let split_denominator = mass_a + mass_b;
    let shift_a = depth * mass_b / split_denominator;
    let shift_b = depth - shift_a;
    position_a[axis] = i64::try_from(i128::from(position_a[axis]) + shift_a * sign)
        .map_err(|_| ContactError::Overflow)?;
    position_b[axis] = i64::try_from(i128::from(position_b[axis]) - shift_b * sign)
        .map_err(|_| ContactError::Overflow)?;

    let rebuild = |body: &RigidBody,
                   position: [i64; 3],
                   velocity: [i64; 3]|
     -> Result<RigidBody, ContactError> {
        RigidBody::new(
            body.mass_mg(),
            position,
            velocity,
            body.center_of_mass_offset_nm(),
            body.principal_inertia_mgm2(),
            body.angular_velocity_urad_per_s(),
        )
        .map_err(|_| ContactError::Overflow)
    };
    let next_a = rebuild(a, position_a, velocity_a)?;
    let next_b = rebuild(b, position_b, velocity_b)?;

    let energy_after = translational_kinetic_energy_nj(&next_a)?
        .checked_add(translational_kinetic_energy_nj(&next_b)?)
        .ok_or(ContactError::Overflow)?;

    Ok(Some(CollisionResolution {
        normal: manifold.normal(),
        impulse_mg_nm_per_s: impulse,
        next_a,
        next_b,
        kinetic_energy_before_nj: energy_before,
        kinetic_energy_after_nj: energy_after,
    }))
}
