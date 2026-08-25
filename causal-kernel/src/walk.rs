//! Durable walk `ControlEpisode` (ARCHITECTURE §8, INVARIANTS §57–58):
//! a closed-loop locomotion controller over the floor plane.
//!
//! The episode stores controller state — gait phase, the target
//! constraint, observed progress, elapsed canonical seconds and the
//! last blocker — never a promised outcome. Each canonical second the
//! caller feeds the actual walker observables back in; the episode
//! re-validates them through the balance slice and proposes at most one
//! declared kinematic transition. Lost balance blocks without advancing
//! (interruption is primary), completion is an observed fact, and a
//! fresh episode replans over the perturbed world.
//!
//! Declared envelope (provenance `synthetic_fixture` per ADR-0014;
//! walking straight along +x on the environment floor, bipedal):
//! - lane geometry and stride come from the observables and the
//!   constants below; all shifts are whole-nanometre proportional
//!   shares of the stance direction, otherwise typed-rejected;
//! - replay determinism: `step_walk_episode` is a pure function of its
//!   arguments — partitioning, restart or repetition cannot change the
//!   proposed stream.

use thiserror::Error;

use crate::balance::{BalanceAssessment, BalanceState, balance_assessment};

/// Centre-of-mass shift budget per canonical second, in nm/s.
pub const COM_SHIFT_SPEED_NM_PER_S: i64 = 250_000_000;
/// Forward displacement of one swing foot per canonical second, in nm.
pub const STRIDE_LENGTH_NM: i64 = 400_000_000;

#[derive(Clone, Copy, Debug, Error, Eq, PartialEq)]
pub enum WalkError {
    #[error("walk target must be a positive nanometre displacement")]
    InvalidTarget,
    #[error("weight shift share toward the stance foot is not representable in whole nanometres")]
    NonRepresentableShift,
    #[error("walker state is outside the declared validity range of this slice")]
    OutsideValidityRange,
    #[error("checked arithmetic overflow in walk control")]
    Overflow,
}

/// Which foot carries the current phase.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    Left,
    Right,
}

impl Side {
    fn other(self) -> Self {
        match self {
            Side::Left => Side::Right,
            Side::Right => Side::Left,
        }
    }
}

/// Gait phase: double support while the centre of mass travels to the
/// stance foot (`WeightShift`), single support while the named foot
/// swings forward (`Swing` carries the *swinging* side; the planted
/// foot is its opposite). The next phase always emerges from feedback,
/// never from a schedule of promised steps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WalkPhase {
    WeightShift { stance: Side },
    Swing { swing: Side },
}

/// Recorded interruption reason; stored on the episode so durable
/// readers see why the walker last stopped proposing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WalkBlocker {
    BalanceLost,
}

/// Durable closed-loop controller state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WalkControlEpisode {
    phase: WalkPhase,
    target_displacement_nm: i64,
    completed_displacement_nm: i64,
    seconds_elapsed: u64,
    last_blocker: Option<WalkBlocker>,
}

impl WalkControlEpisode {
    /// Starts an episode for a positive displacement target along the
    /// declared +x direction. The first phase shifts weight onto the
    /// right foot so the left foot can take the first swing.
    pub fn begin(target_displacement_nm: i64) -> Result<Self, WalkError> {
        if target_displacement_nm <= 0 {
            return Err(WalkError::InvalidTarget);
        }
        Ok(Self {
            phase: WalkPhase::WeightShift {
                stance: Side::Right,
            },
            target_displacement_nm,
            completed_displacement_nm: 0,
            seconds_elapsed: 0,
            last_blocker: None,
        })
    }

    pub fn phase(&self) -> WalkPhase {
        self.phase
    }
    pub fn target_displacement_nm(&self) -> i64 {
        self.target_displacement_nm
    }
    /// Observed progress so far: the signed sum of centre-of-mass
    /// displacements committed by accepted proposals.
    pub fn completed_displacement_nm(&self) -> i64 {
        self.completed_displacement_nm
    }
    /// Canonical seconds consumed by accepted proposals; blockers do
    /// not advance the clock. This is the reevaluation boundary.
    pub fn seconds_elapsed(&self) -> u64 {
        self.seconds_elapsed
    }
    pub fn last_blocker(&self) -> Option<WalkBlocker> {
        self.last_blocker
    }
}

/// Authoritative world snapshot fed back into the episode each second:
/// both foot contact points on the floor plane and the centre-of-mass
/// projection, in nanometres. The episode never stores these; it only
/// reads them fresh every step.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WalkerObservables {
    left_foot_xz_nm: [i64; 2],
    right_foot_xz_nm: [i64; 2],
    centre_of_mass_xz_nm: [i64; 2],
}

impl WalkerObservables {
    pub fn new(
        left_foot_xz_nm: [i64; 2],
        right_foot_xz_nm: [i64; 2],
        centre_of_mass_xz_nm: [i64; 2],
    ) -> Self {
        Self {
            left_foot_xz_nm,
            right_foot_xz_nm,
            centre_of_mass_xz_nm,
        }
    }

    pub fn left_foot_xz_nm(&self) -> [i64; 2] {
        self.left_foot_xz_nm
    }
    pub fn right_foot_xz_nm(&self) -> [i64; 2] {
        self.right_foot_xz_nm
    }
    pub fn centre_of_mass_xz_nm(&self) -> [i64; 2] {
        self.centre_of_mass_xz_nm
    }

    fn foot(&self, side: Side) -> [i64; 2] {
        match side {
            Side::Left => self.left_foot_xz_nm,
            Side::Right => self.right_foot_xz_nm,
        }
    }
}

/// One evaluated second: a proposal the authoritative writer may adopt
/// (with the successor episode), a recorded blocker over an unchanged
/// episode, or the observed completion. Completion is absorbing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WalkStep {
    Proposed {
        next: WalkControlEpisode,
        com_delta_nm: [i64; 2],
        swing_delta_nm: [i64; 2],
        /// The foot the swing delta applies to, if any.
        swing_foot: Option<Side>,
    },
    Blocked {
        episode: WalkControlEpisode,
        blocker: WalkBlocker,
    },
    Completed {
        episode: WalkControlEpisode,
    },
}

fn assess(
    contacts: [[i64; 2]; 2],
    count: usize,
    com: [i64; 2],
) -> Result<BalanceAssessment, WalkError> {
    let verdict = balance_assessment(&contacts[..count], com);
    // The bipedal slices above can never exceed the balance slice's
    // two-contact validity range; the mapping stays defensive anyway.
    verdict.map_err(|_| WalkError::OutsideValidityRange)
}

fn single_support_stable(stance: [i64; 2], com: [i64; 2]) -> Result<bool, WalkError> {
    let verdict = balance_assessment(&[stance], com);
    verdict
        .map(|assessment| assessment.state() == BalanceState::Stable)
        .map_err(|_| WalkError::OutsideValidityRange)
}

fn shifted(com: [i64; 2], delta: [i128; 2]) -> Result<[i64; 2], WalkError> {
    let mut out = [0i64; 2];
    for axis in 0..2 {
        let moved = i128::from(com[axis])
            .checked_add(delta[axis])
            .ok_or(WalkError::Overflow)?;
        out[axis] = moved.try_into().map_err(|_| WalkError::Overflow)?;
    }
    Ok(out)
}

fn advance_seconds(episode: &WalkControlEpisode) -> Result<WalkControlEpisode, WalkError> {
    Ok(WalkControlEpisode {
        seconds_elapsed: episode
            .seconds_elapsed
            .checked_add(1)
            .ok_or(WalkError::Overflow)?,
        ..episode.clone()
    })
}

fn blocked(episode: &WalkControlEpisode, blocker: WalkBlocker) -> Result<WalkStep, WalkError> {
    Ok(WalkStep::Blocked {
        episode: WalkControlEpisode {
            last_blocker: Some(blocker),
            ..episode.clone()
        },
        blocker,
    })
}

/// Evaluates one canonical second of the walk against fresh world
/// observables. Pure function: identical inputs yield identical steps,
/// so restart, replay and worker partitioning cannot drift the stream.
///
/// Completion is an observed fact at the reevaluation boundary: an
/// episode whose recorded progress has reached the target reports
/// `Completed` instead of proposing further motion — never before the
/// motion that reached it has been proposed and adopted.
pub fn step_walk_episode(
    episode: &WalkControlEpisode,
    observables: &WalkerObservables,
) -> Result<WalkStep, WalkError> {
    if episode.completed_displacement_nm >= episode.target_displacement_nm {
        return Ok(WalkStep::Completed {
            episode: episode.clone(),
        });
    }
    let com = observables.centre_of_mass_xz_nm();
    let left = observables.left_foot_xz_nm();
    let right = observables.right_foot_xz_nm();

    match episode.phase {
        WalkPhase::WeightShift { stance } => {
            let stance_pos = observables.foot(stance);
            let double_support_stable =
                assess([left, right], 2, com)?.state() == BalanceState::Stable;
            if !double_support_stable {
                return blocked(episode, WalkBlocker::BalanceLost);
            }
            let mut delta = [0i128; 2];
            for axis in 0..2 {
                delta[axis] = i128::from(stance_pos[axis]) - i128::from(com[axis]);
            }
            let span = delta[0].abs().max(delta[1].abs());
            if span > i128::from(COM_SHIFT_SPEED_NM_PER_S) {
                // Proportional whole-nanometre share of the stance
                // direction; fractional shares are outside the integer
                // envelope of the declared controller law.
                for component in &mut delta {
                    let scaled = (*component)
                        .checked_mul(i128::from(COM_SHIFT_SPEED_NM_PER_S))
                        .ok_or(WalkError::Overflow)?;
                    if scaled % span != 0 {
                        return Err(WalkError::NonRepresentableShift);
                    }
                    *component = scaled / span;
                }
            }
            let new_com = shifted(com, delta)?;
            // A full-budget shift lands exactly on the stance foot; a
            // zero span means the COM is already centred and the phase
            // flips to the swing without motion.
            let arrived = if span == 0 {
                true
            } else {
                stance_pos == new_com
            };

            let progress: i64 = delta[0].try_into().map_err(|_| WalkError::Overflow)?;
            let completed = i128::from(episode.completed_displacement_nm)
                .checked_add(i128::from(progress))
                .ok_or(WalkError::Overflow)?;
            let completed: i64 = completed.try_into().map_err(|_| WalkError::Overflow)?;

            let advanced = advance_seconds(episode)?;
            let next = WalkControlEpisode {
                phase: if arrived {
                    WalkPhase::Swing {
                        swing: stance.other(),
                    }
                } else {
                    WalkPhase::WeightShift { stance }
                },
                completed_displacement_nm: completed,
                ..advanced
            };
            Ok(WalkStep::Proposed {
                next,
                com_delta_nm: [
                    delta[0].try_into().map_err(|_| WalkError::Overflow)?,
                    delta[1].try_into().map_err(|_| WalkError::Overflow)?,
                ],
                swing_delta_nm: [0, 0],
                swing_foot: None,
            })
        }
        WalkPhase::Swing { swing } => {
            // Single support: the planted foot is the swing foot's
            // opposite; the COM must sit exactly over it.
            let planted = observables.foot(swing.other());
            if !single_support_stable(planted, com)? {
                return blocked(episode, WalkBlocker::BalanceLost);
            }
            let swing_foot = observables.foot(swing);
            // Declared gait geometry: the swing foot lands one stride
            // ahead of the planted foot, keeping the stance separation
            // constant at STRIDE_LENGTH_NM.
            let landing_x = i128::from(planted[0])
                .checked_add(i128::from(STRIDE_LENGTH_NM))
                .ok_or(WalkError::Overflow)?;
            let landing_x: i64 = landing_x.try_into().map_err(|_| WalkError::Overflow)?;
            let swing_delta = landing_x
                .checked_sub(swing_foot[0])
                .ok_or(WalkError::Overflow)?;
            let advanced = advance_seconds(episode)?;
            let next = WalkControlEpisode {
                phase: WalkPhase::WeightShift { stance: swing },
                ..advanced
            };
            Ok(WalkStep::Proposed {
                next,
                com_delta_nm: [0, 0],
                swing_delta_nm: [swing_delta, 0],
                swing_foot: Some(swing),
            })
        }
    }
}
