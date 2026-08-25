//! Phase 2 slice 9 evidence: apartment electricity and water
//! infrastructure as unit-typed flow-conservation mechanisms.
//! Independent anchors are hand-derived textbook values, never the
//! production code:
//!
//! - a typical EU apartment branch breaker is 230 V × 16 A = 3680 W;
//!   admitting a 2200 W kettle leaves exactly 1480 W of headroom and
//!   a second kettle is typed-rejected, never clamped;
//! - one second at P watts delivers exactly P joules: 2200 W for 60 s
//!   is 132 kJ = 132_000_000_000 µJ;
//! - a kitchen tap at the declared 150_000 mm³/s (9 l/min) fills
//!   1.5 litres in ten seconds; the metered outflow equals container
//!   gain plus first-class spill bit-exact;
//! - disconnecting the load stops delivery without any promised
//!   outcome: no energy exists to apply anywhere.

use makise_causal_kernel::{
    InfrastructureError, LiquidContainer, PourRequest, PowerNetwork, ReservoirState, WaterNetwork,
    heater_energy_uj,
};

const BRANCH_CAPACITY_W: i64 = 3_680; // 230 V × 16 A
const KETTLE_W: i64 = 2_200;

/// Admission is exact integer accounting: the kettle leaves 1480 W,
/// the second kettle exceeds headroom and is typed-rejected without
/// partial admission, dropping the load restores every watt.
#[test]
fn branch_breaker_admits_and_restores_exactly() {
    let mut network = PowerNetwork::new(BRANCH_CAPACITY_W).expect("positive capacity");
    assert_eq!(network.available_watts(), BRANCH_CAPACITY_W);

    network.admit_load(KETTLE_W).expect("kettle fits");
    assert_eq!(network.available_watts(), 1_480);

    assert!(matches!(
        network.admit_load(KETTLE_W),
        Err(InfrastructureError::CapacityExceeded)
    ));
    assert_eq!(
        network.available_watts(),
        1_480,
        "a rejected draw never partially loads the network"
    );

    network.drop_load(KETTLE_W).expect("admitted load");
    assert_eq!(network.available_watts(), BRANCH_CAPACITY_W);
}

/// Delivery is the textbook identity E = P·t in microjoules, metered
/// cumulatively, and only for admitted load.
#[test]
fn one_second_at_p_watts_delivers_exactly_p_joules() {
    let mut network = PowerNetwork::new(BRANCH_CAPACITY_W).expect("positive capacity");
    network.admit_load(KETTLE_W).expect("kettle fits");

    assert_eq!(
        network.deliver_one_second(KETTLE_W).expect("admitted"),
        2_200_000_000,
        "hand: 2200 J = 2200 W · 1 s"
    );
    let minute = std::iter::repeat_n((), 60)
        .try_fold(0i64, |total, ()| {
            network
                .deliver_one_second(KETTLE_W)
                .map(|energy| total + energy)
        })
        .expect("admitted every second");
    assert_eq!(minute, 132_000_000_000);
    assert_eq!(
        network.cumulative_delivered_uj(),
        132_000_000_000 + 2_200_000_000
    );

    // The delivered amount matches the shared burner conversion.
    assert_eq!(
        heater_energy_uj(KETTLE_W, 60).expect("representable"),
        minute
    );
}

/// The gate negative test: after disconnection there is nothing to
/// deliver — a typed rejection with zero energy, so no downstream
/// reservoir can move and no completion can be promised.
#[test]
fn disconnecting_power_stops_heating_without_promised_outcome() {
    let mut network = PowerNetwork::new(BRANCH_CAPACITY_W).expect("positive capacity");
    network.admit_load(KETTLE_W).expect("kettle fits");

    // A pot on the burner, held outside the network.
    let mut pot = ReservoirState::new(900_000 * 293_150, 900_000);

    network.deliver_one_second(KETTLE_W).expect("admitted");
    pot = ReservoirState::new(
        pot.internal_energy_microjoule() + 2_200_000_000,
        pot.heat_capacity_microjoule_per_millikelvin(),
    );
    let heated = pot.internal_energy_microjoule();

    network.drop_load(KETTLE_W).expect("admitted load");
    assert!(matches!(
        network.deliver_one_second(KETTLE_W),
        Err(InfrastructureError::LoadNotAdmitted)
    ));
    // Nothing was delivered, so applying "nothing" leaves the pot
    // bit-identical — the episode sees a blocker, not a completion.
    assert_eq!(pot.internal_energy_microjoule(), heated);
}

/// Drawing more than the admitted load bypasses admission and is
/// rejected; degenerate parameters are parameter errors.
#[test]
fn unadmitted_or_degenerate_draws_are_typed() {
    let mut network = PowerNetwork::new(BRANCH_CAPACITY_W).expect("positive capacity");
    assert!(matches!(
        PowerNetwork::new(0),
        Err(InfrastructureError::InvalidParameters)
    ));
    assert!(matches!(
        network.admit_load(0),
        Err(InfrastructureError::InvalidParameters)
    ));
    network.admit_load(1_000).expect("small load");
    assert!(matches!(
        network.deliver_one_second(1_001),
        Err(InfrastructureError::LoadNotAdmitted)
    ));
    assert!(matches!(
        network.drop_load(2_000),
        Err(InfrastructureError::InvalidParameters)
    ));
}

/// The tap delivers rate × time as an exact volume; pouring via a
/// jug conserves the chain bit-exact: metered outflow == container
/// content + first-class spill.
#[test]
fn tap_fills_the_pot_with_bit_exact_conservation() {
    let mut mains = WaterNetwork::new(150_000).expect("positive capacity"); // 9 l/min
    mains.admit_flow(150_000).expect("at capacity");

    // First fill: exactly 1.5 litres in ten seconds, poured fully in.
    let jug = LiquidContainer::new(3_000_000, 0).expect("3 litre jug");
    let mut pot = LiquidContainer::new(3_000_000, 0).expect("3 litre pot");

    let first = mains.draw_mm3(150_000, 10).expect("admitted flow");
    assert_eq!(first, 1_500_000, "hand: 150_000 mm³/s × 10 s = 1.5 l");
    let jug = LiquidContainer::new(jug.capacity_mm3(), first).expect("refilled jug");
    let outcome = jug
        .pour_into(&pot, &PourRequest::new(first).expect("positive"))
        .expect("valid pour");
    assert_eq!(outcome.spilled_mm3(), 0);
    pot = *outcome.next_target();

    // Second draw overfills: twenty seconds at full flow, poured into
    // the remaining free space — the overflow is a first-class spill.
    let second = mains.draw_mm3(150_000, 20).expect("admitted flow");
    assert_eq!(second, 3_000_000);
    let jug = LiquidContainer::new(jug.capacity_mm3(), second).expect("refilled jug");
    let outcome = jug
        .pour_into(&pot, &PourRequest::new(second).expect("positive"))
        .expect("valid pour");
    assert_eq!(outcome.spilled_mm3(), 1_500_000);
    pot = *outcome.next_target();

    // Bit-exact chain: every metered millilitre is accounted for.
    assert_eq!(
        mains.cumulative_delivered_mm3(),
        pot.content_mm3() + 1_500_000,
        "hand: 4_500_000 out of the main == 3_000_000 in the pot + 1_500_000 spilled"
    );
}

/// Requesting more flow than the main admits, drawing through a
/// closed tap, and dropping unopened flow are typed rejections.
#[test]
fn excess_flow_and_closed_taps_are_typed() {
    let mut mains = WaterNetwork::new(150_000).expect("positive capacity");
    assert!(matches!(
        mains.draw_mm3(50_000, 5),
        Err(InfrastructureError::LoadNotAdmitted)
    ));
    assert!(matches!(
        mains.admit_flow(150_001),
        Err(InfrastructureError::CapacityExceeded)
    ));
    mains.admit_flow(100_000).expect("fits");
    assert_eq!(mains.draw_mm3(100_000, 5).expect("admitted flow"), 500_000);
    mains.close_flow(100_000).expect("open flow");
    assert!(matches!(
        mains.draw_mm3(100_000, 5),
        Err(InfrastructureError::LoadNotAdmitted)
    ));
    assert!(matches!(
        mains.close_flow(1),
        Err(InfrastructureError::InvalidParameters)
    ));
}
