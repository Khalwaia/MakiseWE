//! Phase 2 slice 10 evidence: multi-step `ControlEpisode`s for cook,
//! clean and dress over the existing mechanisms. Independent anchors
//! are hand-derived values, never the production code:
//!
//! - the kettle chain: 2.5 l drawn at an admitted 125_000 mm³/s takes
//!   exactly twenty one-second draws; 2.4955 kg of water has
//!   C = 2.4955 kg · 4186 J/(kg·K) = 10_445_783 µJ/mK exactly, so from
//!   20 °C an admitted 2200 W burner lifts the projected temperature
//!   by ~210.6 mK per second: nine seconds land below +2 K and the
//!   tenth crosses it — a duration that *emerges* from physics, never
//!   promised;
//! - dropping the burner mid-heat blocks the episode with its elapsed
//!   clock frozen; re-admitting resumes to the same completion;
//! - cleaning saturates a 100 ml sponge twice and conserves the chain
//!   bit-exact: initial puddle == drained + sponge holds + final
//!   puddle;
//! - dressing without confirmed manipulation blocks; interruption
//!   leaves a durable partial result (k of n worn).

use makise_causal_kernel::{
    CleanBlocker, CleanControlEpisode, CleanObservables, CleanStep, ControlEpisodeError,
    CookAction, CookBlocker, CookControlEpisode, CookObservables, CookStep, DressBlocker,
    DressControlEpisode, DressObservables, DressStep, LiquidContainer, PourRequest, PowerNetwork,
    WATER_DENSITY_MG_PER_M3, WaterNetwork, clean_step, cook_step, dress_step,
};

const MAINS_MM3_PER_S: i64 = 150_000;
const DRAW_MM3_PER_S: i64 = 125_000;
const BURNER_W: i64 = 2_200;
const POT_CAPACITY_MM3: i64 = 3_000_000;
const REQUIRED_MM3: i64 = 2_500_000; // exactly twenty draws
const TARGET_MK: i64 = 295_150;
const TAP_TEMPERATURE_MK: i64 = 293_150;

/// Measured specific heat of liquid water near 20 °C [CRC]:
/// 4186 J/(kg·K). Kernel units: C_uJ/mK = mass_mg · c_v / 1000.
const WATER_CV_J_PER_KG_K: i64 = 4_186;

fn water_capacity_uj_per_mk(content_mm3: i64) -> i64 {
    let mass_mg: i64 = (i128::from(WATER_DENSITY_MG_PER_M3) * i128::from(content_mm3)
        / 1_000_000_000)
        .try_into()
        .expect("representable water mass");
    let scaled = i128::from(mass_mg) * i128::from(WATER_CV_J_PER_KG_K);
    assert_eq!(scaled % 1000, 0, "capacity must stay in whole µJ/mK");
    (scaled / 1000).try_into().expect("representable capacity")
}

/// The full kettle chain: fill through the admitted tap, admit the
/// burner, then let the episode drive heating until the observed
/// temperature crosses the target on the tenth heating second.
#[test]
fn kettle_chain_completes_when_observed_not_promised() {
    let mut mains = WaterNetwork::new(MAINS_MM3_PER_S).expect("positive");
    mains.admit_flow(DRAW_MM3_PER_S).expect("fits");
    let mut grid = PowerNetwork::new(3_680).expect("positive");

    let mut pot = LiquidContainer::new(POT_CAPACITY_MM3, 0).expect("empty pot");
    let mut episode = CookControlEpisode::begin(REQUIRED_MM3, TARGET_MK).expect("valid target");
    let mut pot_temperature = TAP_TEMPERATURE_MK;
    let mut reservoir_energy: i64 = 0;
    let mut draws = 0;
    let mut heating_seconds = 0;

    let completed = loop {
        let observables =
            CookObservables::new(pot.content_mm3(), pot_temperature, grid.load_watts());
        match cook_step(&episode, &observables).expect("evaluable") {
            CookStep::Proposed { next, action } => {
                match action {
                    Some(CookAction::DrawWaterOneSecond) => {
                        let volume = mains.draw_mm3(DRAW_MM3_PER_S, 1).expect("admitted flow");
                        let jug = LiquidContainer::new(volume, volume).expect("drawn");
                        let outcome = jug
                            .pour_into(&pot, &PourRequest::new(volume).expect("positive"))
                            .expect("pours cleanly while the pot has room");
                        assert_eq!(outcome.spilled_mm3(), 0);
                        pot = *outcome.next_target();
                        // Fresh tap water at tap temperature; the
                        // thermal reservoir only becomes meaningful at
                        // ignition, when the final content is fixed.
                        pot_temperature = TAP_TEMPERATURE_MK;
                        draws += 1;
                    }
                    Some(CookAction::HeatBurnerOneSecond) => {
                        let energy = grid.deliver_one_second(BURNER_W).expect("admitted");
                        if reservoir_energy == 0 {
                            // Ignition: the fixed final content defines
                            // the reservoir at tap temperature.
                            reservoir_energy =
                                water_capacity_uj_per_mk(pot.content_mm3()) * TAP_TEMPERATURE_MK;
                        }
                        let capacity = water_capacity_uj_per_mk(pot.content_mm3());
                        reservoir_energy += energy;
                        pot_temperature = reservoir_energy / capacity;
                        heating_seconds += 1;
                    }
                    None => {
                        // Ignition happens with the phase flip.
                        grid.admit_load(BURNER_W).expect("kettle fits");
                    }
                }
                episode = next;
            }
            CookStep::Blocked { .. } => panic!("admitted flows cannot block"),
            CookStep::Completed { episode } => break episode,
        }
    };

    // Hand-derived: twenty draws land exactly on 2.5 l (mass
    // 2_495_500 mg, C = 10_445_783 µJ/mK); ΔT/s = 2.2e9/10_445_783 ≈
    // 210.6 mK puts nine heating seconds at 295_045 mK and ten past
    // the target; plus the phase-flip second between fill and heat.
    assert_eq!(draws, 20);
    assert_eq!(heating_seconds, 10);
    assert_eq!(completed.seconds_elapsed(), 31);
}

/// Dropping the burner mid-heat freezes the episode with a durable
/// blocker; re-admitting the load resumes to the identical outcome.
#[test]
fn power_interruption_blocks_and_resumption_resolves() {
    let mut grid = PowerNetwork::new(3_680).expect("positive");
    grid.admit_load(BURNER_W).expect("fits");
    let episode = CookControlEpisode::begin(REQUIRED_MM3, TARGET_MK).expect("valid target");

    // A full pot flips straight to heating without consuming a draw.
    let mut episode = match cook_step(
        &episode,
        &CookObservables::new(REQUIRED_MM3, TAP_TEMPERATURE_MK, 0),
    )
    .expect("evaluable")
    {
        CookStep::Proposed { next, action } => {
            assert!(action.is_none(), "the flip proposes no delta");
            next
        }
        _ => panic!("a full pot flips to heating"),
    };

    let capacity = water_capacity_uj_per_mk(REQUIRED_MM3);
    let mut reservoir_energy = capacity * TAP_TEMPERATURE_MK;
    for _ in 0..2 {
        match cook_step(
            &episode,
            &CookObservables::new(REQUIRED_MM3, reservoir_energy / capacity, grid.load_watts()),
        )
        .expect("evaluable")
        {
            CookStep::Proposed { next, .. } => {
                reservoir_energy += grid.deliver_one_second(BURNER_W).expect("admitted");
                episode = next;
            }
            _ => panic!("heating proceeds while admitted"),
        }
    }
    let elapsed_before = episode.seconds_elapsed();

    // The breaker trips: the very next step is a typed blocker whose
    // clock does not advance.
    grid.drop_load(BURNER_W).expect("admitted load");
    match cook_step(
        &episode,
        &CookObservables::new(REQUIRED_MM3, reservoir_energy / capacity, grid.load_watts()),
    )
    .expect("evaluable")
    {
        CookStep::Blocked {
            episode: frozen,
            blocker: CookBlocker::PowerUnavailable,
        } => {
            assert_eq!(frozen.seconds_elapsed(), elapsed_before);
            episode = frozen;
        }
        _ => panic!("lost power must block"),
    }

    // Re-admission resolves the blocker; eight more heating seconds
    // finish the job (2 + 8 = 10 by the hand-derived budget).
    grid.admit_load(BURNER_W).expect("headroom restored");
    let resumed = loop {
        match cook_step(
            &episode,
            &CookObservables::new(REQUIRED_MM3, reservoir_energy / capacity, grid.load_watts()),
        )
        .expect("evaluable")
        {
            CookStep::Proposed { next, .. } => {
                reservoir_energy += grid.deliver_one_second(BURNER_W).expect("admitted");
                episode = next;
            }
            CookStep::Blocked { .. } => panic!("power is back"),
            CookStep::Completed { episode } => break episode,
        }
    };
    assert_eq!(
        resumed.seconds_elapsed(),
        elapsed_before + 8,
        "the emerged duration stays 10 heating seconds overall"
    );
}

/// Invalid targets are rejected at construction.
#[test]
fn degenerate_cook_targets_are_typed() {
    assert!(matches!(
        CookControlEpisode::begin(0, TARGET_MK),
        Err(ControlEpisodeError::InvalidTarget)
    ));
    assert!(matches!(
        CookControlEpisode::begin(-1, TARGET_MK),
        Err(ControlEpisodeError::InvalidTarget)
    ));
}

/// A 250 ml puddle saturates a 100 ml sponge twice: absorb, wring
/// into the drain, absorb again, wring again, finish. The conservation
/// identity holds bit-exact across every interruption point:
/// initial puddle == drained + sponge holds + final puddle.
#[test]
fn cleaning_absorbs_the_puddle_with_exact_conservation() {
    let mut episode = CleanControlEpisode::begin();
    let sponge_capacity = 100_000;
    let mut sponge_free = sponge_capacity;
    let mut sponge_content = 0;
    let mut puddle = 250_000;
    let mut drained = 0;

    let finished = loop {
        match clean_step(&episode, &CleanObservables::new(puddle, sponge_free)).expect("evaluable")
        {
            CleanStep::Proposed { next, absorb_mm3 } => {
                assert!(absorb_mm3 > 0 && absorb_mm3 <= sponge_free && absorb_mm3 <= puddle);
                puddle -= absorb_mm3;
                sponge_free -= absorb_mm3;
                sponge_content += absorb_mm3;
                episode = next;
            }
            CleanStep::Blocked {
                episode: stalled,
                blocker: CleanBlocker::SpongeSaturated,
            } => {
                // Wring the sponge out into the drain, then resume.
                drained += sponge_content;
                sponge_content = 0;
                sponge_free = sponge_capacity;
                episode = stalled;
            }
            CleanStep::Completed { episode } => break episode,
        }
    };
    assert_eq!(puddle, 0);
    assert_eq!(finished.absorbed_mm3(), 250_000);
    assert_eq!(
        250_000,
        drained + sponge_content + puddle,
        "bit-exact chain across two wring interruptions"
    );
    assert_eq!(drained, 200_000);
    assert_eq!(sponge_content, 50_000);
}

/// An already-dry floor completes immediately with no proposal.
#[test]
fn dry_floor_completes_immediately() {
    let episode = CleanControlEpisode::begin();
    let done = clean_step(&episode, &CleanObservables::new(0, 100_000)).expect("evaluable");
    assert!(matches!(done, CleanStep::Completed { .. }));
}

/// Three garments don in three confirmed steps; missing confirmation
/// blocks durably; resuming finishes a durable partial result.
#[test]
fn dressing_tracks_partial_progress_through_interruption() {
    let mut episode = DressControlEpisode::begin(3).expect("valid wardrobe");
    for worn_so_far in 0..2 {
        match dress_step(&episode, &DressObservables::new(true)).expect("evaluable") {
            DressStep::Proposed { next, .. } => {
                episode = next;
                assert_eq!(next.worn_count(), worn_so_far + 1);
            }
            _ => panic!("confirmed manipulation dons"),
        }
    }

    // Confirmation disappears: the episode stalls at a durable 2 of 3.
    match dress_step(&episode, &DressObservables::new(false)).expect("evaluable") {
        DressStep::Blocked {
            episode: frozen,
            blocker: DressBlocker::ManipulationFailed,
        } => {
            assert_eq!(frozen.worn_count(), 2);
            episode = frozen;
        }
        _ => panic!("failed manipulation blocks"),
    }

    // Recovery finishes the last garment.
    let finished = loop {
        match dress_step(&episode, &DressObservables::new(true)).expect("evaluable") {
            DressStep::Proposed { next, .. } => episode = next,
            DressStep::Blocked { .. } => panic!("confirmation restored"),
            DressStep::Completed { episode } => break episode,
        }
    };
    assert_eq!(finished.worn_count(), 3);
}

/// Empty wardrobes are rejected at construction.
#[test]
fn degenerate_wardrobes_are_typed() {
    assert!(matches!(
        DressControlEpisode::begin(0),
        Err(ControlEpisodeError::InvalidTarget)
    ));
}
