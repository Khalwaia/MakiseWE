use crate::morphotype::MorphotypeDefinition;
use thiserror::Error;

/// Declared joint data from a morphotype anatomy edge: the driven
/// segment (`from`) articulates around its parent (`to`) within integer
/// microradian limits; `driven_inertia_mgm2` is the segment moment of
/// inertia about this joint in mg·m².
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JointSpec {
    pub from_anatomy_node: String,
    pub to_anatomy_node: String,
    pub limit_min_urad: i64,
    pub limit_max_urad: i64,
    pub driven_inertia_mgm2: i64,
}

impl JointSpec {
    pub fn new(
        from_anatomy_node: impl Into<String>,
        to_anatomy_node: impl Into<String>,
        limit_min_urad: i64,
        limit_max_urad: i64,
        driven_inertia_mgm2: i64,
    ) -> Result<Self, ArticulationError> {
        if limit_min_urad > limit_max_urad || driven_inertia_mgm2 < 1 {
            return Err(ArticulationError::InvalidSpec);
        }
        Ok(Self {
            from_anatomy_node: from_anatomy_node.into(),
            to_anatomy_node: to_anatomy_node.into(),
            limit_min_urad,
            limit_max_urad,
            driven_inertia_mgm2,
        })
    }
}

#[derive(Clone, Copy, Debug, Error, Eq, PartialEq)]
pub enum ArticulationError {
    #[error("joint declaration is outside declared validity range")]
    InvalidSpec,
    #[error("an articulated body needs at least one declared joint")]
    NoArticulatedJoints,
    #[error("neutral angle lies outside declared limits of joint {joint_index}")]
    NeutralPoseOutsideLimits { joint_index: usize },
    #[error("no joint with index {joint_index} in this skeleton")]
    UnknownJointIndex { joint_index: usize },
    #[error("motion would exceed the declared limits of joint {joint_index}")]
    JointLimitExceeded { joint_index: usize },
    #[error("one-second step is not representable in whole microradians")]
    NonRepresentableStep,
    #[error("checked arithmetic overflow in articulation dynamics")]
    Overflow,
}

/// Authoritative articulated-skeleton state: per-joint angle and angular
/// velocity, bound to morphotype-declared joint specifications. The
/// torque unit is nJ/µrad (= 1e-3 N·m), which makes the work–energy
/// identity ΔE_rot = τ·Δθ an exact integer product on every interval
/// boundary.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArticulatedBody {
    joints: Vec<JointState>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct JointState {
    spec: JointSpec,
    angle_urad: i64,
    angular_velocity_urad_per_s: i64,
}

impl ArticulatedBody {
    /// Binds the skeleton of a morphotype definition. Joints appear in
    /// anatomy-edge order and start at the neutral pose (angle 0).
    pub fn from_definition(definition: &MorphotypeDefinition) -> Result<Self, ArticulationError> {
        Self::from_joint_specs(definition.anatomy_joints().to_vec())
    }

    /// Builds a skeleton directly from joint specifications.
    pub fn from_joint_specs(specs: Vec<JointSpec>) -> Result<Self, ArticulationError> {
        if specs.is_empty() {
            return Err(ArticulationError::NoArticulatedJoints);
        }
        let joints = specs
            .into_iter()
            .enumerate()
            .map(|(joint_index, spec)| {
                if spec.limit_min_urad > 0 || spec.limit_max_urad < 0 {
                    return Err(ArticulationError::NeutralPoseOutsideLimits { joint_index });
                }
                Ok(JointState {
                    spec,
                    angle_urad: 0,
                    angular_velocity_urad_per_s: 0,
                })
            })
            .collect::<Result<Vec<_>, ArticulationError>>()?;
        Ok(Self { joints })
    }

    pub fn joint_count(&self) -> usize {
        self.joints.len()
    }

    pub fn joint(&self, joint_index: usize) -> &JointSpec {
        &self.joints[joint_index].spec
    }

    pub fn angle_urad(&self, joint_index: usize) -> Result<i64, ArticulationError> {
        self.joint_state(joint_index).map(|state| state.angle_urad)
    }

    pub fn angular_velocity_urad_per_s(
        &self,
        joint_index: usize,
    ) -> Result<i64, ArticulationError> {
        self.joint_state(joint_index)
            .map(|state| state.angular_velocity_urad_per_s)
    }

    fn joint_state(&self, joint_index: usize) -> Result<&JointState, ArticulationError> {
        self.joints
            .get(joint_index)
            .ok_or(ArticulationError::UnknownJointIndex { joint_index })
    }

    /// Sum of per-joint rotational kinetic energies in nanojoules,
    /// E = Σ I·ω²/2 with exact whole-nanojoule representability.
    pub fn rotational_energy_nj(&self) -> Result<i64, ArticulationError> {
        let mut total = 0i64;
        for state in &self.joints {
            let numerator = i128::from(state.spec.driven_inertia_mgm2)
                .checked_mul(i128::from(state.angular_velocity_urad_per_s))
                .and_then(|product| {
                    product.checked_mul(i128::from(state.angular_velocity_urad_per_s))
                })
                .ok_or(ArticulationError::Overflow)?;
            const DENOMINATOR: i128 = 2_000_000_000;
            if numerator % DENOMINATOR != 0 {
                return Err(ArticulationError::NonRepresentableStep);
            }
            total = total
                .checked_add(
                    (numerator / DENOMINATOR)
                        .try_into()
                        .map_err(|_| ArticulationError::Overflow)?,
                )
                .ok_or(ArticulationError::Overflow)?;
        }
        Ok(total)
    }

    /// Validates a proposed constant torque over the next canonical
    /// second against joint limits and integer representability, without
    /// mutating this body. Kinematics are exact: ω₁ = ω₀ + Δω and
    /// θ₁ = θ₀ + ω₀ + Δω/2, so the work–energy identity holds exactly.
    pub fn apply_torque_proposal(
        &self,
        proposal: &MotorTorqueProposal,
    ) -> Result<MotionStep, ArticulationError> {
        let state = self.joint_state(proposal.joint_index)?;
        // Δω[µrad/s] = τ[nJ/µrad]·1e9 / I[mg·m²]; the scales cancel
        // exactly, leaving an integer quotient when I divides τ·10⁹.
        let scaled = i128::from(proposal.torque_nj_per_urad)
            .checked_mul(1_000_000_000)
            .ok_or(ArticulationError::Overflow)?;
        if scaled % i128::from(state.spec.driven_inertia_mgm2) != 0 {
            return Err(ArticulationError::NonRepresentableStep);
        }
        let delta_angular_velocity: i64 = (scaled / i128::from(state.spec.driven_inertia_mgm2))
            .try_into()
            .map_err(|_| ArticulationError::Overflow)?;
        if delta_angular_velocity % 2 != 0 {
            return Err(ArticulationError::NonRepresentableStep);
        }
        let delta_angle = i128::from(state.angular_velocity_urad_per_s)
            .checked_add(i128::from(delta_angular_velocity) / 2)
            .ok_or(ArticulationError::Overflow)?;
        let delta_angle: i64 = delta_angle
            .try_into()
            .map_err(|_| ArticulationError::Overflow)?;
        let new_angle = i128::from(state.angle_urad)
            .checked_add(i128::from(delta_angle))
            .ok_or(ArticulationError::Overflow)?;
        let new_angle: i64 = new_angle
            .try_into()
            .map_err(|_| ArticulationError::Overflow)?;
        if new_angle < state.spec.limit_min_urad || new_angle > state.spec.limit_max_urad {
            return Err(ArticulationError::JointLimitExceeded {
                joint_index: proposal.joint_index,
            });
        }
        let work_done_nj = i128::from(proposal.torque_nj_per_urad)
            .checked_mul(i128::from(delta_angle))
            .ok_or(ArticulationError::Overflow)?
            .try_into()
            .map_err(|_| ArticulationError::Overflow)?;

        let mut next_joints = self.joints.clone();
        let driven = &mut next_joints[proposal.joint_index];
        driven.angle_urad = new_angle;
        driven.angular_velocity_urad_per_s = state
            .angular_velocity_urad_per_s
            .checked_add(delta_angular_velocity)
            .ok_or(ArticulationError::Overflow)?;

        Ok(MotionStep {
            next: Self {
                joints: next_joints,
            },
            joint_index: proposal.joint_index,
            delta_angle_urad: delta_angle,
            delta_angular_velocity_urad_per_s: delta_angular_velocity,
            work_done_nj,
        })
    }
}

/// Proposed motor cause: constant torque applied to one joint over one
/// canonical second. Negative values drive the opposite direction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MotorTorqueProposal {
    joint_index: usize,
    torque_nj_per_urad: i64,
}

impl MotorTorqueProposal {
    pub fn new(joint_index: usize, torque_nj_per_urad: i64) -> Self {
        Self {
            joint_index,
            torque_nj_per_urad,
        }
    }

    pub fn joint_index(&self) -> usize {
        self.joint_index
    }
    pub fn torque_nj_per_urad(&self) -> i64 {
        self.torque_nj_per_urad
    }
}

/// Validated outcome of one motor step: the moved body plus the exact
/// deltas and the work performed on the closed interval.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MotionStep {
    next: ArticulatedBody,
    joint_index: usize,
    delta_angle_urad: i64,
    delta_angular_velocity_urad_per_s: i64,
    work_done_nj: i64,
}

impl MotionStep {
    pub fn joint_index(&self) -> usize {
        self.joint_index
    }
    pub fn delta_angle_urad(&self) -> i64 {
        self.delta_angle_urad
    }
    pub fn delta_angular_velocity_urad_per_s(&self) -> i64 {
        self.delta_angular_velocity_urad_per_s
    }
    pub fn work_done_nj(&self) -> i64 {
        self.work_done_nj
    }
    pub fn next(&self) -> &ArticulatedBody {
        &self.next
    }
    pub fn into_next(self) -> ArticulatedBody {
        self.next
    }
}
