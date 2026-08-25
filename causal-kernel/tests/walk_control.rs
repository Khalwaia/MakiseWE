//! Phase 2 slice 12 evidence: the articulated walk ControlEpisode.
//!
//! The episode is a durable closed-loop controller state (INVARIANTS
//! §57–58): it stores the gait phase, the target constraint, observed
//! progress, elapsed canonical seconds and the last blocker; it never
//! stores a promised completion. Every second it re-validates the
//! actual walker observables through the slice-11 balance assessment,
//! proposes at most one declared kinematic transition, and reports
//! `Blocked` on lost balance — interruption is a first-class outcome,
//! and a fresh episode replans from the perturbed world.
//!
//! Independent anchors are the hand-derived gait table below (lane
//! ±150 mm, stride 400 mm, COM shift speed 250 mm/s per canonical
//! second under the declared max-axis proportional law: each shift
//! closes toward the stance foot with the whole-nanometre share
//! (Δx,Δz)·v/max(|Δx|,|Δz|)), never the production code.
//!
//! Canonical nine-second trace, coordinates in millimetres:
//!
//! ```text
//! sec phase      COM               completed
//! 1   WS{R}      (0,-150)          0     arrive over right foot
//! 2   Swing{L}   (0,-150)          0     left foot 0→400 (stride ahead)
//! 3   WS{L}      (250,37.5)        250   400:300 share 0.625
//! 4   WS{L}      (400,150)         400   arrive over left foot
//! 5   Swing{R}   (400,150)         400   right foot 0→800
//! 6   WS{R}      (650,-37.5)       650   400:-300 share 0.625
//! 7   WS{R}      (800,-150)        800   arrive over right foot
//! 8   Swing{L}   (800,-150)        800   left foot 400→1200
//! 9   WS{L}      (1050,37.5)       1050  target reached by this motion
//! 10  —          observed at the reevaluation boundary: COMPLETED
//! ```

use makise_causal_kernel::{
    Side, WalkBlocker, WalkControlEpisode, WalkError, WalkPhase, WalkStep, WalkerObservables,
    step_walk_episode,
};

const MM: i64 = 1_000_000;

/// The test owns the authoritative world copy; episodes only propose.
struct Walker {
    observables: WalkerObservables,
}

impl Walker {
    fn start() -> Self {
        Self {
            observables: WalkerObservables::new([0, 150 * MM], [0, -150 * MM], [0, 0]),
        }
    }

    /// Applies one proposal to the world and records the observable
    /// COM afterwards.
    fn apply(&mut self, step: &WalkStep) -> [i64; 2] {
        match step {
            WalkStep::Proposed {
                com_delta_nm,
                swing_delta_nm,
                swing_foot,
                ..
            } => {
                let com = self.observables.centre_of_mass_xz_nm();
                let mut left = self.observables.left_foot_xz_nm();
                let mut right = self.observables.right_foot_xz_nm();
                let target = match swing_foot {
                    Some(Side::Left) => &mut left,
                    Some(Side::Right) => &mut right,
                    None => &mut left, // unused: swing delta is zero
                };
                if swing_foot.is_some() {
                    *target = [target[0] + swing_delta_nm[0], target[1] + swing_delta_nm[1]];
                }
                self.observables = WalkerObservables::new(
                    left,
                    right,
                    [com[0] + com_delta_nm[0], com[1] + com_delta_nm[1]],
                );
                self.observables.centre_of_mass_xz_nm()
            }
            _ => self.observables.centre_of_mass_xz_nm(),
        }
    }
}

/// Drives the canonical nine-second gait to an exact completion.
#[test]
fn gait_reaches_the_target_in_nine_canonical_seconds() {
    let episode = WalkControlEpisode::begin(1000 * MM).expect("positive target");
    let mut walker = Walker::start();

    let expected_com_nm: [[i64; 2]; 9] = [
        [0, -150 * MM],
        [0, -150 * MM],
        [250 * MM, 37_500_000],
        [400 * MM, 150 * MM],
        [400 * MM, 150 * MM],
        [650 * MM, -37_500_000],
        [800 * MM, -150 * MM],
        [800 * MM, -150 * MM],
        [1050 * MM, 37_500_000],
    ];
    let expected_phases = [
        WalkPhase::WeightShift {
            stance: Side::Right,
        },
        WalkPhase::Swing { swing: Side::Left },
        WalkPhase::WeightShift { stance: Side::Left },
        WalkPhase::WeightShift { stance: Side::Left },
        WalkPhase::Swing { swing: Side::Right },
        WalkPhase::WeightShift {
            stance: Side::Right,
        },
        WalkPhase::WeightShift {
            stance: Side::Right,
        },
        WalkPhase::Swing { swing: Side::Left },
        WalkPhase::WeightShift { stance: Side::Left },
        WalkPhase::WeightShift { stance: Side::Left },
    ];
    let expected_completed_nm = [
        0, // after sec 1
        0, // after sec 2
        250 * MM,
        400 * MM,
        400 * MM,
        650 * MM,
        800 * MM,
        800 * MM,
        1050 * MM, // after sec 9
    ];

    let mut current = episode;
    for second in 0..9 {
        let step = step_walk_episode(&current, &walker.observables).expect("gait stays valid");
        let WalkStep::Proposed { ref next, .. } = step else {
            panic!("second {second} must propose, got {step:?}");
        };
        // next.phase() is the phase entering the following second.
        assert_eq!(
            next.phase(),
            expected_phases[second + 1],
            "entering phase after second {second}"
        );
        assert_eq!(
            next.completed_displacement_nm(),
            expected_completed_nm[second],
            "progress after second {second}"
        );
        let com = walker.apply(&step);
        assert_eq!(com, expected_com_nm[second]);
        current = next.clone();
    }

    // Tenth evaluation observes the recorded target at the reevaluation
    // boundary — after the motion that reached it.
    let step = step_walk_episode(&current, &walker.observables).expect("gait stays valid");
    let WalkStep::Completed {
        episode: ref final_episode,
    } = step
    else {
        panic!("tenth evaluation must complete, got {step:?}");
    };
    assert_eq!(final_episode.completed_displacement_nm(), 1050 * MM);
    assert_eq!(final_episode.seconds_elapsed(), 9);
    assert_eq!(final_episode.last_blocker(), None);
    let com = walker.apply(&step);
    assert_eq!(com, [1050 * MM, 37_500_000]);

    // A terminal episode proposes nothing further.
    let after =
        step_walk_episode(final_episode, &walker.observables).expect("terminal query is valid");
    assert!(
        matches!(after, WalkStep::Completed { .. }),
        "completion is an absorbing outcome"
    );
}

/// A huge target keeps returning honest partial results forever; the
/// episode never fabricates completion.
#[test]
fn unreachable_target_yields_partial_results_only() {
    let episode = WalkControlEpisode::begin(i64::MAX).expect("positive target");
    let mut walker = Walker::start();
    let mut current = episode;

    for _ in 0..20 {
        let step = step_walk_episode(&current, &walker.observables).expect("gait stays valid");
        let WalkStep::Proposed { ref next, .. } = step else {
            panic!("no completion may occur for an unreachable target");
        };
        current = next.clone();
        walker.apply(&step);
    }
    assert_eq!(current.seconds_elapsed(), 20);
    assert_eq!(current.completed_displacement_nm(), 2400 * MM);
    assert!(matches!(
        current.phase(),
        WalkPhase::WeightShift { stance: Side::Left }
    ));
}

/// Losing the stance point mid-swing blocks without mutating the
/// episode: the blocker is recorded, nothing advances.
#[test]
fn swing_perturbation_blocks_without_advancing() {
    let episode = WalkControlEpisode::begin(1000 * MM).expect("positive target");
    let mut walker = Walker::start();
    let mut current = episode;

    // Second 1: clean weight shift onto the right foot.
    let step = step_walk_episode(&current, &walker.observables).expect("valid");
    let WalkStep::Proposed { ref next, .. } = step else {
        panic!("first second must propose");
    };
    current = next.clone();
    walker.apply(&step);

    // Perturbation: the world nudges the COM one micrometre off the
    // stance foot before the swing second is evaluated.
    let com = walker.observables.centre_of_mass_xz_nm();
    walker.observables = WalkerObservables::new(
        walker.observables.left_foot_xz_nm(),
        walker.observables.right_foot_xz_nm(),
        [com[0], com[1] + 1_000],
    );

    let step = step_walk_episode(&current, &walker.observables).expect("valid");
    let WalkStep::Blocked {
        episode: unchanged,
        blocker,
    } = step
    else {
        panic!("perturbed swing must block, got {step:?}");
    };
    assert_eq!(blocker, WalkBlocker::BalanceLost);
    assert_eq!(unchanged.phase(), current.phase());
    assert_eq!(unchanged.seconds_elapsed(), current.seconds_elapsed());
    assert_eq!(
        unchanged.completed_displacement_nm(),
        current.completed_displacement_nm()
    );
    assert_eq!(unchanged.last_blocker(), Some(WalkBlocker::BalanceLost));
}

/// Replanning is a fresh episode over the perturbed world: one recovery
/// second recentres the COM, then the same gait walks home and lands
/// on the identical completion coordinate (1050 mm, 37.5 mm).
#[test]
fn replanned_episode_recovers_to_the_same_completion_point() {
    // Build the perturbed world from the blocker scenario.
    let mut walker = Walker::start();
    let first = WalkControlEpisode::begin(1000 * MM).expect("positive target");
    let step = step_walk_episode(&first, &walker.observables).expect("valid");
    walker.apply(&step);
    let com = walker.observables.centre_of_mass_xz_nm();
    walker.observables = WalkerObservables::new(
        walker.observables.left_foot_xz_nm(),
        walker.observables.right_foot_xz_nm(),
        [com[0], com[1] + 1_000],
    );

    // Replan: a fresh episode with the same total target.
    let mut current = WalkControlEpisode::begin(1000 * MM).expect("positive target");
    let mut completed = false;
    for _ in 0..12 {
        let step = step_walk_episode(&current, &walker.observables).expect("recovery stays valid");
        match step {
            WalkStep::Proposed { ref next, .. } => {
                walker.apply(&step);
                current = next.clone();
            }
            WalkStep::Completed { episode } => {
                let com = walker.observables.centre_of_mass_xz_nm();
                assert_eq!(
                    com,
                    [1050 * MM, 37_500_000],
                    "same completion coordinate as the clean gait"
                );
                assert_eq!(episode.completed_displacement_nm(), 1050 * MM);
                completed = true;
                break;
            }
            WalkStep::Blocked { .. } => panic!("recovery walk must not block"),
        }
    }
    assert!(completed, "replanned episode completes within the bound");
}

/// An episode whose stance foot is already under the COM skips the
/// shift with a zero-delta proposal and enters the swing phase.
#[test]
fn already_centred_weight_shift_flips_to_swing_without_motion() {
    let episode = WalkControlEpisode::begin(1000 * MM).expect("positive target");
    // COM already exactly over the right foot; feet straddle it.
    let observables = WalkerObservables::new([0, 300 * MM], [0, -300 * MM], [0, -300 * MM]);
    let step = step_walk_episode(&episode, &observables).expect("valid stance");
    let WalkStep::Proposed {
        next,
        com_delta_nm,
        swing_delta_nm,
        ..
    } = step
    else {
        panic!("centred stance must propose, got {step:?}");
    };
    assert_eq!(com_delta_nm, [0, 0]);
    assert_eq!(swing_delta_nm, [0, 0]);
    assert_eq!(next.phase(), WalkPhase::Swing { swing: Side::Left });
    assert_eq!(next.seconds_elapsed(), 1);
}

/// Declared controller law: shifts close along the stance direction
/// with a proportional whole-nanometre share; stances whose geometry
/// makes the share fractional are typed-rejected, never rounded.
#[test]
fn non_representable_weight_shift_is_typed() {
    let episode = WalkControlEpisode::begin(1000 * MM).expect("positive target");
    // Antipodal feet keep the COM on the segment (stable double
    // support), but the 100/300 direction has no whole-nanometre
    // 250 mm share: 100·250/300 is fractional.
    let observables = WalkerObservables::new([-100 * MM, -300 * MM], [100 * MM, 300 * MM], [0, 0]);
    let error = step_walk_episode(&episode, &observables)
        .expect_err("fractional shift share leaves the integer envelope");
    assert!(matches!(error, WalkError::NonRepresentableShift));
}

/// Targets must be positive displacements along the declared +x
/// direction; zero and negative are construction-time rejections.
#[test]
fn non_positive_targets_are_typed() {
    assert!(matches!(
        WalkControlEpisode::begin(0),
        Err(WalkError::InvalidTarget)
    ));
    assert!(matches!(
        WalkControlEpisode::begin(-5),
        Err(WalkError::InvalidTarget)
    ));
}

/// The episode transition stream is a pure function of its inputs:
/// evaluating the same (episode, observables) pair twice yields
/// identical outcomes bit for bit.
#[test]
fn stepping_is_deterministic_under_repetition() {
    let episode = WalkControlEpisode::begin(1000 * MM).expect("positive target");
    let walker = Walker::start();
    let first = step_walk_episode(&episode, &walker.observables).expect("valid");
    let second = step_walk_episode(&episode, &walker.observables).expect("valid");
    assert_eq!(first, second);
}
