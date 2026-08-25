//! Tangential Coulomb-friction evidence: the friction impulse is capped
//! by µ·|normal impulse|, sticks the contact exactly when the cap covers
//! the reduced-mass stopping impulse, and slides along the relative
//! motion otherwise. Independent anchors come from hand-derived
//! textbook values (E = ½mv², p = mv, reduced mass m₁m₂/(m₁+m₂)), never
//! from the production code. All contacts lie on the x axis, so the
//! declared single-tangential-axis validity range covers every fixture.

use makise_causal_kernel::{
    BoxCollider, CollisionResponseProposal, ContactError, RigidBody, contact_proposal,
    resolve_collision,
};

const TWENTY_KG: i64 = 20_000_000;

fn body(mass_mg: i64, position_nm: [i64; 3], velocity_nm_per_s: [i64; 3]) -> RigidBody {
    RigidBody::new(
        mass_mg,
        position_nm,
        velocity_nm_per_s,
        [0; 3],
        [1_000, 1_000, 1_000],
        [0; 3],
    )
    .expect("valid rigid body")
}

fn collider() -> BoxCollider {
    BoxCollider::new([50_000_000; 3]).expect("valid collider")
}

/// Two cubes half-extent 50 mm centred 80 mm apart: a 20 mm overlap on x
/// with b on the positive side, so the normal points from b toward a
/// along −x and the tangential plane is y/z.
fn pair(
    velocity_a_nm_per_s: [i64; 3],
    velocity_b_nm_per_s: [i64; 3],
) -> (RigidBody, BoxCollider, RigidBody, BoxCollider) {
    (
        body(TWENTY_KG, [0, 500_000_000, 0], velocity_a_nm_per_s),
        collider(),
        body(TWENTY_KG, [80_000_000, 500_000_000, 0], velocity_b_nm_per_s),
        collider(),
    )
}

fn momentum_sum(body_a: &RigidBody, body_b: &RigidBody) -> [i128; 3] {
    let pa = body_a.linear_momentum_mg_nm_per_s().expect("representable");
    let pb = body_b.linear_momentum_mg_nm_per_s().expect("representable");
    [
        i128::from(pa[0]) + i128::from(pb[0]),
        i128::from(pa[1]) + i128::from(pb[1]),
        i128::from(pa[2]) + i128::from(pb[2]),
    ]
}

/// Elastic head-on collision (±2 m/s) plus 3 m/s of relative slip on z.
/// Hand anchors: normal impulses swap the x velocities; the stopping
/// impulse for the slip is μ_r·Δv_t = 10 kg · 3 m/s = 30 kg·m/s, well
/// inside the µ = 1 cap of µ·J_n = 1 · 40 kg·m/s · ... = 80 kg·m/s, so
/// the contact sticks: both bodies leave with the common centre-of-mass
/// slip 1.5 m/s. KE falls 170 J → 125 J, exactly the ½·μ_r·Δv_t² = 45 J
/// the slipping contact dissipates; z momentum stays 60 kg·m/s.
#[test]
fn sufficient_friction_sticks_the_slip_exactly() {
    let (a, ca, b, cb) = pair([2_000_000_000, 0, 3_000_000_000], [-2_000_000_000, 0, 0]);
    let proposal = CollisionResponseProposal::new(1_000_000)
        .expect("valid restitution")
        .with_friction_coefficient(1_000_000)
        .expect("valid friction");
    let momentum_before = momentum_sum(&a, &b);

    let resolution = resolve_collision(&a, &ca, &b, &cb, &proposal)
        .expect("valid geometry")
        .expect("bodies overlap");

    assert_eq!(
        resolution.next_a().velocity_nm_per_s(),
        [-2_000_000_000, 0, 1_500_000_000]
    );
    assert_eq!(
        resolution.next_b().velocity_nm_per_s(),
        [2_000_000_000, 0, 1_500_000_000]
    );
    assert_eq!(
        resolution.friction_impulse_mg_nm_per_s(),
        30_000_000_000_000_000
    );
    assert_eq!(resolution.kinetic_energy_before_nj(), 170_000_000_000);
    assert_eq!(resolution.kinetic_energy_after_nj(), 125_000_000_000);
    assert_eq!(
        momentum_sum(resolution.next_a(), resolution.next_b()),
        momentum_before
    );
}

/// The exact boundary µ·J_n = μ_r·Δv_t closes the cone: 30/80 = 0.375
/// caps the friction impulse at precisely the sticking requirement, so
/// the outcome equals the sticking case bit for bit.
#[test]
fn exact_cone_boundary_still_sticks() {
    let (a, ca, b, cb) = pair([2_000_000_000, 0, 3_000_000_000], [-2_000_000_000, 0, 0]);
    let proposal = CollisionResponseProposal::new(1_000_000)
        .expect("valid restitution")
        .with_friction_coefficient(375_000)
        .expect("valid friction");

    let resolution = resolve_collision(&a, &ca, &b, &cb, &proposal)
        .expect("valid geometry")
        .expect("bodies overlap");

    assert_eq!(
        resolution.next_a().velocity_nm_per_s(),
        [-2_000_000_000, 0, 1_500_000_000]
    );
    assert_eq!(
        resolution.next_b().velocity_nm_per_s(),
        [2_000_000_000, 0, 1_500_000_000]
    );
    assert_eq!(
        resolution.friction_impulse_mg_nm_per_s(),
        30_000_000_000_000_000
    );
}

/// µ = 0.25 caps the friction impulse at 0.25 · 80 = 20 kg·m/s < 30,
/// so the contact slides: the slip shrinks from 3 m/s to 1 m/s without
/// reversing, KE falls 170 J → 130 J (½·μ_r·(9−1) = 40 J), and z
/// momentum is still conserved bit-exact.
#[test]
fn insufficient_friction_slides_and_never_reverses_the_slip() {
    let (a, ca, b, cb) = pair([2_000_000_000, 0, 3_000_000_000], [-2_000_000_000, 0, 0]);
    let proposal = CollisionResponseProposal::new(1_000_000)
        .expect("valid restitution")
        .with_friction_coefficient(250_000)
        .expect("valid friction");
    let momentum_before = momentum_sum(&a, &b);

    let resolution = resolve_collision(&a, &ca, &b, &cb, &proposal)
        .expect("valid geometry")
        .expect("bodies overlap");

    assert_eq!(
        resolution.next_a().velocity_nm_per_s(),
        [-2_000_000_000, 0, 2_000_000_000]
    );
    assert_eq!(
        resolution.next_b().velocity_nm_per_s(),
        [2_000_000_000, 0, 1_000_000_000]
    );
    assert_eq!(
        resolution.friction_impulse_mg_nm_per_s(),
        20_000_000_000_000_000
    );
    assert_eq!(resolution.kinetic_energy_after_nj(), 130_000_000_000);
    assert!(resolution.kinetic_energy_after_nj() < resolution.kinetic_energy_before_nj());
    assert_eq!(
        momentum_sum(resolution.next_a(), resolution.next_b()),
        momentum_before
    );
}

/// Without a declared friction coefficient the slice behaves exactly as
/// before: tangential state passes through untouched.
#[test]
fn default_proposal_applies_no_friction() {
    let (a, ca, b, cb) = pair([2_000_000_000, 0, 3_000_000_000], [-2_000_000_000, 0, 0]);
    let proposal = CollisionResponseProposal::new(1_000_000).expect("valid restitution");

    let resolution = resolve_collision(&a, &ca, &b, &cb, &proposal)
        .expect("valid geometry")
        .expect("bodies overlap");

    assert_eq!(proposal.friction_coefficient_micro(), 0);
    assert_eq!(resolution.friction_impulse_mg_nm_per_s(), 0);
    assert_eq!(resolution.next_a().velocity_nm_per_s()[2], 3_000_000_000);
    assert_eq!(resolution.next_b().velocity_nm_per_s()[2], 0);
}

/// ADR-0014 negative validation: a coefficient above unity creates
/// energy and is rejected at construction.
#[test]
fn friction_coefficient_above_unity_is_invalid() {
    let error = CollisionResponseProposal::new(0)
        .expect("valid restitution")
        .with_friction_coefficient(1_000_001)
        .expect_err("coefficient above 1 violates the unit interval");
    assert!(matches!(error, ContactError::InvalidParameters));
}

/// Declared validity range: relative slip along both tangential axes
/// cannot be resolved with exact axis-aligned integer arithmetic, so a
/// coefficient-bearing request is typed-rejected instead of silently
/// approximating the friction direction.
#[test]
fn two_axis_slip_is_outside_declared_validity_range() {
    let (a, ca, b, cb) = pair(
        [2_000_000_000, 400_000_000, 3_000_000_000],
        [-2_000_000_000, 0, 0],
    );
    let proposal = CollisionResponseProposal::new(1_000_000)
        .expect("valid restitution")
        .with_friction_coefficient(1_000_000)
        .expect("valid friction");

    let error = resolve_collision(&a, &ca, &b, &cb, &proposal)
        .expect_err("two-axis slip is outside the declared validity range");
    assert!(matches!(error, ContactError::OutsideValidityRange));

    // The same fixture without friction stays resolvable: the manifold
    // exists and the normal response proceeds.
    let plain = CollisionResponseProposal::new(1_000_000).expect("valid restitution");
    let resolution = resolve_collision(&a, &ca, &b, &cb, &plain)
        .expect("valid geometry")
        .expect("bodies overlap");
    assert!(
        contact_proposal(&a, &ca, &b, &cb)
            .expect("valid geometry")
            .is_some()
    );
    assert_eq!(resolution.normal(), [-1, 0, 0]);
}
