//! Phase 2 slice 11 evidence: exact integer balance feedback over the
//! declared support base. One foot contact supports only a centre of
//! mass projected exactly onto it; two contacts support the whole
//! segment between them (collinear projection inside both axis bounds);
//! anything else tips. Independent anchors are the fixture geometry in
//! nanometres, never the production code.

use makise_causal_kernel::{BalanceError, BalanceState, balance_assessment};

const MM: i64 = 1_000_000;

/// Double support with the centre of mass halfway between the feet:
/// the classic stable stance.
#[test]
fn centred_double_support_is_stable() {
    let assessment =
        balance_assessment(&[[0, 0], [400 * MM, 0]], [200 * MM, 0]).expect("within validity range");
    assert_eq!(assessment.state(), BalanceState::Stable);
}

/// Sliding the projection past either foot leaves the segment: the
/// body tips forward even though it stays collinear with the feet.
#[test]
fn projection_beyond_either_foot_tips() {
    let behind =
        balance_assessment(&[[0, 0], [400 * MM, 0]], [-100 * MM, 0]).expect("within range");
    assert_eq!(behind.state(), BalanceState::Tipping);

    let ahead = balance_assessment(&[[0, 0], [400 * MM, 0]], [500 * MM, 0]).expect("within range");
    assert_eq!(ahead.state(), BalanceState::Tipping);

    // Exactly over the front foot is the boundary and still supported.
    let boundary =
        balance_assessment(&[[0, 0], [400 * MM, 0]], [400 * MM, 0]).expect("within range");
    assert_eq!(boundary.state(), BalanceState::Stable);
}

/// Leaning sideways off the foot line leaves the support segment even
/// though the along-line coordinate is centred.
#[test]
fn lateral_offset_from_the_foot_line_tips() {
    let assessment = balance_assessment(&[[0, 0], [400 * MM, 0]], [200 * MM, 50 * MM])
        .expect("within validity range");
    assert_eq!(assessment.state(), BalanceState::Tipping);
}

/// Single support is a point base: the projection must coincide with
/// the foot exactly; one nanometre of lean already tips.
#[test]
fn single_support_requires_exact_alignment() {
    let balanced = balance_assessment(&[[-150 * MM, 300 * MM]], [-150 * MM, 300 * MM])
        .expect("within validity range");
    assert_eq!(balanced.state(), BalanceState::Stable);

    let leaning = balance_assessment(&[[-150 * MM, 300 * MM]], [-150 * MM, 300 * MM - 1])
        .expect("within validity range");
    assert_eq!(leaning.state(), BalanceState::Tipping);
}

/// No contact with the floor is a legitimate physical state, distinct
/// from tipping: the walker is airborne.
#[test]
fn empty_support_set_reports_airborne() {
    let assessment = balance_assessment(&[], [0, 0]).expect("empty set is valid input");
    assert_eq!(assessment.state(), BalanceState::Airborne);
}

/// Declared bipedal validity range: three simultaneous contacts would
/// need a full convex-hull base and are typed-rejected, never silently
/// approximated by a bounding box.
#[test]
fn three_contacts_are_outside_bipedal_validity_range() {
    let error = balance_assessment(&[[0, 0], [100 * MM, 0], [0, 100 * MM]], [30 * MM, 30 * MM])
        .expect_err("tripod stances exceed this slice's declared range");
    assert!(matches!(error, BalanceError::OutsideValidityRange));
}

/// Diagonal stances keep working in negative coordinates: the segment
/// test is affine, not anchored to the first quadrant.
#[test]
fn diagonal_stance_in_negative_coordinates_is_assessed_exactly() {
    let feet = [[-200 * MM, -100 * MM], [200 * MM, 100 * MM]];
    let midpoint = [0, 0];
    let on_segment = balance_assessment(&feet, midpoint).expect("within validity range");
    assert_eq!(on_segment.state(), BalanceState::Stable);

    let off_line = balance_assessment(&feet, [10 * MM, 0]).expect("within validity range");
    assert_eq!(off_line.state(), BalanceState::Tipping);
}
