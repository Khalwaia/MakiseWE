use makise_causal_kernel::{MorphotypeDefinition, OrganismState, ReservoirState};

#[test]
fn human_and_neko_anatomy_graphs_drive_organ_bindings() {
    let human_json = include_str!("../../contracts/fixtures/morphotypes/human-minimal.json");
    let neko_json = include_str!("../../contracts/fixtures/morphotypes/neko-minimal.json");

    let human = MorphotypeDefinition::from_fixture(human_json).expect("human package");
    let neko = MorphotypeDefinition::from_fixture(neko_json).expect("neko package");

    assert_eq!(human.morphotype_id(), "human-v1");
    assert_eq!(neko.morphotype_id(), "neko-v1");
    assert_eq!(human.anatomy_nodes().len(), 7);
    assert_eq!(neko.anatomy_nodes().len(), 11);
    assert_eq!(human.organ_bindings().len(), 1);
    assert_eq!(neko.organ_bindings().len(), 4);

    let circulation = neko
        .binding_for_anatomy_node("circulation")
        .expect("circulation binding");
    assert_eq!(circulation.resolution_id, "cell-cohort-v1");
    assert_eq!(
        circulation.mechanism_digest,
        "sha256:1111111111111111111111111111111111111111111111111111111111111111"
    );
}

#[test]
fn runtime_morphotype_can_be_created_from_definition_data() {
    let human_json = include_str!("../../contracts/fixtures/morphotypes/human-minimal.json");
    let definition = MorphotypeDefinition::from_fixture(human_json).expect("human package");
    let runtime = definition.runtime_parameters();

    assert_eq!(runtime.awake_metabolism_uj_per_second(), 95_000_000);
    assert_eq!(runtime.core_heat_capacity_uj_per_mk(), 216_380_000);
}

#[test]
fn anatomy_graph_is_available_on_runtime_state() {
    let neko_json = include_str!("../../contracts/fixtures/morphotypes/neko-minimal.json");
    let definition = MorphotypeDefinition::from_fixture(neko_json).expect("neko package");
    let organism = OrganismState::with_morphotype(
        definition.runtime_parameters(),
        8_400_000_000_000,
        160_000_000,
        ReservoirState::new(20_000_000_000_000, 1_000_000),
    );

    assert!(
        organism
            .morphotype()
            .anatomy_nodes()
            .iter()
            .any(|node| node.node_id == "tail-vertebrae")
    );
}
