//! Collision-response evidence: impulse resolution along the declared
//! contact normal with exact momentum conservation, restitution as a
//! fixed-point parameter, and mass-split positional correction.
//!
//! Independent anchors come from textbook one-dimensional collisions of
//! equal and 20 kg / 10 kg bodies (E = ½mv², p = mv), never from the
//! production code. All contacts lie on the x axis, so the positional
//! correction never shifts a body vertically and potential energy is
//! unaffected.

use makise_causal_kernel::{
    BoxCollider, CollisionResolution, CollisionResponseProposal, ContactError, RigidBody,
    contact_proposal, resolve_collision,
};

fn body(mass_mg: i64, position_nm: [i64; 3], velocity_x_nm_per_s: i64) -> RigidBody {
    RigidBody::new(
        mass_mg,
        position_nm,
        [velocity_x_nm_per_s, 0, 0],
        [0; 3],
        [1_000, 1_000, 1_000],
        [0; 3],
    )
    .expect("valid rigid body")
}

fn resolve(
    a: &RigidBody,
    ca: &BoxCollider,
    b: &RigidBody,
    cb: &BoxCollider,
    proposal: &CollisionResponseProposal,
) -> CollisionResolution {
    resolve_collision(a, ca, b, cb, proposal)
        .expect("fixture geometry is valid")
        .expect("fixture bodies overlap")
}

fn collider() -> BoxCollider {
    BoxCollider::new([50_000_000; 3]).expect("valid collider")
}

/// Two cubes half-extent 50 mm centred 80 mm apart: a 20 mm overlap on x
/// with b on the positive side, so the normal pushes a toward −x.
fn pair(
    v_a: i64,
    m_a: i64,
    v_b: i64,
    m_b: i64,
) -> (RigidBody, BoxCollider, RigidBody, BoxCollider) {
    (
        body(m_a, [0, 500_000_000, 0], v_a),
        collider(),
        body(m_b, [80_000_000, 500_000_000, 0], v_b),
        collider(),
    )
}

const TWENTY_KG: i64 = 20_000_000;
const TEN_KG: i64 = 10_000_000;

#[test]
fn elastic_equal_masses_exchange_velocities_exactly() {
    // Textbook anchor: equal masses, e = 1 head-on → velocities swap.
    let (a, ca, b, cb) = pair(2_000_000_000, TWENTY_KG, -2_000_000_000, TWENTY_KG);
    let proposal = CollisionResponseProposal::new(1_000_000).expect("valid restitution");
    let resolution = resolve(&a, &ca, &b, &cb, &proposal);
    assert_eq!(resolution.next_a().velocity_nm_per_s()[0], -2_000_000_000);
    assert_eq!(resolution.next_b().velocity_nm_per_s()[0], 2_000_000_000);
    assert_eq!(
        resolution.kinetic_energy_before_nj(),
        resolution.kinetic_energy_after_nj(),
        "e = 1 conserves kinetic energy exactly"
    );
    assert_eq!(resolution.kinetic_energy_before_nj(), 80_000_000_000);
}

#[test]
fn plastic_collision_stops_both_in_the_centre_of_mass_frame() {
    // Equal masses, opposite velocities, e = 0 → both stop dead.
    let (a, ca, b, cb) = pair(2_000_000_000, TWENTY_KG, -2_000_000_000, TWENTY_KG);
    let proposal = CollisionResponseProposal::new(0).expect("valid restitution");
    let resolution = resolve(&a, &ca, &b, &cb, &proposal);
    assert_eq!(resolution.next_a().velocity_nm_per_s()[0], 0);
    assert_eq!(resolution.next_b().velocity_nm_per_s()[0], 0);
    assert_eq!(resolution.kinetic_energy_after_nj(), 0);
}

#[test]
fn unequal_masses_elastic_collision_matches_textbook_values() {
    // Independent anchors: m₁v₁ = (m₁−m₂)/(m₁+m₂)·v₁ and
    // v₂ = 2m₁/(m₁+m₂)·v₁. With 20 kg at 3 m/s hitting 10 kg at rest:
    // v₁' = 1 m/s, v₂' = 4 m/s; KE = 90 J before and after.
    let (a, ca, b, cb) = pair(3_000_000_000, TWENTY_KG, 0, TEN_KG);
    let proposal = CollisionResponseProposal::new(1_000_000).expect("valid restitution");
    let resolution = resolve(&a, &ca, &b, &cb, &proposal);
    assert_eq!(resolution.next_a().velocity_nm_per_s()[0], 1_000_000_000);
    assert_eq!(resolution.next_b().velocity_nm_per_s()[0], 4_000_000_000);
    assert_eq!(resolution.kinetic_energy_before_nj(), 90_000_000_000);
    assert_eq!(
        resolution.kinetic_energy_before_nj(),
        resolution.kinetic_energy_after_nj()
    );
}

#[test]
fn partial_restitution_loses_exactly_the_declared_share() {
    // 20 kg at 2 m/s into an equal resting mass with e = 0.5:
    // momentum keeps both moving at v₁' = 0.5, v₂' = 1.5 m/s;
    // KE falls 40 J → 25 J, monotonically, never up.
    let (a, ca, b, cb) = pair(2_000_000_000, TWENTY_KG, 0, TWENTY_KG);
    let proposal = CollisionResponseProposal::new(500_000).expect("valid restitution");
    let resolution = resolve(&a, &ca, &b, &cb, &proposal);
    assert_eq!(resolution.next_a().velocity_nm_per_s()[0], 500_000_000);
    assert_eq!(resolution.next_b().velocity_nm_per_s()[0], 1_500_000_000);
    assert_eq!(resolution.kinetic_energy_before_nj(), 40_000_000_000);
    assert_eq!(resolution.kinetic_energy_after_nj(), 25_000_000_000);

    // Momentum is conserved bit-exact even where energy is not.
    let momentum_x =
        |state: &RigidBody| state.linear_momentum_mg_nm_per_s().expect("representable")[0];
    assert_eq!(
        momentum_x(&a) + momentum_x(&b),
        momentum_x(resolution.next_a()) + momentum_x(resolution.next_b())
    );
}

#[test]
fn separating_bodies_receive_no_impulse() {
    // a already moves away from b along the normal: no closing velocity,
    // so velocities stay untouched while positions still de-penetrate.
    let (a, ca, b, cb) = pair(-1_000_000_000, TWENTY_KG, 0, TWENTY_KG);
    let proposal = CollisionResponseProposal::new(1_000_000).expect("valid restitution");
    let resolution = resolve(&a, &ca, &b, &cb, &proposal);
    assert_eq!(resolution.impulse_mg_nm_per_s(), 0);
    assert_eq!(resolution.next_a().velocity_nm_per_s()[0], -1_000_000_000);
    assert_eq!(resolution.next_b().velocity_nm_per_s()[0], 0);
}

#[test]
fn positional_correction_splits_depth_and_clears_overlap() {
    // Equal masses share the 20 mm penetration exactly 10 mm each; after
    // the split the boxes merely touch, so the detector reports no
    // further positive overlap.
    let (a, ca, b, cb) = pair(0, TWENTY_KG, 0, TWENTY_KG);
    let proposal = CollisionResponseProposal::new(0).expect("valid restitution");
    let resolution = resolve(&a, &ca, &b, &cb, &proposal);
    assert_eq!(resolution.next_a().position_nm()[0], -10_000_000);
    assert_eq!(resolution.next_b().position_nm()[0], 90_000_000);
    assert!(
        contact_proposal(resolution.next_a(), &ca, resolution.next_b(), &cb)
            .expect("valid geometry")
            .is_none(),
        "correction clears the positive overlap"
    );
}

#[test]
fn restitution_outside_unit_range_is_invalid() {
    let error =
        CollisionResponseProposal::new(1_000_001).expect_err("restitution above 1 creates energy");
    assert!(matches!(error, ContactError::InvalidParameters));
}

#[test]
fn non_representable_mass_velocity_ratio_is_typed() {
    // 1 kg into 3 kg with a 1 nm/s closing speed leaves the impulse
    // quotient fractional: typed rejection, never silent rounding.
    let (a, ca, b, cb) = pair(1, 1_000_000, 0, 3_000_000);
    let proposal = CollisionResponseProposal::new(0).expect("valid restitution");
    let error = resolve_collision(&a, &ca, &b, &cb, &proposal)
        .expect_err("quotient is fractional for these integers");
    assert!(matches!(error, ContactError::NonRepresentableResponse));
}
