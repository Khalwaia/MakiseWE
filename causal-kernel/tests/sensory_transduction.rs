use makise_causal_kernel::{
    CommitRequest, OpenSpec, StorageLocation, TimelineId, WorldEngine, WorldId,
};

fn spec(world: &str, timeline: &str) -> OpenSpec {
    OpenSpec::new(
        WorldId::new(world).expect("valid"),
        TimelineId::new(timeline).expect("valid"),
    )
}

#[test]
fn organism_thermal_exchange_with_ambient_reservoir_conserves_total_energy() {
    let path = tempfile::tempdir().unwrap();
    let (mut engine, _) = WorldEngine::open(
        spec("sensory-world", "sensory-timeline"),
        StorageLocation::sqlite(path.path().join("t.sqlite")),
    )
    .expect("open");

    engine
        .commit(CommitRequest::advance_to("init", 0, 0))
        .unwrap();

    // First tick initializes organism; second tick performs exchange.
    let core_before = {
        let organism = engine.organism().unwrap_or_else(|| unreachable!());
        organism.core_internal_energy_uj()
    };
    let ambient_before = engine.organism().unwrap().ambient_internal_energy_uj();
    let chemical_before = engine.organism().unwrap().chemical_store_uj();

    engine
        .commit(CommitRequest::advance_to("tick-1", 1, 1))
        .unwrap();

    let core_after = engine.organism().unwrap().core_internal_energy_uj();
    let ambient_after = engine.organism().unwrap().ambient_internal_energy_uj();
    let chemical_after = engine.organism().unwrap().chemical_store_uj();

    assert!(core_after < core_before, "cooling exceeds metabolic input");
    assert!(ambient_after > ambient_before, "ambient should gain heat");

    // Metabolism converts chemical energy into the core; thermal exchange
    // moves core heat to ambient without loss or creation.
    assert_eq!(
        chemical_before - chemical_after,
        (core_after - core_before) + (ambient_after - ambient_before),
        "chemical burn exactly equals net thermal gain in both reservoirs"
    );
}

#[test]
fn ambient_state_survives_restart() {
    let dir = tempfile::tempdir().unwrap();
    let world_id = WorldId::new("restart-world").expect("valid");
    let timeline_id = TimelineId::new("restart-timeline").expect("valid");

    {
        let open_spec = OpenSpec::new(world_id.clone(), timeline_id.clone());
        let (mut engine, _) = WorldEngine::open(
            open_spec,
            StorageLocation::sqlite(dir.path().join("t.sqlite")),
        )
        .expect("open");
        engine
            .commit(CommitRequest::advance_to("init", 0, 0))
            .unwrap();
        engine
            .commit(CommitRequest::advance_to("warm-up", 1, 5))
            .unwrap();
        std::mem::drop(engine);
    }

    let reopened_spec = OpenSpec::new(world_id, timeline_id);
    let (reopened, _) = WorldEngine::open(
        reopened_spec,
        StorageLocation::sqlite(dir.path().join("t.sqlite")),
    )
    .unwrap();
    let after = reopened.organism().expect("organism after restart");
    assert!(after.ambient_internal_energy_uj() > 0);
}
