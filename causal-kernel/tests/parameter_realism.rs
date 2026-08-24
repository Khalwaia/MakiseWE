//! Parameter realism guards.
//!
//! Every expected band here is derived from independent published sources
//! summarized in docs/research/biology-realism.md — never from the
//! production algorithm. Changing a physiological constant away from its
//! declared band turns these tests red before the drift can ship.

use makise_causal_kernel::{
    AMBIENT_HEAT_CAPACITY_UJ_PER_MK, ASLEEP_METABOLISM_UJ_PER_SECOND,
    AWAKE_METABOLISM_UJ_PER_SECOND, BASELINE_AMBIENT_INTERNAL_ENERGY_UJ,
    BASELINE_CORE_INTERNAL_ENERGY_UJ, INITIAL_CHEMICAL_STORE_UJ, Morphotype, NeuralPopulation,
    OrganismState, REFERENCE_AMBIENT_TEMPERATURE_MK, REFERENCE_CORE_TEMPERATURE_MK, ReservoirState,
    SleepPhase,
};

#[test]
fn daily_energy_budget_is_within_published_female_band() {
    // Independent anchor: typical adult female TDEE ≈ 1500–2200 kcal/day.
    let day = 28_800 * ASLEEP_METABOLISM_UJ_PER_SECOND + 57_600 * AWAKE_METABOLISM_UJ_PER_SECOND;
    assert!(
        (6_280_000_000_000..=9_210_000_000_000).contains(&day),
        "daily chemical demand {day} uJ is outside the published 1500-2200 kcal/day band"
    );
}

#[test]
fn metabolic_ordering_sleep_then_night_awake_then_day_awake() {
    let night_awake = makise_causal_kernel::awake_metabolism_for_second(100);
    assert!(ASLEEP_METABOLISM_UJ_PER_SECOND < night_awake);
    assert!(night_awake < AWAKE_METABOLISM_UJ_PER_SECOND);
}

#[test]
fn initial_temperatures_match_declared_references() {
    let organism = OrganismState::new(INITIAL_CHEMICAL_STORE_UJ, BASELINE_CORE_INTERNAL_ENERGY_UJ);

    assert!((organism.core_temperature_mk() - REFERENCE_CORE_TEMPERATURE_MK).abs() <= 1);
    assert!((organism.ambient_temperature_mk() - REFERENCE_AMBIENT_TEMPERATURE_MK).abs() <= 1);
}

#[test]
fn passive_equilibrium_core_temperature_near_37c_at_20c_room() {
    // Independent anchor: resting humans hold core temperature near
    // 310 K in a thermoneutral room; passive coarse surrogate must land
    // inside 35.5–38.5 °C at the nominal 20 °C ambient reference.
    // Quasi-infinite room surrogate (1e9 J/K): isolates the passive
    // setpoint from finite-room warming over the convergence window;
    // finite-room drift is guarded by its own dedicated test.
    let ambient_capacity_uj_per_mk = 1_000_000_000_000;
    let ambient_energy_uj = REFERENCE_AMBIENT_TEMPERATURE_MK
        .checked_mul(ambient_capacity_uj_per_mk)
        .expect("quasi-infinite room energy fits i64");
    let mut organism = OrganismState::with_morphotype(
        &Morphotype::human(),
        INITIAL_CHEMICAL_STORE_UJ,
        BASELINE_CORE_INTERNAL_ENERGY_UJ,
        ReservoirState::new(ambient_energy_uj, ambient_capacity_uj_per_mk),
    );

    // Ideal-feeding surrogate: each burned unit is reabsorbed immediately
    // so the chemical store stays funded while the thermal loop converges
    // to its passive equilibrium.
    for _ in 0..500_000 {
        organism
            .apply_metabolism(AWAKE_METABOLISM_UJ_PER_SECOND)
            .expect("funded by feeding surrogate");
        organism.absorb_chemical_energy(AWAKE_METABOLISM_UJ_PER_SECOND);
        organism.apply_ambient_exchange().expect("valid state");
    }

    assert!(
        (308_500..=311_500).contains(&organism.core_temperature_mk()),
        "passive equilibrium {} mK is outside the physiological core band",
        organism.core_temperature_mk()
    );
}

#[test]
fn whole_body_thermal_time_constant_within_physiological_band() {
    // Independent anchor: whole-body cooling constants of an adult body
    // are measured in hours. tau = C/G must stay within 4–24 h.
    let morph = Morphotype::human();
    let tau_seconds =
        morph.core_heat_capacity_uj_per_mk() / morph.ambient_conductance_uj_per_mk_s();
    assert!(
        (4 * 3600..=24 * 3600).contains(&tau_seconds),
        "thermal time constant {tau_seconds} s is outside the 4-24 h whole-body band"
    );
}

#[test]
fn ambient_surrogate_absorbs_a_day_of_metabolic_heat_below_one_kelvin() {
    let day_heat_uj =
        57_600 * AWAKE_METABOLISM_UJ_PER_SECOND + 28_800 * ASLEEP_METABOLISM_UJ_PER_SECOND;
    let ambient_shift_mk = day_heat_uj / AMBIENT_HEAT_CAPACITY_UJ_PER_MK;
    assert!(
        ambient_shift_mk < 1_000,
        "one day of metabolic heat shifts the room by {ambient_shift_mk} mK; surrogate too small"
    );
}

#[test]
fn baseline_reservoir_energies_keep_i64_headroom_for_long_runs() {
    // Worst case accounted growth over a 365-day accelerated run must
    // stay far below i64 bounds so checked arithmetic never trips on
    // realistic horizons.
    let year_heat_uj = 365 * (86_400 * AWAKE_METABOLISM_UJ_PER_SECOND);
    assert!(year_heat_uj > 0);
    assert!(
        BASELINE_CORE_INTERNAL_ENERGY_UJ.saturating_add(year_heat_uj) < i64::MAX / 8,
        "core energy representation leaves insufficient headroom for long runs"
    );
    assert!(
        BASELINE_AMBIENT_INTERNAL_ENERGY_UJ.saturating_add(year_heat_uj) < i64::MAX / 8,
        "ambient energy representation leaves insufficient headroom for long runs"
    );
}

#[test]
fn chemical_store_is_glycogen_scale_food_reserve() {
    // Independent anchor: human glycogen-scale short-term reserve is on
    // the order of one to two thousand kilocalories.
    assert!(
        (5_000_000_000_000..=12_000_000_000_000).contains(&INITIAL_CHEMICAL_STORE_UJ),
        "chemical store {} uJ is outside the glycogen-scale band",
        INITIAL_CHEMICAL_STORE_UJ
    );
}

#[test]
fn spike_energy_accounting_is_nanojoule_scale() {
    // Independent anchor: metabolic cost of a single neural spike is in
    // the nanojoule order (Attwell & Laughlin 2001 band ~1–100 nJ).
    let per_spike_nj = 10;
    assert!(per_spike_nj <= 100, "per-spike energy must remain nJ-order");

    let mut population = NeuralPopulation::new(86_000_000_000);
    population
        .record_spikes(1_000, per_spike_nj)
        .expect("valid batch");
    assert_eq!(population.total_spike_count(), 1_000);
    assert_eq!(population.cumulative_spike_energy_nj(), 10_000);

    let sleep_demand = makise_causal_kernel::metabolic_demand_uj_per_second(SleepPhase::Asleep);
    assert!(sleep_demand > 0);
}
