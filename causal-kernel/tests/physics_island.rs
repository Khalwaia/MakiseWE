//! Phase 2 slice 5 evidence: active islands, deterministic scheduling,
//! and explicit rest transitions.
//!
//! Independent anchors: free-fall kinematics from the slice-2 body
//! (v₁ = v₀ − g·Δt, y₁ = y₀ + v₀·Δt − g·Δt²/2 with
//! g = 9_806_650_000 nm/s²) and box overlaps from the slice-3 contact
//! proposal. Bodies weigh 20 kg so every boundary energy stays a whole
//! number of nanojoules (½·20·g² J = 961_703_842_225 nJ), a limit
//! established in the slice-2 evidence.

use makise_causal_kernel::{
    BoxCollider, IslandError, RestSuspension, RigidBody, advance_awake_bodies,
    advance_island_members, layout_islands, resume_island, suspend_island,
};

const GRAVITY_NM_PER_S2: i64 = 9_806_650_000;

fn body(position_nm: [i64; 3]) -> RigidBody {
    RigidBody::new(
        20_000_000,
        position_nm,
        [0; 3],
        [0; 3],
        [1_000, 1_000, 1_000],
        [0; 3],
    )
    .expect("valid rigid body")
}

fn unit_collider() -> BoxCollider {
    BoxCollider::new([50_000_000; 3]).expect("valid collider")
}

/// Three 20 kg cubes spaced 80 mm apart along x (20 mm overlap between
/// neighbours only) plus one isolated cube elsewhere: bodies 0–2 form a
/// transitive chain, body 3 stands alone.
fn chain_and_lonely() -> (Vec<RigidBody>, Vec<BoxCollider>) {
    let positions = [
        [0, 500_000_000, 0],
        [80_000_000, 500_000_000, 0],
        [160_000_000, 500_000_000, 0],
        [900_000_000, -300_000_000, 400_000_000],
    ];
    let bodies = positions.map(body).to_vec();
    let colliders = vec![unit_collider(); bodies.len()];
    (bodies, colliders)
}

fn step_members(members: &[usize], bodies: &[RigidBody]) -> Vec<(usize, RigidBody)> {
    advance_island_members(members, bodies)
        .expect("one-second gravity step is always valid here")
        .into_iter()
        .zip(members.iter().copied())
        .map(|(state, index)| (index, state))
        .collect()
}

fn merge(base: &[RigidBody], proposals: &[(usize, RigidBody)]) -> Vec<RigidBody> {
    let mut merged = base.to_vec();
    for (index, state) in proposals {
        merged[*index] = *state;
    }
    merged
}

#[test]
fn transitive_contacts_group_bodies_into_deterministic_islands() {
    let (bodies, colliders) = chain_and_lonely();
    let layout = layout_islands(&bodies, &colliders).expect("valid inputs");

    assert_eq!(
        layout.islands(),
        &[vec![0, 1, 2], vec![3]],
        "members ascend inside an island; islands order by smallest member"
    );
    for (expected, body_index) in [(0, 0), (0, 1), (0, 2), (1, 3)] {
        assert_eq!(layout.island_of_body(body_index), Some(expected));
    }
}

#[test]
fn mismatched_inputs_are_rejected_typed() {
    let (bodies, _) = chain_and_lonely();
    let error = layout_islands(&bodies, &[unit_collider()])
        .expect_err("every body needs a declared collider");
    assert!(matches!(error, IslandError::MismatchedInputs));
}

#[test]
fn unknown_island_index_is_rejected_typed() {
    let (bodies, colliders) = chain_and_lonely();
    let layout = layout_islands(&bodies, &colliders).expect("valid inputs");
    let error =
        suspend_island(&layout, 7, &bodies).expect_err("only two islands exist in this fixture");
    assert!(matches!(
        error,
        IslandError::UnknownIsland { island_index: 7 }
    ));
}

#[test]
fn island_execution_order_does_not_change_the_outcome() {
    // Islands are disjoint by construction, so stepping them in forward
    // or reverse order must land every member on identical kinematics.
    let (bodies, colliders) = chain_and_lonely();
    let layout = layout_islands(&bodies, &colliders).expect("valid inputs");

    let mut forward = bodies.clone();
    for members in layout.islands() {
        forward = merge(&forward, &step_members(members, &forward));
    }
    let mut reverse = bodies.clone();
    for members in layout.islands().iter().rev() {
        reverse = merge(&reverse, &step_members(members, &reverse));
    }
    assert_eq!(forward, reverse);

    // One flat single-writer pass agrees with both island schedules.
    assert_eq!(advance_awake_bodies(&bodies).expect("flat step"), forward);
}

#[test]
fn worker_partition_reduces_identically_to_single_writer() {
    // Worker one takes the first island, worker two the second; each
    // proposes stepped states for its members only, and the canonical
    // merge by body index equals the single-writer flat pass exactly —
    // the Phase 0 matrix row "1 and N workers agree", inherited by the
    // physics scheduler.
    let (bodies, colliders) = chain_and_lonely();
    let layout = layout_islands(&bodies, &colliders).expect("valid inputs");
    assert_eq!(layout.islands().len(), 2);

    let mut proposals = Vec::new();
    for members in layout.islands() {
        proposals.extend(step_members(members, &bodies));
    }
    let merged = merge(&bodies, &proposals);
    assert_eq!(merged, advance_awake_bodies(&bodies).expect("flat step"));

    // Independent kinematics anchor: every body falls exactly
    // g·Δt²/2 = 4_903_325_000 nm regardless of schedule.
    let expected_y = [
        500_000_000 - GRAVITY_NM_PER_S2 / 2,
        -300_000_000 - GRAVITY_NM_PER_S2 / 2,
    ];
    for (group, expected) in expected_y.iter().enumerate() {
        let member = layout.islands()[group][0];
        assert_eq!(merged[member].position_nm()[1], *expected);
        assert_eq!(merged[member].velocity_nm_per_s()[1], -GRAVITY_NM_PER_S2);
    }
}

#[test]
fn suspended_island_resumes_bit_exact_and_conserving() {
    // Suspension is an explicit representation transition: it snapshots
    // the island, excludes it from active stepping, and restores it bit
    // exactly. Energy accounting before == after proves the transition
    // itself conserves everything (INVARIANTS §12).
    let (bodies, colliders) = chain_and_lonely();
    let layout = layout_islands(&bodies, &colliders).expect("valid inputs");
    let energy_of = |states: &[RigidBody]| -> i128 {
        states
            .iter()
            .map(|state| i128::from(state.total_mechanical_energy_nj().expect("representable")))
            .sum()
    };
    let total_before = energy_of(&bodies);

    let suspension: RestSuspension = suspend_island(&layout, 0, &bodies).expect("suspend");
    assert_eq!(suspension.members(), &[0, 1, 2]);
    assert_eq!(suspension.awake_indices(), &[3]);

    // The awake remainder keeps falling while the island rests…
    let awake = suspension.awake_bodies(&bodies);
    let stepped_awake = advance_awake_bodies(&awake).expect("step");
    assert_eq!(stepped_awake.len(), 1);
    assert_eq!(
        stepped_awake[0].position_nm()[1],
        -300_000_000 - GRAVITY_NM_PER_S2 / 2
    );

    // …but a moved resting member cannot silently resume.
    let mut tampered = bodies.clone();
    tampered[1] = body([80_000_000, 400_000_000, 0]);
    let error =
        resume_island(&suspension, &tampered).expect_err("digest detects drift since suspension");
    assert!(matches!(error, IslandError::RestStateMismatch));

    // …while the untouched world resumes bit exactly and conserves.
    resume_island(&suspension, &bodies).expect("clean resume");
    let restored = suspension.restore_members();
    assert_eq!(restored, bodies[..3]);
    assert_eq!(energy_of(&restored) + energy_of(&awake), total_before);
}
