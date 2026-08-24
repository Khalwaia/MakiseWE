use makise_causal_kernel::{RigidBody, RigidBodyError};

#[test]
fn body_construction_rejects_nonpositive_mass() {
    let error = RigidBody::new(0, [0, 0], [0, 0]).expect_err("zero mass is outside validity range");
    assert!(matches!(error, RigidBodyError::OutsideValidityRange));

    let error = RigidBody::new(-1_000, [0, 0], [0, 0])
        .expect_err("negative mass is outside validity range");
    assert!(matches!(error, RigidBodyError::OutsideValidityRange));
}

#[test]
fn one_second_free_fall_matches_kinematics() {
    let body = RigidBody::new(1_000_000_000, [0, 1_000_000_000], [0, 0]).expect("valid rigid body");
    let proposal = body.gravity_proposal().expect("valid gravity proposal");

    let next = proposal.apply(&body);
    let velocity_y = next.velocity_nm_per_s()[1];
    let position_y = next.position_nm()[1];

    assert_eq!(velocity_y, -9_806_650_000);
    assert_eq!(position_y, 1_000_000_000 - 4_903_325_000);
}

#[test]
fn energy_conservation_tracks_gravity_work_exactly() {
    let body = RigidBody::new(2_000_000_000, [0, 500_000_000], [0, 0]).expect("valid rigid body");
    let proposal = body.gravity_proposal().expect("valid gravity proposal");
    let next = proposal.apply(&body);

    let total_before = body.total_mechanical_energy_nj();
    let total_after = next.total_mechanical_energy_nj();
    assert_eq!(total_before, total_after);
}

#[test]
fn overflow_is_typed_failure_not_wraparound() {
    let body = RigidBody::new(i64::MAX / 2, [0, 0], [i64::MAX / 2, 0])
        .expect("extreme values are representable");
    let error = body
        .total_mechanical_energy_nj()
        .expect_err("energy computation must return typed overflow");
    assert!(matches!(error, RigidBodyError::Overflow));
}
