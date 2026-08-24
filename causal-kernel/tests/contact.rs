//! Phase 2 slice 3 evidence: contacts, grasp friction feasibility, and
//! the physical hold projection.
//!
//! Expected values are hand-derived against the declared integer units:
//! force in mg·nm/s² (= 1e-15 N), length in nm, dimensionless friction
//! coefficient in micro fixed point (1e-6). The friction cone test uses
//! cross-multiplied exact integers: μ·N ≥ m·g holds exactly at the
//! boundary grip force and fails one unit below it.

use makise_causal_kernel::{
    BoxCollider, ContactError, ContactManifold, GraspRequest, HoldState, RigidBody,
    contact_proposal, grasp_proposal, hold_projection,
};

const GRAVITY_NM_PER_S2: i64 = 9_806_650_000;

fn body(position_nm: [i64; 3]) -> RigidBody {
    RigidBody::new(
        2_000_000,
        position_nm,
        [0; 3],
        [0; 3],
        [1_000, 1_000, 1_000],
        [0; 3],
    )
    .expect("valid rigid body")
}

fn collider(half_extents_nm: [i64; 3]) -> BoxCollider {
    BoxCollider::new(half_extents_nm).expect("valid collider")
}

fn contact(
    a: &RigidBody,
    a_collider: &BoxCollider,
    b: &RigidBody,
    b_collider: &BoxCollider,
) -> Option<ContactManifold> {
    contact_proposal(a, a_collider, b, b_collider).expect("fixture geometry is valid")
}

#[test]
fn collider_rejects_nonpositive_half_extents() {
    let error = BoxCollider::new([100_000_000, 0, 50_000_000])
        .expect_err("zero extent cannot enclose a physical object");
    assert!(matches!(error, ContactError::InvalidParameters));

    let error = BoxCollider::new([100_000_000, -1, 50_000_000])
        .expect_err("negative extent is outside validity range");
    assert!(matches!(error, ContactError::InvalidParameters));
}

#[test]
fn separated_bodies_produce_no_contact() {
    // Half extents 100 mm + 80 mm along x with centres 300 mm apart:
    // gap = 300 − 180 = 120 mm of separation.
    let a = body([0, 0, 0]);
    let b = body([300_000_000, 0, 0]);
    assert!(
        contact(
            &a,
            &collider([100_000_000, 50_000_000, 50_000_000]),
            &b,
            &collider([80_000_000, 40_000_000, 40_000_000])
        )
        .is_none()
    );
}

#[test]
fn overlap_produces_manifold_on_minimum_translation_axis() {
    // Independent anchor: boxes overlap by min(100+80, …) − max(−100, 70)
    // = 30 mm along x, 80 mm along y and z, so x carries the least
    // penetration and b sits on the positive side of a.
    let a = body([0, 0, 0]);
    let b = body([150_000_000, 0, 0]);
    let manifold = contact(
        &a,
        &collider([100_000_000, 50_000_000, 50_000_000]),
        &b,
        &collider([80_000_000, 40_000_000, 40_000_000]),
    )
    .expect("overlapping boxes are in contact");
    assert_eq!(manifold.penetration_nm(), 30_000_000);
    assert_eq!(manifold.normal(), [-1, 0, 0], "normal pushes a away from b");
    assert_eq!(manifold.overlaps_nm(), [30_000_000, 80_000_000, 80_000_000]);

    // Swapping argument order mirrors the normal deterministically.
    let mirrored = contact(
        &b,
        &collider([80_000_000, 40_000_000, 40_000_000]),
        &a,
        &collider([100_000_000, 50_000_000, 50_000_000]),
    )
    .expect("contact is symmetric");
    assert_eq!(mirrored.normal(), [1, 0, 0]);
    assert_eq!(mirrored.penetration_nm(), 30_000_000);
}

#[test]
fn equal_penetration_ties_break_by_lowest_axis() {
    // Symmetric diagonal placement gives equal 40 mm overlaps on all
    // axes; axis 0 must win and the sign must follow centre order.
    let a = body([0, 0, 0]);
    let b = body([60_000_000, 60_000_000, 60_000_000]);
    let unit = [50_000_000, 50_000_000, 50_000_000];
    let manifold = contact(&a, &collider(unit), &b, &collider(unit))
        .expect("overlapping cubes are in contact");
    assert_eq!(manifold.normal(), [-1, 0, 0]);
    assert_eq!(manifold.penetration_nm(), 40_000_000);

    let flipped = contact(&b, &collider(unit), &a, &collider(unit))
        .expect("overlapping cubes are in contact");
    assert_eq!(flipped.normal(), [1, 0, 0]);
}

#[test]
fn grasp_without_contact_is_rejected_typed() {
    let pot = body([0, 0, 0]);
    let request =
        GraspRequest::new(39_226_600_000_000_000, 500_000).expect("valid grip parameters");
    let error = grasp_proposal(None, &pot, &request)
        .expect_err("grasping through empty space has no cause");
    assert!(matches!(error, ContactError::GraspRequiresContact));
}

#[test]
fn grasp_request_rejects_negative_grip_parameters() {
    let error = GraspRequest::new(-1, 500_000)
        .expect_err("negative normal force cannot press surfaces together");
    assert!(matches!(error, ContactError::InvalidParameters));

    let error = GraspRequest::new(39_226_600_000_000_000, -1)
        .expect_err("negative friction coefficient is outside validity range");
    assert!(matches!(error, ContactError::InvalidParameters));
}

#[test]
fn friction_cone_boundary_is_exact() {
    // Independent anchors: weight = m·g = 2 kg · 9.80665 m/s² =
    // 19.6133 N = 19_613_300_000_000_000 mg·nm/s². With μ = 0.5 the
    // cone closes exactly at N = W/μ = 39_226_600_000_000_000; one unit
    // less leaves the object unsupported.
    let contact = contact_fixture();
    let pot = body([0, 0, 0]);

    let boundary = GraspRequest::new(39_226_600_000_000_000, 500_000)
        .and_then(|request| grasp_proposal(Some(&contact), &pot, &request))
        .expect("boundary grip force closes the friction cone exactly");
    assert_eq!(
        boundary.weight_force_mgnm_per_s2(),
        2_000_000 * GRAVITY_NM_PER_S2
    );
    assert_eq!(
        boundary.max_friction_force_mgnm_per_s2(),
        boundary.weight_force_mgnm_per_s2()
    );

    let short = GraspRequest::new(39_226_600_000_000_000 - 1, 500_000)
        .and_then(|request| grasp_proposal(Some(&contact), &pot, &request))
        .expect_err("one unit below the cone boundary cannot hold the weight");
    assert!(matches!(short, ContactError::FrictionInfeasible));
}

#[test]
fn zero_grip_force_or_frictionless_pads_cannot_hold_any_mass() {
    let contact = contact_fixture();
    let pot = body([0, 0, 0]);

    let request = GraspRequest::new(0, 500_000).expect("zero force is representable");
    let error = grasp_proposal(Some(&contact), &pot, &request)
        .expect_err("weight is always positive for a valid body");
    assert!(matches!(error, ContactError::FrictionInfeasible));

    let request =
        GraspRequest::new(39_226_600_000_000_000, 0).expect("frictionless pads are representable");
    let error = grasp_proposal(Some(&contact), &pot, &request)
        .expect_err("μ = 0 closes the cone for any weight above zero");
    assert!(matches!(error, ContactError::FrictionInfeasible));
}

#[test]
fn hold_projection_follows_contact_not_flags() {
    // The projection is a stateless function of the current contact and
    // the last feasibility assessment: nothing anywhere stores a held
    // flag, so losing contact immediately reads as released.
    let contact = contact_fixture();
    let pot = body([0, 0, 0]);
    let assessment = GraspRequest::new(78_453_200_000_000_000, 500_000)
        .and_then(|request| grasp_proposal(Some(&contact), &pot, &request))
        .expect("double the boundary force holds with margin");

    assert_eq!(
        hold_projection(Some(&contact), &assessment),
        HoldState::Held
    );
    assert_eq!(
        hold_projection(None, &assessment),
        HoldState::Released,
        "no stored flag can keep possession alive without contact"
    );
}

#[test]
fn infeasible_assessment_never_projects_to_held() {
    // μ = 0.001 (1_000 micro) needs N ≥ 1000·W = 19.6 kN, which exceeds
    // the representable force range entirely; even the largest sane grip
    // force fails the cone while contact exists.
    let contact = contact_fixture();
    let pot = body([0, 0, 0]);
    let error = GraspRequest::new(9_000_000_000_000_000_000, 1_000)
        .and_then(|request| grasp_proposal(Some(&contact), &pot, &request))
        .expect_err("9 kN of normal force at μ = 0.001 gives only 9 N of friction");
    assert!(matches!(error, ContactError::FrictionInfeasible));

    // μ = 0.01 (10_000 micro) closes the cone with N = 2·W/μ exactly:
    // max friction = 39.2266 N against the 19.6133 N weight.
    let holding = GraspRequest::new(3_922_660_000_000_000_000, 10_000)
        .and_then(|request| grasp_proposal(Some(&contact), &pot, &request))
        .expect("double-margin grip holds the pot");
    assert_eq!(hold_projection(Some(&contact), &holding), HoldState::Held);
    assert_eq!(
        holding.max_friction_force_mgnm_per_s2(),
        2 * holding.weight_force_mgnm_per_s2()
    );
}

fn contact_fixture() -> ContactManifold {
    let a = body([0, 0, 0]);
    let b = body([150_000_000, 0, 0]);
    contact(
        &a,
        &collider([100_000_000, 50_000_000, 50_000_000]),
        &b,
        &collider([80_000_000, 40_000_000, 40_000_000]),
    )
    .expect("fixture bodies overlap")
}
