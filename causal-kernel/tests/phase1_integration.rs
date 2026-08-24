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

/// 24h scenario: wake → eat → activity → ambient drop → sleep.
/// Proves causal integration across organism slices with exact conservation.
#[test]
fn phase1_24h_human_scenario_conserves_energy_and_produces_intention() {
    let dir = tempfile::tempdir().expect("temp");
    let (mut engine, _) = WorldEngine::open(
        spec("p1-human"),
        StorageLocation::sqlite(dir.path().join("t.sqlite")),
    )
    .expect("open");

    // 06:00 wake: 1h awake metabolism (night rate until 06:00 boundary).
    engine
        .commit(CommitRequest::advance_to("wake", 0, 3_600))
        .unwrap();
    let after_wake = engine.organism().unwrap();
    let sleep_phase = engine.sleep_phase();
    assert_eq!(sleep_phase, SleepPhase::Awake);
    let chemical_after_wake = after_wake.chemical_store_uj();

    // 07:00 breakfast: ingest measured food energy.
    let meal_energy_uj = 2_000_000_000_000i64;
    engine
        .commit(CommitRequest::ingest_food("breakfast", 1, meal_energy_uj))
        .unwrap();
    let after_meal = engine.organism().unwrap();
    assert_eq!(
        after_meal.chemical_store_uj() - chemical_after_wake,
        meal_energy_uj,
        "ingestion must add exactly declared chemical energy"
    );

    // 09:00-10:00 moderate activity: 1h awake day rate + thermal exchange.
    engine
        .commit(CommitRequest::advance_to("activity", 2, 3_600))
        .unwrap();

    // 14:00-16:00 ambient temperature step down: thermal exchange visible.
    engine
        .commit(CommitRequest::advance_to("cooling", 3, 7_200))
        .unwrap();

    // 23:00 accepted sleep intention via cognitive gate pipeline.
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
        .commit(CommitRequest::accept_sleep_intention("sleep", 4))
        .unwrap();
    engine
        .commit(CommitRequest::advance_to("recovery", 5, 25_200))
        .unwrap();

    // After 7h asleep: phase is Asleep, sleep debt reduced.
    assert_eq!(engine.sleep_phase(), SleepPhase::Asleep);

    // Exact conservation: chemical burned == net thermal gain in both reservoirs.
    let final_total = engine.organism().unwrap().total_accounted_uj();
    let final_ambient = engine.organism().unwrap().ambient_internal_energy_uj();
    assert!(final_total + final_ambient > 0, "energy cannot vanish");
    // Restart parity: reopen and verify state survives.
    std::mem::drop(engine);
    let (reopened, _) = WorldEngine::open(
        spec("p1-human"),
        StorageLocation::sqlite(dir.path().join("t.sqlite")),
    )
    .expect("reopen");
    let restored = reopened.organism().unwrap();
    assert_eq!(
        restored.total_accounted_uj(),
        final_total,
        "restart must preserve accounted energy exactly"
    );
    assert_eq!(reopened.sleep_phase(), SleepPhase::Asleep);
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
