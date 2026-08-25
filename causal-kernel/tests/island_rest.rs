//! Phase 2 slice 10 evidence: the physical rest trigger that feeds
//! island scheduling. An island may be proposed for rest only when every
//! member is quiescent (exactly zero velocity — integer exactness leaves
//! no epsilon) and physically supported: either by the declared
//! apartment floor plane at y = 0 or by a vertical contact against a
//! lower island member.
//!
//! Independent anchors are the fixture geometry itself (box extents and
//! centres in nm), never the production code.

use makise_causal_kernel::{BoxCollider, IslandError, RigidBody, layout_islands, resting_islands};

fn body(position_nm: [i64; 3], velocity_nm_per_s: [i64; 3]) -> RigidBody {
    RigidBody::new(
        20_000_000,
        position_nm,
        velocity_nm_per_s,
        [0; 3],
        [1_000, 1_000, 1_000],
        [0; 3],
    )
    .expect("valid rigid body")
}

fn unit_collider() -> BoxCollider {
    BoxCollider::new([50_000_000; 3]).expect("valid collider")
}

fn world(entries: &[[i64; 3]]) -> (Vec<RigidBody>, Vec<BoxCollider>) {
    (
        entries
            .iter()
            .map(|&position| body(position, [0; 3]))
            .collect(),
        vec![unit_collider(); entries.len()],
    )
}

fn resting(entries: &[[i64; 3]]) -> Vec<usize> {
    let (bodies, colliders) = world(entries);
    let layout = layout_islands(&bodies, &colliders).expect("valid inputs");
    resting_islands(&layout, &bodies, &colliders).expect("valid inputs")
}

/// A cube half-extent 50 mm centred at y = 50 mm touches the declared
/// floor plane exactly; its singleton island is a rest candidate.
#[test]
fn floor_resting_cube_qualifies() {
    assert_eq!(resting(&[[0, 50_000_000, 0]]), [0]);
}

/// One nanometre of clearance removes environmental support; sideways
/// motion breaks quiescence. Neither state may look like rest.
#[test]
fn lifted_or_moving_bodies_are_not_rest_candidates() {
    // Bottom face at y = +1 nm: unsupported.
    assert!(resting(&[[0, 50_000_001, 0]]).is_empty());

    // Resting pose but drifting along x: not quiescent.
    let (bodies, colliders) = world(&[[0, 50_000_000, 0]]);
    let mut drifting = bodies;
    drifting[0] = body([0, 50_000_000, 0], [1_000, 0, 0]);
    let layout = layout_islands(&drifting, &colliders).expect("valid inputs");
    assert!(
        resting_islands(&layout, &drifting, &colliders)
            .expect("valid inputs")
            .is_empty()
    );
}

/// A two-cube stack: the lower cube stands on the floor, the upper cube
/// overlaps it by 1 nm along y, so the upper cube's weight path runs
/// through an intra-island vertical contact. One island, fully
/// supported, fully quiescent.
#[test]
fn stack_rests_through_intra_island_vertical_contacts() {
    let entries = [
        [0, 50_000_000, 0],  // lower cube: bottom on the floor plane
        [0, 149_999_999, 0], // upper cube: 1 nm overlap onto the lower
    ];
    let (bodies, colliders) = world(&entries);
    let layout = layout_islands(&bodies, &colliders).expect("valid inputs");
    assert_eq!(layout.islands(), &[vec![0, 1]], "the stack is one island");
    assert_eq!(
        resting_islands(&layout, &bodies, &colliders).expect("valid inputs"),
        [0]
    );
}

/// Two floor-standing cubes sharing a lateral (x-axis) contact stay a
/// valid rest island: gravity is opposed for each member by the floor,
/// and a side contact neither supports nor destabilises.
#[test]
fn lateral_contact_preserves_floor_supported_rest() {
    let entries = [
        [0, 50_000_000, 0],
        [80_000_000, 50_000_000, 0], // 20 mm lateral overlap
    ];
    let (bodies, colliders) = world(&entries);
    let layout = layout_islands(&bodies, &colliders).expect("valid inputs");
    assert_eq!(layout.islands(), &[vec![0, 1]]);
    assert_eq!(
        resting_islands(&layout, &bodies, &colliders).expect("valid inputs"),
        [0]
    );
}

/// Two cubes touching mid-air hang on nothing: the island is connected
/// but no member has upward support, so the trigger stays silent.
#[test]
fn floating_touching_pair_is_not_a_rest_candidate() {
    let entries = [[0, 500_000_000, 0], [80_000_000, 500_000_000, 0]];
    assert!(resting(&entries).is_empty());
}

/// An island rests only as a whole: one floating member latched onto a
/// grounded stack keeps the entire island out of the rest set, while
/// the untouched ground pair still qualifies.
#[test]
fn partially_supported_island_is_rejected_as_a_whole() {
    let base = [
        [0, 50_000_000, 0],  // grounded base cube
        [0, 149_999_999, 0], // stacked cube, 1 nm overlap onto the base
    ];
    let with_floater = [
        base[0],
        base[1],
        [80_000_000, 150_000_001, 0], // floating cube latched sideways:
                                      // touches the stack laterally but
                                      // clears the base top by 1 nm
    ];
    let (bodies, colliders) = world(&with_floater);
    let layout = layout_islands(&bodies, &colliders).expect("valid inputs");
    assert_eq!(layout.islands(), &[vec![0, 1, 2]]);
    assert!(
        resting_islands(&layout, &bodies, &colliders)
            .expect("valid inputs")
            .is_empty()
    );

    // Removing the floater restores the clean two-cube rest island.
    assert_eq!(resting(&base), [0]);
}

/// The proposal is a pure function of its inputs: repeated evaluation
/// over identical states yields identical islands.
#[test]
fn rest_proposal_is_deterministic_under_repetition() {
    let entries = [
        [0, 50_000_000, 0],
        [900_000_000, 400_000_000, 300_000_000], // isolated floater
    ];
    let first = resting(&entries);
    let second = resting(&entries);
    assert_eq!(first, second);
    assert_eq!(first, [0]);
}

/// Every body needs a declared collider; length mismatch is typed.
#[test]
fn mismatched_inputs_are_rejected_typed() {
    let (bodies, colliders) = world(&[[0, 50_000_000, 0]]);
    let layout = layout_islands(&bodies, &colliders).expect("valid inputs");
    let error =
        resting_islands(&layout, &bodies, &[]).expect_err("every body needs a declared collider");
    assert!(matches!(error, IslandError::MismatchedInputs));
}
