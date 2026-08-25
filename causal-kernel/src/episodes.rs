//! Multi-step `ControlEpisode`s for everyday apartment actions:
//! cooking, cleaning and dressing (plan 0003 slice 10).
//!
//! All three follow the durable walk-episode discipline
//! (INVARIANTS §57–58): the episode stores controller state — phase,
//! target constraint, observed progress, elapsed canonical seconds and
//! the last blocker — never a promised outcome or a guaranteed
//! duration. Each canonical second the caller feeds fresh world
//! observables back in; the episode proposes at most one declared
//! action whose delta the caller applies through the existing
//! mechanisms (water network, power grid, thermal ports, absorbent
//! accounting). Completion is an observed fact; blockers freeze the
//! clock; interruption leaves a durable partial result.
//!
//! Declared envelope: the controllers are `synthetic_fixture`
//! (ADR-0014) — every physical quantity they act on carries its own
//! provenance from the underlying mechanism slices. Dressing gates on
//! a caller-supplied manipulation confirmation because clothing
//! physics is a Phase 7 mechanism; the confirmation stands in for the
//! missing physical validator exactly as declared, not silently.

use thiserror::Error;

#[derive(Clone, Copy, Debug, Error, Eq, PartialEq)]
pub enum ControlEpisodeError {
    #[error("episode target must be positive")]
    InvalidTarget,
    #[error("checked arithmetic overflow in episode state")]
    Overflow,
}

// ---------------------------------------------------------------------
// Cooking

/// Cook phases: draw water until the pot holds the required amount,
/// then heat until the observed temperature crosses the target. The
/// transition between them is proposed with no physical delta, like a
/// zero-span weight shift in the walk episode.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CookPhase {
    Filling,
    Heating,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CookBlocker {
    /// The admitted burner disappeared mid-heat; nothing is delivered
    /// and no completion can be promised.
    PowerUnavailable,
}

/// Authoritative world snapshot fed back each canonical second: pot
/// content, pot temperature projection, and the load currently
/// admitted on the burner's network. Never stored by the episode.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CookObservables {
    pot_content_mm3: i64,
    pot_temperature_mk: i64,
    burner_watts: i64,
}

impl CookObservables {
    pub fn new(pot_content_mm3: i64, pot_temperature_mk: i64, burner_watts: i64) -> Self {
        Self {
            pot_content_mm3,
            pot_temperature_mk,
            burner_watts,
        }
    }

    pub fn pot_content_mm3(&self) -> i64 {
        self.pot_content_mm3
    }

    pub fn pot_temperature_mk(&self) -> i64 {
        self.pot_temperature_mk
    }

    pub fn burner_watts(&self) -> i64 {
        self.burner_watts
    }
}

/// One declared action per accepted second; the caller executes it
/// against the public mechanisms and feeds back fresh observables.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CookAction {
    /// Draw one second of admitted tap flow into the pot.
    DrawWaterOneSecond,
    /// Deliver one second of admitted burner power to the pot.
    HeatBurnerOneSecond,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CookControlEpisode {
    phase: CookPhase,
    required_mm3: i64,
    target_temperature_mk: i64,
    seconds_elapsed: u64,
    last_blocker: Option<CookBlocker>,
}

impl CookControlEpisode {
    pub fn begin(
        required_mm3: i64,
        target_temperature_mk: i64,
    ) -> Result<Self, ControlEpisodeError> {
        if required_mm3 <= 0 || target_temperature_mk <= 0 {
            return Err(ControlEpisodeError::InvalidTarget);
        }
        Ok(Self {
            phase: CookPhase::Filling,
            required_mm3,
            target_temperature_mk,
            seconds_elapsed: 0,
            last_blocker: None,
        })
    }

    pub fn phase(&self) -> CookPhase {
        self.phase
    }

    pub fn required_mm3(&self) -> i64 {
        self.required_mm3
    }

    pub fn target_temperature_mk(&self) -> i64 {
        self.target_temperature_mk
    }

    pub fn seconds_elapsed(&self) -> u64 {
        self.seconds_elapsed
    }

    pub fn last_blocker(&self) -> Option<CookBlocker> {
        self.last_blocker
    }

    fn advanced(&self) -> Result<Self, ControlEpisodeError> {
        Ok(Self {
            seconds_elapsed: self
                .seconds_elapsed
                .checked_add(1)
                .ok_or(ControlEpisodeError::Overflow)?,
            ..*self
        })
    }
}

/// One evaluated second: a proposal the authoritative writer may adopt
/// (with the successor episode), a recorded blocker over an unchanged
/// episode, or the observed completion. Completion is absorbing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CookStep {
    Proposed {
        next: CookControlEpisode,
        /// `None` marks the pure fill→heat phase flip.
        action: Option<CookAction>,
    },
    Blocked {
        episode: CookControlEpisode,
        blocker: CookBlocker,
    },
    Completed {
        episode: CookControlEpisode,
    },
}

fn blocked_cook(
    episode: &CookControlEpisode,
    blocker: CookBlocker,
) -> Result<CookStep, ControlEpisodeError> {
    Ok(CookStep::Blocked {
        episode: CookControlEpisode {
            last_blocker: Some(blocker),
            ..*episode
        },
        blocker,
    })
}

/// Evaluates one canonical second of cooking against fresh world
/// observables. Pure function: identical inputs yield identical steps,
/// so restart, replay and worker partitioning cannot drift the stream.
pub fn cook_step(
    episode: &CookControlEpisode,
    observables: &CookObservables,
) -> Result<CookStep, ControlEpisodeError> {
    match episode.phase {
        CookPhase::Filling => {
            if observables.pot_content_mm3() >= episode.required_mm3 {
                let next = CookControlEpisode {
                    phase: CookPhase::Heating,
                    ..episode.advanced()?
                };
                return Ok(CookStep::Proposed { next, action: None });
            }
            let next = CookControlEpisode {
                ..episode.advanced()?
            };
            Ok(CookStep::Proposed {
                next,
                action: Some(CookAction::DrawWaterOneSecond),
            })
        }
        CookPhase::Heating => {
            if observables.pot_temperature_mk() >= episode.target_temperature_mk {
                return Ok(CookStep::Completed { episode: *episode });
            }
            if observables.burner_watts() <= 0 {
                return blocked_cook(episode, CookBlocker::PowerUnavailable);
            }
            let next = CookControlEpisode {
                ..episode.advanced()?
            };
            Ok(CookStep::Proposed {
                next,
                action: Some(CookAction::HeatBurnerOneSecond),
            })
        }
    }
}

// ---------------------------------------------------------------------
// Cleaning

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CleanBlocker {
    SpongeSaturated,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CleanObservables {
    puddle_mm3: i64,
    sponge_free_mm3: i64,
}

impl CleanObservables {
    pub fn new(puddle_mm3: i64, sponge_free_mm3: i64) -> Self {
        Self {
            puddle_mm3,
            sponge_free_mm3,
        }
    }

    pub fn puddle_mm3(&self) -> i64 {
        self.puddle_mm3
    }

    pub fn sponge_free_mm3(&self) -> i64 {
        self.sponge_free_mm3
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CleanControlEpisode {
    absorbed_mm3: i64,
    seconds_elapsed: u64,
    last_blocker: Option<CleanBlocker>,
}

impl CleanControlEpisode {
    pub fn begin() -> Self {
        Self {
            absorbed_mm3: 0,
            seconds_elapsed: 0,
            last_blocker: None,
        }
    }

    /// Observed progress so far: everything this episode has soaked up.
    pub fn absorbed_mm3(&self) -> i64 {
        self.absorbed_mm3
    }

    pub fn seconds_elapsed(&self) -> u64 {
        self.seconds_elapsed
    }

    pub fn last_blocker(&self) -> Option<CleanBlocker> {
        self.last_blocker
    }

    fn advanced(&self) -> Result<Self, ControlEpisodeError> {
        Ok(Self {
            seconds_elapsed: self
                .seconds_elapsed
                .checked_add(1)
                .ok_or(ControlEpisodeError::Overflow)?,
            ..*self
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CleanStep {
    Proposed {
        next: CleanControlEpisode,
        /// Exact volume to move from puddle into sponge capacity.
        absorb_mm3: i64,
    },
    Blocked {
        episode: CleanControlEpisode,
        blocker: CleanBlocker,
    },
    Completed {
        episode: CleanControlEpisode,
    },
}

/// Absorbs the smaller of puddle and free sponge capacity per second;
/// the conservation identity puddle_before == absorbed + puddle_after
/// holds bit-exact across any interruption point.
pub fn clean_step(
    episode: &CleanControlEpisode,
    observables: &CleanObservables,
) -> Result<CleanStep, ControlEpisodeError> {
    if observables.puddle_mm3() <= 0 {
        return Ok(CleanStep::Completed { episode: *episode });
    }
    if observables.sponge_free_mm3() <= 0 {
        return Ok(CleanStep::Blocked {
            episode: CleanControlEpisode {
                last_blocker: Some(CleanBlocker::SpongeSaturated),
                ..*episode
            },
            blocker: CleanBlocker::SpongeSaturated,
        });
    }
    let absorb = observables.puddle_mm3().min(observables.sponge_free_mm3());
    let next = CleanControlEpisode {
        absorbed_mm3: episode
            .absorbed_mm3
            .checked_add(absorb)
            .ok_or(ControlEpisodeError::Overflow)?,
        ..episode.advanced()?
    };
    Ok(CleanStep::Proposed {
        next,
        absorb_mm3: absorb,
    })
}

// ---------------------------------------------------------------------
// Dressing

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DressBlocker {
    ManipulationFailed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DressObservables {
    manipulation_confirmed: bool,
}

impl DressObservables {
    pub fn new(manipulation_confirmed: bool) -> Self {
        Self {
            manipulation_confirmed,
        }
    }

    pub fn manipulation_confirmed(&self) -> bool {
        self.manipulation_confirmed
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DressControlEpisode {
    garment_total: i64,
    worn_count: i64,
    seconds_elapsed: u64,
    last_blocker: Option<DressBlocker>,
}

impl DressControlEpisode {
    pub fn begin(garment_total: i64) -> Result<Self, ControlEpisodeError> {
        if garment_total <= 0 {
            return Err(ControlEpisodeError::InvalidTarget);
        }
        Ok(Self {
            garment_total,
            worn_count: 0,
            seconds_elapsed: 0,
            last_blocker: None,
        })
    }

    /// Durable partial result: how many garments are already worn.
    pub fn worn_count(&self) -> i64 {
        self.worn_count
    }

    pub fn garment_total(&self) -> i64 {
        self.garment_total
    }

    pub fn seconds_elapsed(&self) -> u64 {
        self.seconds_elapsed
    }

    pub fn last_blocker(&self) -> Option<DressBlocker> {
        self.last_blocker
    }

    fn advanced(&self) -> Result<Self, ControlEpisodeError> {
        Ok(Self {
            seconds_elapsed: self
                .seconds_elapsed
                .checked_add(1)
                .ok_or(ControlEpisodeError::Overflow)?,
            ..*self
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DressStep {
    Proposed {
        next: DressControlEpisode,
        worn_count: i64,
    },
    Blocked {
        episode: DressControlEpisode,
        blocker: DressBlocker,
    },
    Completed {
        episode: DressControlEpisode,
    },
}

/// Dons at most one garment per confirmed second. The confirmation is
/// the caller-side physical evidence that the manipulation succeeded;
/// without it the episode blocks durably at its partial result.
pub fn dress_step(
    episode: &DressControlEpisode,
    observables: &DressObservables,
) -> Result<DressStep, ControlEpisodeError> {
    if episode.worn_count >= episode.garment_total {
        return Ok(DressStep::Completed { episode: *episode });
    }
    if !observables.manipulation_confirmed() {
        return Ok(DressStep::Blocked {
            episode: DressControlEpisode {
                last_blocker: Some(DressBlocker::ManipulationFailed),
                ..*episode
            },
            blocker: DressBlocker::ManipulationFailed,
        });
    }
    let worn = episode
        .worn_count
        .checked_add(1)
        .ok_or(ControlEpisodeError::Overflow)?;
    let next = DressControlEpisode {
        worn_count: worn,
        ..episode.advanced()?
    };
    Ok(DressStep::Proposed {
        next,
        worn_count: worn,
    })
}
