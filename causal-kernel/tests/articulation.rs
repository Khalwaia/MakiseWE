//! Phase 2 slice 4 evidence: data-driven articulation with declared
//! joint limits and motor torque ports.
//!
//! Unit conventions: joint angle and angular velocity in microradians
//! (per second), driven-segment inertia in mg·m², torque in nJ/µrad
//! (= 1e-3 N·m). With these scales the work–energy identity
//! ΔE_rot = τ·Δθ holds as a pure integer product on every one-second
//! interval boundary — verified below against hand-derived
//! E = ½Iω² values, never against the production algorithm.

use makise_causal_kernel::{
    ArticulatedBody, ArticulationError, JointSpec, MorphotypeDefinition, MotorTorqueProposal,
};

fn human() -> MorphotypeDefinition {
    let json = include_str!("../../contracts/fixtures/morphotypes/human-minimal.json");
    MorphotypeDefinition::from_fixture(json).expect("human package")
}

fn neko() -> MorphotypeDefinition {
    let json = include_str!("../../contracts/fixtures/morphotypes/neko-minimal.json");
    MorphotypeDefinition::from_fixture(json).expect("neko package")
}

#[test]
fn skeleton_binds_declared_joints_from_morphotype_data() {
    // One shared code path consumes both packages; every difference
    // below comes from fixture data, never from morphotype branching.
    let human = ArticulatedBody::from_definition(&human()).expect("human skeleton");
    assert_eq!(human.joint_count(), 4);
    assert_eq!(human.joint(0).limit_min_urad, -300_000);
    assert_eq!(human.joint(0).limit_max_urad, 2_000_000);
    assert_eq!(human.joint(0).from_anatomy_node, "thigh-left");
    assert_eq!(human.joint(1).to_anatomy_node, "thigh-left");
    assert_eq!(human.angle_urad(0), Ok(0));
    assert_eq!(human.angular_velocity_urad_per_s(0), Ok(0));

    let neko = ArticulatedBody::from_definition(&neko()).expect("neko skeleton");
    assert_eq!(neko.joint_count(), 5, "hind limb pairs plus tail base");
    // Edge order is deterministic: the tail base edge precedes limbs.
    assert_eq!(neko.joint(0).from_anatomy_node, "tail-vertebrae");
    assert_eq!(neko.joint(0).limit_min_urad, -1_500_000);
    assert_eq!(neko.joint(0).limit_max_urad, 1_500_000);
    assert_ne!(
        neko.joint(1).driven_inertia_mgm2,
        human.joint(0).driven_inertia_mgm2,
        "morphotypes declare their own segment inertias"
    );
}

#[test]
fn first_torque_step_matches_hand_derived_kinematics_and_work() {
    // Independent anchors: I = 1 kg·m², τ = 1000 units = 1 N·m ⇒
    // α = 1 rad/s², so after 1 s: ω₁ = 1 rad/s = 1_000_000 µrad/s and
    // Δθ = ω₀ + α/2 = 500_000 µrad. Work = τ·Δθ = 0.5 J; E_rot = ½Iω²
    // = 0.5 J. Both land in whole nanojoules.
    let body = ArticulatedBody::from_definition(&human()).expect("human skeleton");
    let step = body
        .apply_torque_proposal(&MotorTorqueProposal::new(0, 1_000))
        .expect("within hip limits");
    assert_eq!(step.delta_angular_velocity_urad_per_s(), 1_000_000);
    assert_eq!(step.delta_angle_urad(), 500_000);
    assert_eq!(step.work_done_nj(), 500_000_000);

    let moved = step.into_next();
    assert_eq!(moved.angle_urad(0), Ok(500_000));
    assert_eq!(moved.angular_velocity_urad_per_s(0), Ok(1_000_000));
    assert_eq!(
        moved.rotational_energy_nj().expect("representable"),
        500_000_000
    );
}

#[test]
fn repeated_steps_accumulate_energy_exactly_equal_to_cumulative_work() {
    let body = ArticulatedBody::from_definition(&human()).expect("human skeleton");
    let proposal = MotorTorqueProposal::new(0, 1_000);

    let first = body.apply_torque_proposal(&proposal).expect("in limits");
    let second = first
        .next()
        .apply_torque_proposal(&proposal)
        .expect("in limits");

    // Second interval: Δθ = ω₁ + Δω/2 = 1.5e6 µrad, W₂ = 1.5 J.
    assert_eq!(second.delta_angle_urad(), 1_500_000);
    assert_eq!(second.work_done_nj(), 1_500_000_000);
    assert_eq!(
        second.next().rotational_energy_nj().expect("representable"),
        first.work_done_nj() + second.work_done_nj(),
        "ΔE_rot equals cumulative work exactly across both boundaries"
    );
    assert_eq!(
        second.next().rotational_energy_nj().expect("representable"),
        2_000_000_000,
        "½ · 1 kg·m² · (2 rad/s)²"
    );
}

#[test]
fn limit_violation_is_rejected_without_clamping() {
    // τ = 3e6 units = 3 kN·m for 1 s would swing the thigh by 1500 rad:
    // far outside the declared hip range, so the proposal must be
    // rejected and the body must stay untouched.
    let body = ArticulatedBody::from_definition(&human()).expect("human skeleton");
    let before = body.angle_urad(0);
    let error = body
        .apply_torque_proposal(&MotorTorqueProposal::new(0, 3_000_000))
        .expect_err("free swing exceeds the declared range");
    assert!(matches!(
        error,
        ArticulationError::JointLimitExceeded { joint_index: 0 }
    ));
    assert_eq!(body.angle_urad(0), before, "no silent clamp");
}

#[test]
fn unknown_joint_index_is_typed() {
    let body = ArticulatedBody::from_definition(&human()).expect("human skeleton");
    let error = body
        .apply_torque_proposal(&MotorTorqueProposal::new(4, 1_000))
        .expect_err("the human skeleton declares exactly four joints");
    assert!(matches!(
        error,
        ArticulationError::UnknownJointIndex { joint_index: 4 }
    ));
}

#[test]
fn non_representable_inertia_ratio_is_rejected() {
    // Δω = τ·10⁹/I must be a whole microradian per second; a prime-ish
    // inertia that divides no multiple of the torque leaves the step
    // outside the integer model's validity range.
    let spec = JointSpec::new("a", "b", -1_000_000, 1_000_000, 999_983).expect("valid spec");
    let body = ArticulatedBody::from_joint_specs(vec![spec]).expect("skeleton");
    let error = body
        .apply_torque_proposal(&MotorTorqueProposal::new(0, 1_000))
        .expect_err("τ·10⁹ is not divisible by this inertia");
    assert!(matches!(error, ArticulationError::NonRepresentableStep));
}

#[test]
fn neutral_pose_must_lie_inside_declared_limits() {
    let spec = JointSpec::new("a", "b", 100_000, 1_000_000, 1_000_000).expect("valid spec");
    let error = ArticulatedBody::from_joint_specs(vec![spec])
        .expect_err("neutral angle 0 lies outside [100000, 1000000]");
    assert!(matches!(
        error,
        ArticulationError::NeutralPoseOutsideLimits { joint_index: 0 }
    ));
}

#[test]
fn empty_skeleton_is_rejected_typed() {
    let error = ArticulatedBody::from_joint_specs(Vec::new())
        .expect_err("an articulated body needs at least one joint");
    assert!(matches!(error, ArticulationError::NoArticulatedJoints));
}
