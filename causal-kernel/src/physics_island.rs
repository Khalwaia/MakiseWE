use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::contact::{BoxCollider, ContactError, contact_proposal};
use crate::rigid_body::RigidBody;

#[derive(Clone, Copy, Debug, Error, Eq, PartialEq)]
pub enum IslandError {
    #[error("body count and collider count differ")]
    MismatchedInputs,
    #[error("no island with index {island_index} in this layout")]
    UnknownIsland { island_index: usize },
    #[error("rest state drifted since suspension; resume rejected")]
    RestStateMismatch,
    #[error("contact proposal failed during island layout: {0}")]
    Contact(ContactError),
    #[error("checked arithmetic overflow in island scheduling")]
    Overflow,
}

impl From<ContactError> for IslandError {
    fn from(error: ContactError) -> Self {
        match error {
            ContactError::Overflow => Self::Overflow,
            other => Self::Contact(other),
        }
    }
}

/// Deterministic grouping of bodies into active islands: members of an
/// island ascend, islands order by smallest member index. Disjointness
/// makes the per-island reduction order-insensitive by construction.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IslandLayout {
    islands: Vec<Vec<usize>>,
}

impl IslandLayout {
    pub fn islands(&self) -> &[Vec<usize>] {
        &self.islands
    }

    pub fn island_of_body(&self, body_index: usize) -> Option<usize> {
        self.islands
            .iter()
            .position(|members| members.contains(&body_index))
    }
}

/// Groups bodies into connected components over pairwise box contacts.
/// A pair is linked when their colliders overlap; components are grown
/// from seeds scanned in ascending index order, so the layout is a pure
/// function of the input states.
pub fn layout_islands(
    bodies: &[RigidBody],
    colliders: &[BoxCollider],
) -> Result<IslandLayout, IslandError> {
    if bodies.len() != colliders.len() {
        return Err(IslandError::MismatchedInputs);
    }
    let mut links: Vec<Vec<usize>> = vec![Vec::new(); bodies.len()];
    for i in 0..bodies.len() {
        for j in (i + 1)..bodies.len() {
            if contact_proposal(&bodies[i], &colliders[i], &bodies[j], &colliders[j])?.is_some() {
                links[i].push(j);
                links[j].push(i);
            }
        }
    }

    let mut component_of = vec![usize::MAX; bodies.len()];
    let mut islands: Vec<Vec<usize>> = Vec::new();
    for seed in 0..bodies.len() {
        if component_of[seed] != usize::MAX {
            continue;
        }
        let island_index = islands.len();
        component_of[seed] = island_index;
        let mut members = vec![seed];
        let mut queue = vec![seed];
        while let Some(current) = queue.pop() {
            for &neighbour in &links[current] {
                if component_of[neighbour] == usize::MAX {
                    component_of[neighbour] = island_index;
                    members.push(neighbour);
                    queue.push(neighbour);
                }
            }
        }
        members.sort_unstable();
        islands.push(members);
    }

    Ok(IslandLayout { islands })
}

/// Steps exactly the member bodies through their free-fall proposals and
/// returns the new states in member order.
pub fn advance_island_members(
    members: &[usize],
    bodies: &[RigidBody],
) -> Result<Vec<RigidBody>, IslandError> {
    members
        .iter()
        .map(|&index| {
            let proposal = bodies[index]
                .gravity_proposal()
                .map_err(|_| IslandError::Overflow)?;
            Ok(proposal.apply(&bodies[index]))
        })
        .collect()
}

/// Flat single-writer pass over every given body.
pub fn advance_awake_bodies(bodies: &[RigidBody]) -> Result<Vec<RigidBody>, IslandError> {
    advance_island_members(&(0..bodies.len()).collect::<Vec<_>>(), bodies)
}

fn body_digest(body: &RigidBody) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(body.mass_mg().to_be_bytes());
    for value in body.position_nm() {
        hasher.update(value.to_be_bytes());
    }
    for value in body.velocity_nm_per_s() {
        hasher.update(value.to_be_bytes());
    }
    hasher.finalize().into()
}

/// Explicit rest representation transition (INVARIANTS §12): suspending
/// snapshots the island's full state plus a content digest over the
/// suspension point. Nothing physical mutates — conservation is
/// structural — and resuming verifies the live world still matches the
/// suspension point bit exactly before the island re-enters the active
/// schedule.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RestSuspension {
    island_index: usize,
    members: Vec<usize>,
    world_len: usize,
    snapshot: Vec<RigidBody>,
    digest: [u8; 32],
}

impl RestSuspension {
    pub fn members(&self) -> &[usize] {
        &self.members
    }

    /// Indices of every body outside the suspended island, ascending.
    pub fn awake_indices(&self) -> Vec<usize> {
        let mut awake = Vec::new();
        let mut cursor = 0;
        for index in 0..self.world_len {
            if cursor < self.members.len() && self.members[cursor] == index {
                cursor += 1;
            } else {
                awake.push(index);
            }
        }
        awake
    }

    pub fn awake_bodies(&self, bodies: &[RigidBody]) -> Vec<RigidBody> {
        self.awake_indices()
            .into_iter()
            .map(|index| bodies[index])
            .collect()
    }

    pub fn restore_members(&self) -> Vec<RigidBody> {
        self.snapshot.clone()
    }
}

/// Suspends one island: a typed explicit transition with no heuristic
/// trigger, mirroring the durable ResolutionChanged discipline at the
/// mechanism layer.
pub fn suspend_island(
    layout: &IslandLayout,
    island_index: usize,
    bodies: &[RigidBody],
) -> Result<RestSuspension, IslandError> {
    let members = layout
        .islands()
        .get(island_index)
        .ok_or(IslandError::UnknownIsland { island_index })?;
    let snapshot = members.iter().map(|&index| bodies[index]).collect();
    let mut hasher = Sha256::new();
    hasher.update((island_index as u64).to_be_bytes());
    for &member in members {
        hasher.update(body_digest(&bodies[member]));
    }
    Ok(RestSuspension {
        island_index,
        members: members.clone(),
        world_len: bodies.len(),
        snapshot,
        digest: hasher.finalize().into(),
    })
}

/// Resumes a suspended island: the recorded digest must still match the
/// live states of all members; any drift is a typed rejection instead of
/// a plausible-looking substitution.
pub fn resume_island(suspension: &RestSuspension, bodies: &[RigidBody]) -> Result<(), IslandError> {
    if bodies.len() != suspension.world_len {
        return Err(IslandError::RestStateMismatch);
    }
    let mut hasher = Sha256::new();
    hasher.update((suspension.island_index as u64).to_be_bytes());
    for &member in &suspension.members {
        hasher.update(body_digest(&bodies[member]));
    }
    if hasher.finalize().as_slice() != suspension.digest {
        return Err(IslandError::RestStateMismatch);
    }
    Ok(())
}
