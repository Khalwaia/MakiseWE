//! Phase 2 slice 7 evidence: room atmosphere compartment and convective
//! heat coupling through the existing thermal port. Independent anchors
//! are hand-derived textbook values, never the production code:
//!
//! - air mass m = ρ·V with measured dry-air density ρ = 1_204_000 mg/m³
//!   (1.204 kg/m³ at 20 °C, 1 atm, CRC Handbook): a 40 m³ room holds
//!   48_160_000 mg = 48.16 kg of air;
//! - heat capacity C = m·c_v with measured c_v = 718 J/(kg·K):
//!   48.16 kg · 718 J/(kg·K) = 34_578.88 J/K, so warming the room by
//!   exactly one kelvin costs Q = m·c_v·ΔT = 34_578.88 J;
//! - convective conductance G = h·A with a declared free-convection
//!   coefficient h = 10 W/(m²·K) over a 20 cm square pot bottom
//!   (0.04 m²): G = 0.4 W/K;
//! - Newton cooling through the shared thermal port: a pot at 100 °C
//!   above a 20 °C room transfers Q = G·ΔT·t = 0.4 · 80 · 1 = 32 J in
//!   one second, conserved bit-exact across both sides.

use makise_causal_kernel::{
    AtmosphereError, DRY_AIR_DENSITY_MG_PER_M3, LiquidContainer, Morphotype, OrganismState,
    PourRequest, ReservoirPair, ReservoirState, RoomAtmosphere, ThermalProposal,
    convective_conductance_uj_per_mk_s, heater_energy_uj,
};

const ROOM_VOLUME_MM3: i64 = 40_000_000_000; // 40 m³
const REFERENCE_TEMPERATURE_MK: i64 = 293_150; // 20 °C

/// A 40 m³ room holds the hand-multiplied 48.16 kg of dry air.
#[test]
fn room_air_mass_is_the_hand_derived_product() {
    let room =
        RoomAtmosphere::new(ROOM_VOLUME_MM3, REFERENCE_TEMPERATURE_MK).expect("representable room");
    assert_eq!(room.air_mass_mg(), 48_160_000);
    assert_eq!(room.air_mass_mg(), DRY_AIR_DENSITY_MG_PER_M3 * 40);
}

/// One kelvin of this room's air costs m·c_v·ΔT = 48.16 kg · 718
/// J/(kg·K) · 1 K = 34_578.88 J by hand. Injecting exactly that energy
/// raises the projected temperature by exactly 1000 mK, and removing
/// it restores the baseline bit-exact.
#[test]
fn one_kelvin_of_room_air_costs_the_textbook_energy() {
    let mut room =
        RoomAtmosphere::new(ROOM_VOLUME_MM3, REFERENCE_TEMPERATURE_MK).expect("representable room");
    let baseline_temperature = room.temperature_mk();

    room.apply_heating(34_578_880_000)
        .expect("one-kelvin injection stays inside the envelope");
    assert_eq!(room.temperature_mk(), baseline_temperature + 1000);

    room.apply_heating(-34_578_880_000)
        .expect("symmetric extraction");
    assert_eq!(room.temperature_mk(), baseline_temperature);
}

/// A declared burner delivers power × time as an exact microjoule
/// amount: 2 kW for 60 s is 120 kJ.
#[test]
fn burner_power_converts_exactly_to_microjoules() {
    assert_eq!(
        heater_energy_uj(2_000, 60).expect("representable"),
        120_000_000_000
    );
    assert!(matches!(
        heater_energy_uj(0, 60),
        Err(AtmosphereError::InvalidParameters)
    ));
}

/// G = h·A: ten watts per square metre-kelvin over a 20 cm square pot
/// bottom is exactly 0.4 W/K, i.e. 400 µJ/(mK·s).
#[test]
fn convective_conductance_matches_h_times_area() {
    let conductance =
        convective_conductance_uj_per_mk_s(10, 40_000).expect("representable surface");
    assert_eq!(conductance, 400);

    // An area whose conductance leaves whole units leaves the envelope.
    assert!(matches!(
        convective_conductance_uj_per_mk_s(10, 40_001),
        Err(AtmosphereError::NonRepresentableConductance)
    ));
}

/// A 100 °C pot over the 20 °C room loses exactly G·ΔT = 32 J in one
/// second through the existing thermal port; the room gains the same
/// amount; nothing else moves. The pot cools by ~36 mK while the room
/// warms by less than a millikelvin — the realistic asymmetry of heat
/// capacities.
#[test]
fn hot_pot_delivers_thirty_two_joules_through_the_existing_port() {
    let room =
        RoomAtmosphere::new(ROOM_VOLUME_MM3, REFERENCE_TEMPERATURE_MK).expect("representable room");

    // 900 g aluminium-like pot: C = 900 J/K = 900_000 µJ/mK at 100 °C.
    let pot = ReservoirState::new(900_000 * 373_150, 900_000);
    let pair = ReservoirPair::new(pot, *room.air_reservoir());
    let proposal = ThermalProposal::one_second(&pair, 400).expect("declared conductance");

    let transfer = proposal.transfer();
    assert_eq!(transfer.delta_hot_uj(), -32_000_000);
    assert_eq!(transfer.delta_cold_uj(), 32_000_000);
    assert_eq!(
        transfer.delta_hot_uj() + transfer.delta_cold_uj(),
        0,
        "the port conserves energy bit-exact"
    );

    // Applying the transfer: pot projection drops below 1/25 kelvin
    // while the room stays within one millikelvin of baseline.
    let next_pot = ReservoirState::new(
        pot.internal_energy_microjoule() + transfer.delta_hot_uj(),
        pot.heat_capacity_microjoule_per_millikelvin(),
    );
    assert_eq!(next_pot.internal_energy_microjoule() / 900_000, 373_114);
    assert_eq!(room.temperature_mk(), REFERENCE_TEMPERATURE_MK);
}

/// The organism ambient surrogate and the room atmosphere are both
/// plain reservoirs, so they couple through the same port without any
/// new mechanism code: a room two kelvin warmer than the surrogate
/// hands over exactly G·ΔT = 50_000 · 2000 = 10⁸ µJ = 100 J per second.
#[test]
fn organism_ambient_reservoir_couples_through_the_same_port() {
    let room = RoomAtmosphere::new(ROOM_VOLUME_MM3, REFERENCE_TEMPERATURE_MK + 2_000)
        .expect("representable room");
    let organism = OrganismState::physiological_baseline(&Morphotype::human());

    let pair = ReservoirPair::new(*room.air_reservoir(), *organism.ambient_reservoir());
    let proposal = ThermalProposal::one_second(&pair, 50_000).expect("declared conductance");
    let transfer = proposal.transfer();

    assert_eq!(transfer.delta_hot_uj(), -100_000_000);
    assert_eq!(transfer.delta_cold_uj(), 100_000_000);
}

/// Fifty millilitres of pool water evaporate into exactly 49.91 g of
/// vapour (ρ_water = 998.2 g/l), raising absolute humidity to
/// 49.91 g / 40 m³ by hand. The liquid↔vapour bridge conserves total
/// accounted water bit-exact when the liquid leaves a container.
#[test]
fn fifty_millilitres_evaporate_into_exact_vapour_mass() {
    let mut room =
        RoomAtmosphere::new(ROOM_VOLUME_MM3, REFERENCE_TEMPERATURE_MK).expect("representable room");

    let spilled = LiquidContainer::new(50_000, 50_000).expect("filled cup");
    let full_target = LiquidContainer::new(50_000, 50_000).expect("full target");
    let outcome = spilled
        .pour_into(
            &full_target,
            &PourRequest::new(50_000).expect("positive request"),
        )
        .expect("valid pour");
    assert_eq!(outcome.spilled_mm3(), 50_000);

    let vapour_gained = room
        .evaporate_in(outcome.spilled_mm3())
        .expect("representable evaporation");
    assert_eq!(vapour_gained, 49_910);
    assert_eq!(room.vapour_mass_mg(), 49_910);
    assert_eq!(room.total_gas_mass_mg(), 48_160_000 + 49_910);

    // Absolute humidity: 49_910 mg over 40 m³ = 1_247_750 µg/m³.
    assert_eq!(
        room.absolute_humidity_ug_per_m3().expect("representable"),
        1_247_750
    );

    // The same water mass re-condensing would restore the empty floor:
    // volume equivalent of 49_910 mg is exactly 50_000 mm³.
    assert_eq!(vapour_gained * 10_000 / 9_982, outcome.spilled_mm3());
}

/// Repeated evaluation of the pure coupling is identical: no hidden
/// state, no clock dependence.
#[test]
fn repeated_evaluation_is_bit_identical() {
    let first = convective_conductance_uj_per_mk_s(10, 40_000).expect("representable");
    let second = convective_conductance_uj_per_mk_s(10, 40_000).expect("representable");
    assert_eq!(first, second);
}

/// Volumes that do not yield whole milligrams of air or a representable
/// heat capacity are typed-rejected instead of silently rounded, as are
/// out-of-envelope temperatures.
#[test]
fn degenerate_or_fractional_rooms_are_typed() {
    assert!(matches!(
        RoomAtmosphere::new(123_456, REFERENCE_TEMPERATURE_MK),
        Err(AtmosphereError::NonRepresentableAirMass)
    ));
    // 250_000 mm³ gives 301 mg of air, whose capacity 301·718/1000 is
    // fractional.
    assert!(matches!(
        RoomAtmosphere::new(250_000, REFERENCE_TEMPERATURE_MK),
        Err(AtmosphereError::NonRepresentableHeatCapacity)
    ));
    assert!(matches!(
        RoomAtmosphere::new(ROOM_VOLUME_MM3, 350_000), // 76.85 °C > 70 °C
        Err(AtmosphereError::OutsideValidityRange)
    ));
    assert!(matches!(
        RoomAtmosphere::new(ROOM_VOLUME_MM3, -1),
        Err(AtmosphereError::OutsideValidityRange)
    ));
    assert!(matches!(
        RoomAtmosphere::new(0, REFERENCE_TEMPERATURE_MK),
        Err(AtmosphereError::InvalidParameters)
    ));
}

/// Heating that would drive the room outside the declared −10..70 °C
/// apartment envelope is rejected atomically: state is untouched.
#[test]
fn heating_beyond_the_envelope_is_rejected_without_partial_application() {
    let mut room =
        RoomAtmosphere::new(ROOM_VOLUME_MM3, REFERENCE_TEMPERATURE_MK).expect("representable room");
    let before = room.air_reservoir().internal_energy_microjoule();

    assert!(matches!(
        room.apply_heating(i64::MAX),
        Err(AtmosphereError::Overflow)
    ));

    let scorching = heater_energy_uj(3_500, 3600).expect("representable"); // far past 70 °C
    assert!(matches!(
        room.apply_heating(scorching),
        Err(AtmosphereError::OutsideValidityRange)
    ));
    assert_eq!(room.air_reservoir().internal_energy_microjoule(), before);
    assert_eq!(room.temperature_mk(), REFERENCE_TEMPERATURE_MK);
}

/// Evaporation amounts that do not divide into whole milligrams of
/// water leave the envelope, and humidity that does not land on whole
/// µg/m³ is typed rather than rounded.
#[test]
fn fractional_evaporation_and_humidity_are_typed() {
    let mut room =
        RoomAtmosphere::new(ROOM_VOLUME_MM3, REFERENCE_TEMPERATURE_MK).expect("representable room");
    assert!(matches!(
        room.evaporate_in(1),
        Err(AtmosphereError::NonRepresentableVapourGain)
    ));
    assert_eq!(room.vapour_mass_mg(), 0);

    // A closet-sized representable room (0.75 m³) makes the absolute
    // humidity projection fractional even for representable vapour.
    let mut closet =
        RoomAtmosphere::new(750_000_000, REFERENCE_TEMPERATURE_MK).expect("representable");
    closet.evaporate_in(5_000).expect("representable gain");
    assert!(matches!(
        closet.absolute_humidity_ug_per_m3(),
        Err(AtmosphereError::NonRepresentableHumidity)
    ));
}
