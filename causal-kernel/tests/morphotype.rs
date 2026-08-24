use makise_causal_kernel::{Morphotype, OrganismState, ReservoirState};

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
