//! Phase 2 rigid-body slice evidence.
//!
//! Every expected absolute value is derived from published mechanics
//! (E = mgh, E = ½mv², E = ½Iω², L = Iω, impulse = m·g·Δt) computed by
//! hand against the declared integer units — never from the production
//! algorithm. Unit conventions: mass milligram (1e-6 kg), length
//! nanometre (1e-9 m), time second, energy nanojoule, principal inertia
//! milligram·square-metre (1e-6 kg·m²), angular velocity microradian
//! per second.

use makise_causal_kernel::{RigidBody, RigidBodyError};

const GRAVITY_NM_PER_S2: i64 = 9_806_650_000;

fn two_kilogram_body(
    position_nm: [i64; 3],
    velocity_nm_per_s: [i64; 3],
    angular_velocity_urad_per_s: [i64; 3],
) -> RigidBody {
    RigidBody::new(
        2_000_000,
        position_nm,
        velocity_nm_per_s,
        [0, 0, 0],
        [2_000, 4_000, 1_000],
        angular_velocity_urad_per_s,
    )
    .expect("valid rigid body")
}

#[test]
fn body_construction_rejects_nonpositive_mass_and_inertia() {
    let error = RigidBody::new(0, [0; 3], [0; 3], [0; 3], [1_000; 3], [0; 3])
        .expect_err("zero mass is outside validity range");
    assert!(matches!(error, RigidBodyError::OutsideValidityRange));

    let error = RigidBody::new(-1_000, [0; 3], [0; 3], [0; 3], [1_000; 3], [0; 3])
        .expect_err("negative mass is outside validity range");
    assert!(matches!(error, RigidBodyError::OutsideValidityRange));

    for axis in 0..3 {
        let mut inertia = [1_000; 3];
        inertia[axis] = 0;
        let error = RigidBody::new(1_000, [0; 3], [0; 3], [0; 3], inertia, [0; 3])
            .expect_err("zero principal moment is outside validity range");
        assert!(matches!(error, RigidBodyError::OutsideValidityRange));

        let mut inertia = [1_000; 3];
        inertia[axis] = -1;
        let error = RigidBody::new(1_000, [0; 3], [0; 3], [0; 3], inertia, [0; 3])
            .expect_err("negative principal moment is outside validity range");
        assert!(matches!(error, RigidBodyError::OutsideValidityRange));
    }
}

#[test]
fn one_second_free_fall_matches_kinematics() {
    let body = two_kilogram_body([0, 1_000_000_000, 0], [0, 0, 0], [0, 0, 0]);
    let proposal = body.gravity_proposal().expect("valid gravity proposal");

    let next = proposal.apply(&body);
    assert_eq!(
        next.velocity_nm_per_s(),
        [0, -GRAVITY_NM_PER_S2, 0],
        "v = v₀ − g·Δt with Δt = 1 s"
    );
    assert_eq!(
        next.position_nm(),
        [0, 1_000_000_000 - GRAVITY_NM_PER_S2 / 2, 0],
        "y = y₀ + v₀·Δt − g·Δt²/2"
    );
}

#[test]
fn absolute_potential_and_kinetic_energy_match_hand_calculation() {
    // Independent anchors: PE = mgh = 2 kg · 9.80665 m/s² · 1 m = 19.6133 J
    // and KE = ½mv² = ½ · 2 kg · (1 m/s)² = 1 J; 1 J = 1e9 nJ.
    let body = two_kilogram_body([0, 1_000_000_000, 0], [1_000_000_000, 0, 0], [0, 0, 0]);
    assert_eq!(
        body.total_mechanical_energy_nj().expect("representable"),
        19_613_300_000 + 1_000_000_000
    );

    // Same body at rest on the origin holds only PE = 0 J… at h = 0 both
    // terms vanish, so shift down one metre instead: PE = −19.6133 J.
    let below_origin = two_kilogram_body([0, -1_000_000_000, 0], [0, 0, 0], [0, 0, 0]);
    assert_eq!(
        below_origin
            .total_mechanical_energy_nj()
            .expect("representable"),
        -(19_613_300_000)
    );
}

#[test]
fn rotational_energy_enters_total_with_exact_absolute_value() {
    // Independent anchors with ω = (1, 0.5, 2) rad/s and
    // I = (2e-3, 4e-3, 1e-3) kg·m²:
    //   KE_rot = ½(2e-3·1² + 4e-3·0.5² + 1e-3·2²) J
    //          = ½(2e-3 + 1e-3 + 4e-3) J = 3.5e-3 J = 3_500_000 nJ.
    let spinning = two_kilogram_body(
        [0, 1_000_000_000, 0],
        [1_000_000_000, 0, 0],
        [1_000_000, 500_000, 2_000_000],
    );
    assert_eq!(
        spinning
            .total_mechanical_energy_nj()
            .expect("representable"),
        19_613_300_000 + 1_000_000_000 + 3_500_000
    );
}

#[test]
fn torque_free_gravity_preserves_angular_state_exactly() {
    // Gravity acts through the centre of mass, so torque about it is zero
    // and L = Iω must survive free fall bit-exactly. Independent anchor:
    // every component of Iω here equals 2e-3 kg·m²/s.
    let spinning = two_kilogram_body(
        [0, 1_000_000_000, 0],
        [0, 0, 0],
        [1_000_000, 500_000, 2_000_000],
    );
    let before = spinning
        .angular_momentum_mg_m2_urad_per_s()
        .expect("representable angular momentum");
    assert_eq!(before, [2_000_000_000, 2_000_000_000, 2_000_000_000]);

    let next = spinning
        .gravity_proposal()
        .expect("valid gravity proposal")
        .apply(&spinning);
    assert_eq!(
        next.angular_velocity_urad_per_s(),
        spinning.angular_velocity_urad_per_s()
    );
    assert_eq!(
        next.angular_momentum_mg_m2_urad_per_s()
            .expect("representable"),
        before,
        "torque-free fall conserves angular momentum"
    );
}

#[test]
fn linear_momentum_changes_exactly_by_gravity_impulse() {
    // Impulse theorem over Δt = 1 s: Δp = m·g·Δt exactly, horizontal
    // components untouched. Independent anchor:
    // 3 kg · 9.80665 m/s² · 1 s = 29.41995 kg·m/s.
    let body = RigidBody::new(
        3_000_000,
        [0; 3],
        [1_000_000_000, 2_000_000_000, 0],
        [0; 3],
        [1_000, 1_000, 1_000],
        [0; 3],
    )
    .expect("valid rigid body");
    let before = body.linear_momentum_mg_nm_per_s().expect("representable");
    assert_eq!(before, [3_000_000_000_000_000, 6_000_000_000_000_000, 0]);

    let next = body
        .gravity_proposal()
        .expect("valid gravity proposal")
        .apply(&body);
    let after = next.linear_momentum_mg_nm_per_s().expect("representable");
    let impulse_y = -3_000_000 * GRAVITY_NM_PER_S2;
    assert_eq!(after[0], before[0]);
    assert_eq!(after[2], before[2]);
    assert_eq!(after[1] - before[1], impulse_y);
}

#[test]
fn world_center_of_mass_tracks_free_fall_displacement() {
    // The centre of mass is rigidly attached to the body frame: world COM =
    // origin + declared offset, and both undergo the same displacement.
    let body = RigidBody::new(
        500_000,
        [100_000_000, 900_000_000, 30_000_000],
        [-200_000_000, 5_000_000_000, 400_000_000],
        [5_000_000, -10_000_000, 2_000_000],
        [1_000, 1_000, 1_000],
        [0; 3],
    )
    .expect("valid rigid body");
    assert_eq!(
        body.world_center_of_mass_nm().expect("representable"),
        [105_000_000, 890_000_000, 32_000_000]
    );

    let next = body
        .gravity_proposal()
        .expect("valid gravity proposal")
        .apply(&body);
    assert_eq!(
        next.world_center_of_mass_nm().expect("representable"),
        [-95_000_000, 986_675_000, 432_000_000]
    );
    assert_eq!(
        next.center_of_mass_offset_nm(),
        [5_000_000, -10_000_000, 2_000_000]
    );
}

#[test]
fn energy_conservation_tracks_gravity_work_exactly() {
    // 20 kg crate dropped from rest at h₀ = 0.5 m with spin
    // (1, 0, 2) rad/s about I = (2e-3, ·, 1e-3) kg·m². Independent
    // anchors: E_before = mgh₀ + E_rot = 98.0665 J + 0.003 J; after one
    // second h₁ = −4.403325 m and v₁ = −9.80665 m/s, so
    // ΔPE = −961_703_842_225 nJ exactly cancels ΔKE_trans.
    let crate_body = RigidBody::new(
        20_000_000,
        [0, 500_000_000, 0],
        [0, 0, 0],
        [0; 3],
        [2_000, 4_000, 1_000],
        [1_000_000, 0, 2_000_000],
    )
    .expect("valid rigid body");
    let next = crate_body
        .gravity_proposal()
        .expect("valid gravity proposal")
        .apply(&crate_body);
    let before = crate_body
        .total_mechanical_energy_nj()
        .expect("representable boundary energy");
    let after = next
        .total_mechanical_energy_nj()
        .expect("representable boundary energy");
    assert_eq!(before, 98_069_500_000);
    assert_eq!(after, before);
}

#[test]
fn inexact_rotational_energy_is_outside_validity_range() {
    // I·ω² = 9 µrad²·mg·m² per the integer model cannot halve into whole
    // nanojoules, so the state is outside the declared representable band
    // instead of being silently rounded.
    let body = RigidBody::new(1_000_000, [0; 3], [0; 3], [0; 3], [3, 3, 3], [1, 1, 1])
        .expect("valid construction");
    let error = body
        .total_mechanical_energy_nj()
        .expect_err("non-representable energy must be typed");
    assert!(matches!(error, RigidBodyError::OutsideValidityRange));
}

#[test]
fn overflow_is_typed_failure_not_wraparound() {
    let body = RigidBody::new(
        i64::MAX / 2,
        [0; 3],
        [i64::MAX / 2, 0, 0],
        [0; 3],
        [1, 1, 1],
        [0; 3],
    )
    .expect("extreme values are representable");
    let error = body
        .total_mechanical_energy_nj()
        .expect_err("energy computation must return typed overflow");
    assert!(matches!(error, RigidBodyError::Overflow));
}
