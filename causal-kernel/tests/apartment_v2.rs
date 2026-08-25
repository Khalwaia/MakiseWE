//! Phase 2 acceptance scenario: один scripted Human в apartment-v2 —
//! walk к кухне → взять кастрюлю → наполнить водой → поставить на плиту
//! → включить нагрев → непреднамеренный spill (boil-over) → clean →
//! dress. Каждая цепь показывает causes через canonical transitions,
//! unit-typed deltas и bit-exact conservation; plan 0003 §4 negative
//! tests выполняются внутри той же хроники:
//!
//! - grasp без контакта и с недостаточным friction cone отклоняется;
//! - spill сохраняет total water mass bit-exact по всему сценарию;
//! - отключение питания останавливает нагрев typed blocker'ом без
//!   promised outcome, возобновление сходится в тот же бюджет;
//! - interruption ControlEpisode оставляет durable partial state без
//!   completion mutation;
//! - restart даёт identical event stream, bodies, organism state,
//!   identical state hash и idempotent replay старых request id.

use makise_causal_kernel::{
    BoxCollider, CleanControlEpisode, CleanObservables, CleanStep, CommitError, CommitRequest,
    ContactError, CookBlocker, CookControlEpisode, CookObservables, CookStep, DressBlocker,
    DressControlEpisode, DressObservables, DressStep, EventCursor, EventQuery, GraspRequest,
    LiquidContainer, OpenSpec, PourRequest, PowerNetwork, ProjectionRequest, RecoveryStatus,
    RigidBody, Side, StateHash, StorageLocation, TimelineId, WATER_DENSITY_MG_PER_M3,
    WalkControlEpisode, WalkStep, WalkerObservables, WaterNetwork, WorldEngine, WorldId,
    clean_step, contact_proposal, cook_step, dress_step, grasp_proposal, step_walk_episode,
};

const MM: i64 = 1_000_000;
const MAINS_MM3_PER_S: i64 = 150_000;
const DRAW_MM3_PER_S: i64 = 125_000; // twenty draws land on exactly 2_500_000
const BURNER_W: i64 = 2_200;
const POT_CAPACITY_MM3: i64 = 3_000_000;
const REQUIRED_MM3: i64 = 2_500_000;
const TARGET_MK: i64 = 295_150;
const TAP_TEMPERATURE_MK: i64 = 293_150;
/// Measured specific heat of liquid water near 20 °C [CRC].
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

/// The scripted Human's kinematic world copy; episodes only propose.
struct Walker {
    left_foot: [i64; 2],
    right_foot: [i64; 2],
    com: [i64; 2],
}

impl Walker {
    fn start() -> Self {
        Self {
            left_foot: [0, 150 * MM],
            right_foot: [0, -150 * MM],
            com: [0, 0],
        }
    }

    fn observables(&self) -> WalkerObservables {
        WalkerObservables::new(self.left_foot, self.right_foot, self.com)
    }

    fn apply(
        &mut self,
        com_delta_nm: [i64; 2],
        swing_delta_nm: [i64; 2],
        swing_foot: Option<Side>,
    ) {
        for (axis, delta) in com_delta_nm.iter().enumerate() {
            self.com[axis] += delta;
        }
        if let Some(side) = swing_foot {
            let foot = match side {
                Side::Left => &mut self.left_foot,
                Side::Right => &mut self.right_foot,
            };
            foot[0] += swing_delta_nm[0];
        }
    }
}

/// Single-writer bookkeeping: every commit takes the current head as
/// its expected version and adopts the receipt's.
struct Scenario {
    engine: WorldEngine,
    expected: u64,
}

impl Scenario {
    fn commit(&mut self, tag: &str, build: impl FnOnce(&str, u64) -> CommitRequest) {
        let receipt = self
            .engine
            .commit(build(tag, self.expected))
            .expect("commit");
        self.expected = receipt.timeline_version();
    }
}

#[test]
fn apartment_v2_end_to_end_with_negative_tests() {
    let directory = tempfile::tempdir().expect("temp dir");
    let storage_path = directory.path().join("apartment-v2.sqlite");
    let spec = || {
        OpenSpec::new(
            WorldId::new("apartment-v2").expect("valid"),
            TimelineId::new("main").expect("valid"),
        )
    };
    let (engine, _) =
        WorldEngine::open(spec(), StorageLocation::sqlite(&storage_path)).expect("open");
    let mut scenario = Scenario {
        engine,
        expected: 0,
    };

    // ----------------------------------------------------------------
    // walk к кухне: closed-loop episode over synthetic observables.
    let target_nm = 800 * MM;
    let mut episode = WalkControlEpisode::begin(target_nm).expect("valid target");
    let mut walker = Walker::start();
    let mut walk_seconds = 0u64;
    let walked = loop {
        match step_walk_episode(&episode, &walker.observables()).expect("evaluable") {
            WalkStep::Proposed {
                next,
                com_delta_nm,
                swing_delta_nm,
                swing_foot,
            } => {
                walker.apply(com_delta_nm, swing_delta_nm, swing_foot);
                episode = next;
                walk_seconds += 1;
                assert!(walk_seconds < 50, "the gait must terminate");
            }
            WalkStep::Completed { episode } => break episode,
            WalkStep::Blocked { .. } => panic!("a flat floor cannot block"),
        }
    };
    assert_eq!(walked.completed_displacement_nm(), target_nm);
    scenario.commit("walk-to-kitchen", |tag, version| {
        CommitRequest::advance_to(tag, version, walk_seconds as i64)
    });

    // ----------------------------------------------------------------
    // Negative: grasp without a contact is typed-rejected, and so is a
    // grip whose friction cone cannot close over the weight.
    let pot_body = RigidBody::new(
        1_500_000, // 1.5 kg empty pot
        [900 * MM, 450 * MM, 300 * MM],
        [0; 3],
        [0, -20 * MM, 0],
        [1_500_000, 2_000_000, 1_700_000],
        [0; 3],
    )
    .expect("valid pot");
    let hand = RigidBody::new(
        400_000,
        [905 * MM, 470 * MM, 300 * MM],
        [0; 3],
        [0; 3],
        [100_000, 120_000, 110_000],
        [0; 3],
    )
    .expect("valid hand");
    let pot_collider = BoxCollider::new([80 * MM, 50 * MM, 80 * MM]).expect("collider");
    let hand_collider = BoxCollider::new([40 * MM, 15 * MM, 40 * MM]).expect("collider");

    assert!(matches!(
        grasp_proposal(None, &pot_body, &GraspRequest::new(1, 1).expect("request")),
        Err(ContactError::GraspRequiresContact)
    ));
    let manifold = contact_proposal(&hand, &hand_collider, &pot_body, &pot_collider)
        .expect("proposable")
        .expect("overlapping colliders touch");
    assert!(matches!(
        grasp_proposal(
            Some(&manifold),
            &pot_body,
            &GraspRequest::new(3_000_000_000_000_000, 100_000).expect("weak grip")
        ),
        Err(ContactError::FrictionInfeasible)
    ));
    grasp_proposal(
        Some(&manifold),
        &pot_body,
        &GraspRequest::new(30_000_000_000_000_000, 600_000).expect("strong grip"),
    )
    .expect("the cone closes");

    // ----------------------------------------------------------------
    // наполнить водой: admitted tap flow, exact volumes, zero spill
    // while the pot still has room.
    let mut mains = WaterNetwork::new(MAINS_MM3_PER_S).expect("positive main");
    mains.admit_flow(DRAW_MM3_PER_S).expect("fits");
    let mut pot_water = LiquidContainer::new(POT_CAPACITY_MM3, 0).expect("empty pot");
    let mut drawn_total = 0;
    while pot_water.content_mm3() < REQUIRED_MM3 {
        let volume = mains.draw_mm3(DRAW_MM3_PER_S, 1).expect("admitted flow");
        drawn_total += volume;
        let jug = LiquidContainer::new(volume, volume).expect("drawn");
        let outcome = jug
            .pour_into(&pot_water, &PourRequest::new(volume).expect("positive"))
            .expect("pours cleanly with headroom");
        assert_eq!(outcome.spilled_mm3(), 0);
        pot_water = *outcome.next_target();
    }
    assert_eq!(drawn_total, REQUIRED_MM3);
    let fill_seconds = drawn_total / DRAW_MM3_PER_S;

    // поставить на плиту: the stove and the loaded pot become durable
    // timeline state through the single mutation path.
    scenario.commit("place-stove", |tag, version| {
        CommitRequest::place_body(
            tag,
            version,
            "stove",
            RigidBody::new(
                30_000_000,
                [1000 * MM, 0, 400 * MM],
                [0; 3],
                [0; 3],
                [12_000_000, 15_000_000, 13_000_000],
                [0; 3],
            )
            .expect("stove"),
        )
    });
    let pot_on_stove = RigidBody::new(
        pot_body.mass_mg(),
        [1000 * MM, 480 * MM, 400 * MM],
        [0; 3],
        pot_body.center_of_mass_offset_nm(),
        pot_body.principal_inertia_mgm2(),
        [0; 3],
    )
    .expect("placed pot");
    let mut pot_request_holder: Option<CommitRequest> = None;
    scenario.commit("pot-on-stove", |tag, version| {
        let request = CommitRequest::place_body(tag, version, "cooking-pot", pot_on_stove);
        pot_request_holder = Some(request.clone());
        request
    });
    let pot_request = pot_request_holder.expect("placement captured");
    scenario.commit("fill-interval", |tag, version| {
        CommitRequest::advance_to(tag, version, fill_seconds)
    });

    // включить нагрев: the episode drives burner-seconds; the breaker
    // trips mid-heat and the budget survives the interruption.
    let mut grid = PowerNetwork::new(3_680).expect("grid");
    grid.admit_load(BURNER_W).expect("burner fits");
    let cook = CookControlEpisode::begin(REQUIRED_MM3, TARGET_MK).expect("target");
    let mut cook = match cook_step(
        &cook,
        &CookObservables::new(REQUIRED_MM3, TAP_TEMPERATURE_MK, grid.load_watts()),
    )
    .expect("evaluable")
    {
        CookStep::Proposed { next, action } => {
            assert!(action.is_none(), "the flip proposes no delta");
            next
        }
        _ => panic!("a full pot flips to heating"),
    };
    let capacity = water_capacity_uj_per_mk(REQUIRED_MM3); // 10_445_783 exactly
    let mut reservoir_energy = capacity * TAP_TEMPERATURE_MK;
    let mut heating_seconds = 0u64;
    let cooked = loop {
        match cook_step(
            &cook,
            &CookObservables::new(REQUIRED_MM3, reservoir_energy / capacity, grid.load_watts()),
        )
        .expect("evaluable")
        {
            CookStep::Proposed { next, .. } => {
                reservoir_energy += grid.deliver_one_second(BURNER_W).expect("admitted");
                cook = next;
                heating_seconds += 1;
                // Trip the breaker after four heating seconds.
                if heating_seconds == 4 {
                    grid.drop_load(BURNER_W).expect("admitted load");
                }
            }
            CookStep::Blocked {
                episode: frozen,
                blocker: CookBlocker::PowerUnavailable,
            } => {
                assert_eq!(frozen.seconds_elapsed(), cook.seconds_elapsed());
                cook = frozen;
                grid.admit_load(BURNER_W).expect("headroom restored");
            }
            CookStep::Completed { episode } => break episode,
        }
    };
    // Hand-derived: ΔT/s ≈ 210.6 mK crosses +2 K on the tenth heating
    // second, interruption notwithstanding; metered equals applied.
    assert_eq!(heating_seconds, 10);
    assert_eq!(cooked.seconds_elapsed(), 11, "flip second plus ten burns");
    assert_eq!(
        reservoir_energy / capacity,
        TARGET_MK + 106,
        "293_150 + floor(22e9 / 10_445_783)"
    );
    assert_eq!(
        grid.cumulative_delivered_uj(),
        heating_seconds as i64 * BURNER_W * 1_000_000
    );
    scenario.commit("heat-interval", |tag, version| {
        CommitRequest::advance_to(tag, version, heating_seconds as i64)
    });

    // непреднамеренный spill: boil-over pours into an already-full
    // cup; everything given becomes first-class floor puddle.
    let full_cup = LiquidContainer::new(200_000, 200_000).expect("full cup");
    let boil_over = pot_water
        .pour_into(&full_cup, &PourRequest::new(300_000).expect("positive"))
        .expect("valid pour");
    assert_eq!(boil_over.transferred_mm3(), 0);
    let mut puddle_mm3 = boil_over.spilled_mm3();
    assert_eq!(puddle_mm3, 300_000);
    let pot_water = *boil_over.next_source();

    // clean: a 150 ml sponge absorbs the puddle across two wrings.
    let sponge_capacity = 150_000;
    let mut sponge_free = sponge_capacity;
    let mut sponge_content = 0;
    let mut drained = 0;
    let mut clean = CleanControlEpisode::begin();
    let cleaned = loop {
        match clean_step(&clean, &CleanObservables::new(puddle_mm3, sponge_free))
            .expect("evaluable")
        {
            CleanStep::Proposed { next, absorb_mm3 } => {
                puddle_mm3 -= absorb_mm3;
                sponge_free -= absorb_mm3;
                sponge_content += absorb_mm3;
                clean = next;
            }
            CleanStep::Blocked {
                episode: stalled, ..
            } => {
                drained += sponge_content;
                sponge_content = 0;
                sponge_free = sponge_capacity;
                clean = stalled;
            }
            CleanStep::Completed { episode } => break episode,
        }
    };
    assert_eq!(cleaned.absorbed_mm3(), 300_000);

    // Bit-exact scenario water ledger: every drawn millilitre is
    // accounted for — no silent loss anywhere (INVARIANTS §18).
    assert_eq!(
        drawn_total,
        pot_water.content_mm3() + drained + sponge_content + puddle_mm3
    );
    assert_eq!(pot_water.content_mm3(), 2_200_000);
    // The second sponge-load is still held when the floor runs dry:
    // drained 150_000 + sponge 150_000 close the 300_000 chain.
    assert_eq!(drained, 150_000);
    assert_eq!(sponge_content, 150_000);
    assert_eq!(puddle_mm3, 0);

    // dress: confirmed manipulation dons; failed confirmation stalls
    // at a durable partial without any completion mutation.
    let mut dress = DressControlEpisode::begin(2).expect("wardrobe");
    dress = match dress_step(&dress, &DressObservables::new(true)).expect("evaluable") {
        DressStep::Proposed { next, .. } => next,
        _ => panic!("confirmed manipulation dons"),
    };
    match dress_step(&dress, &DressObservables::new(false)).expect("evaluable") {
        DressStep::Blocked {
            episode: frozen,
            blocker: DressBlocker::ManipulationFailed,
        } => {
            assert_eq!(frozen.worn_count(), 1, "durable partial result");
            dress = frozen;
        }
        _ => panic!("missing confirmation blocks"),
    }
    let dressed = loop {
        match dress_step(&dress, &DressObservables::new(true)).expect("evaluable") {
            DressStep::Proposed { next, .. } => dress = next,
            DressStep::Blocked { .. } => panic!("confirmation restored"),
            DressStep::Completed { episode } => break episode,
        }
    };
    assert_eq!(dressed.worn_count(), 2);

    // ----------------------------------------------------------------
    // Restart/replay parity over the whole chain.
    let query = || EventQuery::new(EventCursor::start(), 1000).expect("valid query");
    let stream_before = scenario.engine.events(query()).expect("readable");
    let organism_before = scenario.engine.organism().cloned();
    let body_before = scenario
        .engine
        .body("cooking-pot")
        .expect("readable")
        .expect("present");
    let ambient_hash_before = StateHash::of(
        organism_before
            .as_ref()
            .expect("organism")
            .ambient_reservoir(),
    );
    let projection_version = scenario
        .engine
        .project(ProjectionRequest::current())
        .expect("projection")
        .timeline_version();

    drop(scenario);
    let (mut reopened, recovery) =
        WorldEngine::open(spec(), StorageLocation::sqlite(&storage_path)).expect("reopen");
    assert_eq!(recovery.status(), RecoveryStatus::Recovered);

    let stream_after = reopened.events(query()).expect("readable");
    assert_eq!(
        stream_before.events(),
        stream_after.events(),
        "identical canonical transition stream across restart"
    );
    assert_eq!(stream_after.next_cursor(), stream_before.next_cursor());
    assert_eq!(
        reopened.body("cooking-pot").expect("readable"),
        Some(body_before),
        "bit-exact body restore"
    );
    assert_eq!(reopened.organism(), organism_before.as_ref());
    let ambient_hash_after =
        StateHash::of(reopened.organism().expect("organism").ambient_reservoir());
    assert_eq!(
        ambient_hash_before, ambient_hash_after,
        "identical declared state hash"
    );
    assert_eq!(
        reopened
            .project(ProjectionRequest::current())
            .expect("projection")
            .timeline_version(),
        projection_version
    );

    // Replay discipline after reopen: the identical placement request
    // replays its receipt; a different pose under the same request id
    // stays an idempotency conflict.
    let replayed = reopened.commit(pot_request.clone()).expect("retry");
    assert!(replayed.replayed_request());
    let conflicting = reopened.commit(CommitRequest::place_body(
        "pot-on-stove",
        0,
        "cooking-pot",
        RigidBody::new(
            1_500_000,
            [0; 3],
            [0; 3],
            [0, -20 * MM, 0],
            [1_500_000, 2_000_000, 1_700_000],
            [0; 3],
        )
        .expect("different pose"),
    ));
    assert!(matches!(conflicting, Err(CommitError::IdempotencyConflict)));
}
