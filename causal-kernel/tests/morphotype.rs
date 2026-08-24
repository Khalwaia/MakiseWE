use makise_causal_kernel::{Morphotype, MorphotypeDefinition, OrganismState, ReservoirState};

#[test]
fn human_morphotype_matches_current_baseline() {
    let morph = Morphotype::human();

    assert_eq!(morph.awake_metabolism_uj_per_second(), 1_200_000);
    assert_eq!(morph.asleep_metabolism_uj_per_second(), 800_000);
    assert_eq!(morph.night_awake_metabolism_uj_per_second(), 1_000_000);
    assert_eq!(morph.core_heat_capacity_uj_per_mk(), 4_000);
    assert_eq!(morph.ambient_conductance_uj_per_mk_s(), 50,);
}

#[test]
fn neko_morphotype_differs_from_human_in_declared_parameters() {
    let human = Morphotype::human();
    let neko = Morphotype::neko();

    // Neko: smaller body mass -> lower heat capacity; fur insulation ->
    // lower ambient conductance; slightly different metabolism.
    assert_ne!(
        neko.core_heat_capacity_uj_per_mk(),
        human.core_heat_capacity_uj_per_mk()
    );
    assert_ne!(
        neko.ambient_conductance_uj_per_mk_s(),
        human.ambient_conductance_uj_per_mk_s()
    );
    assert_ne!(
        neko.awake_metabolism_uj_per_second(),
        human.awake_metabolism_uj_per_second()
    );

    // Physical sanity: smaller body loses less heat through insulation.
    assert!(neko.core_heat_capacity_uj_per_mk() < human.core_heat_capacity_uj_per_mk());
    assert!(neko.ambient_conductance_uj_per_mk_s() < human.ambient_conductance_uj_per_mk_s());
}

#[test]
fn neko_fixture_binds_neko_runtime_parameters_not_human() {
    let neko_json = include_str!("../../contracts/fixtures/morphotypes/neko-minimal.json");
    let definition = MorphotypeDefinition::from_fixture(neko_json).expect("neko package");
    let bound = definition.runtime_parameters();
    let human = Morphotype::human();
    let neko = Morphotype::neko();

    assert_eq!(
        bound.awake_metabolism_uj_per_second(),
        neko.awake_metabolism_uj_per_second()
    );
    assert_eq!(
        bound.asleep_metabolism_uj_per_second(),
        neko.asleep_metabolism_uj_per_second()
    );
    assert_eq!(
        bound.night_awake_metabolism_uj_per_second(),
        neko.night_awake_metabolism_uj_per_second()
    );
    assert_eq!(
        bound.core_heat_capacity_uj_per_mk(),
        neko.core_heat_capacity_uj_per_mk()
    );
    assert_eq!(
        bound.ambient_conductance_uj_per_mk_s(),
        neko.ambient_conductance_uj_per_mk_s()
    );
    assert_ne!(
        bound.core_heat_capacity_uj_per_mk(),
        human.core_heat_capacity_uj_per_mk(),
        "neko package must not silently bind human runtime parameters"
    );
}

#[test]
fn unknown_morphotype_id_is_rejected_without_silent_default() {
    let json = r#"{
        "schema_version": "makise.morphotype-definition.v1",
        "root_definition": true,
        "morphotype_id": "unknown-x1",
        "anatomy_graph": {
            "nodes": [{ "node_id": "body", "kind": "mammalian-body", "count": 1 }],
            "edges": []
        },
        "organ_bindings": []
    }"#;

    let error = MorphotypeDefinition::from_fixture(json)
        .expect_err("unregistered morphotype id must be rejected, never defaulted to human");
    assert!(matches!(
        error,
        makise_causal_kernel::MorphotypeError::UnknownMorphotypeParameters(_)
    ));
}

#[test]
fn organism_state_uses_morphotype_parameters_for_exchange() {
    let human = OrganismState::with_morphotype(
        &Morphotype::human(),
        8_400_000_000_000,
        160_000_000,
        ReservoirState::new(20_000_000_000_000, 1_000_000),
    );
    let neko = OrganismState::with_morphotype(
        &Morphotype::neko(),
        8_400_000_000_000,
        160_000_000,
        ReservoirState::new(20_000_000_000_000, 1_000_000),
    );

    let mut h = human;
    let mut n = neko;
    h.apply_ambient_exchange().unwrap();
    n.apply_ambient_exchange().unwrap();

    let h_delta = (h.core_internal_energy_uj() - 160_000_000).abs();
    let n_delta = (n.core_internal_energy_uj() - 160_000_000).abs();
    // Lower conductance means less heat transferred in the same second.
    assert!(
        n_delta < h_delta,
        "Neko with lower conductance should transfer less heat per second"
    );
}
