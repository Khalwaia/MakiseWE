use thiserror::Error;

use crate::rigid_body::GRAVITY_NM_PER_S2;

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
