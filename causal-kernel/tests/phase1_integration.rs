use makise_causal_kernel::{
    CognitiveDisposition, CognitiveGate, CommitRequest, CortexProposal, Morphotype,
    NeuralPopulation, OpenSpec, OrganismState, SleepPhase, StorageLocation, TimelineId,
    WorldEngine, WorldId,
};

fn spec(name: &str) -> OpenSpec {
    OpenSpec::new(
        WorldId::new(format!("{name}-world")).expect("valid"),
        TimelineId::new(format!("{name}-timeline")).expect("valid"),
    )
}

/// 24h scenario: day of waking activity → dinner → accepted bedtime
/// intention → physiological sleep onset in the night window → spontaneous
/// wake in the morning window. Proves causal integration across organism
/// slices with exact conservation.
#[test]
fn phase1_24h_human_scenario_conserves_energy_and_produces_intention() {
    use makise_causal_kernel::{
        BASELINE_AMBIENT_INTERNAL_ENERGY_UJ, BASELINE_CORE_INTERNAL_ENERGY_UJ,
        INITIAL_CHEMICAL_STORE_UJ,
    };

    let dir = tempfile::tempdir().expect("temp");
    let (mut engine, _) = WorldEngine::open(
        spec("p1-human"),
        StorageLocation::sqlite(dir.path().join("t.sqlite")),
    )
    .expect("open");

    // Day of waking activity: 06:00 wake narrative ends at 22:00 canonical.
    engine
        .commit(CommitRequest::advance_to("day", 0, 79_200))
        .expect("day");
    assert_eq!(engine.sleep_phase(), SleepPhase::Awake);
    let chemical_after_day = engine.organism().unwrap().chemical_store_uj();
    assert!(
        chemical_after_day < INITIAL_CHEMICAL_STORE_UJ,
        "waking metabolism must draw on the chemical store"
    );

    // Dinner funds the overnight fast; ingestion adds exactly declared
    // chemical energy.
    let dinner_energy_uj = 3_000_000_000_000i64;
    engine
        .commit(CommitRequest::ingest_food("dinner", 1, dinner_energy_uj))
        .unwrap();

    // Accepted bedtime intention through the cognitive gate pipeline.
    let gate = CognitiveGate::new();
    let proposal = CortexProposal::new(
        "sleep-intent",
        "consciousness-makise",
        "frame-evening",
        "sleep because fatigue is high",
    );
    assert!(matches!(
        gate.evaluate(&proposal),
        CognitiveDisposition::Accepted { .. }
    ));
    let _intention = gate.adopt_intention(&proposal).expect("accepted");

    engine
        .commit(CommitRequest::accept_sleep_intention("sleep", 2))
        .unwrap();

    // One hour into the night window the organism has fallen asleep.
    engine
        .commit(CommitRequest::advance_to("onset", 3, 3_600))
        .unwrap();
    assert_eq!(engine.sleep_phase(), SleepPhase::Asleep);

    // Overnight fast: recovery clears the debt inside the morning window
    // and the organism wakes spontaneously before late morning.
    engine
        .commit(CommitRequest::advance_to("overnight", 4, 43_200))
        .unwrap();
    assert_eq!(
        engine.sleep_phase(),
        SleepPhase::Awake,
        "cleared recovery debt inside the morning window must wake the organism"
    );

    // Exact conservation: accounted chemical+core energy plus ambient
    // reservoir equals the constructed baseline plus ingested energy.
    let accounted_before_restart = engine.organism().unwrap().total_accounted_uj();
    let ambient_before_restart = engine.organism().unwrap().ambient_internal_energy_uj();
    let expected_total = INITIAL_CHEMICAL_STORE_UJ
        + BASELINE_CORE_INTERNAL_ENERGY_UJ
        + BASELINE_AMBIENT_INTERNAL_ENERGY_UJ
        + dinner_energy_uj;
    assert_eq!(
        accounted_before_restart + ambient_before_restart,
        expected_total,
        "energy cannot vanish: burn must land as heat in body or room"
    );

    // Restart parity: reopen and verify state survives exactly.
    std::mem::drop(engine);
    let (reopened, _) = WorldEngine::open(
        spec("p1-human"),
        StorageLocation::sqlite(dir.path().join("t.sqlite")),
    )
    .expect("reopen");
    let restored = reopened.organism().unwrap();
    assert_eq!(
        restored.total_accounted_uj(),
        accounted_before_restart,
        "restart must preserve accounted energy exactly"
    );
    assert_eq!(
        restored.ambient_internal_energy_uj(),
        ambient_before_restart
    );
    assert_eq!(reopened.sleep_phase(), SleepPhase::Awake);
}

/// Neko with different morphotype parameters produces different thermal
/// response under identical stimulus — data-driven difference, no branches.
#[test]
fn phase1_neko_vs_human_morphotype_data_driven_difference() {
    let mut h = OrganismState::physiological_baseline(&Morphotype::human());
    let mut n = OrganismState::physiological_baseline(&Morphotype::neko());

    assert_eq!(h.core_temperature_mk(), n.core_temperature_mk());

    h.apply_ambient_exchange().unwrap();
    n.apply_ambient_exchange().unwrap();

    assert_ne!(
        h.core_internal_energy_uj(),
        n.core_internal_energy_uj(),
        "different morphotypes must produce different thermal outcomes"
    );
}

/// NeuralPopulation integrates into the same accounting: spikes cost nJ.
#[test]
fn phase1_neural_activity_costs_exact_energy() {
    let mut brain = NeuralPopulation::new(86_000_000_000);
    brain.record_spikes(1_000, 10).expect("spikes");
    assert_eq!(brain.total_spike_count(), 1_000);
    assert_eq!(brain.cumulative_spike_energy_nj(), 10_000);
}
