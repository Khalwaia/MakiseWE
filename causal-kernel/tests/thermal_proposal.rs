use makise_causal_kernel::{ReservoirPair, ReservoirState, ThermalError, ThermalProposal};

const CONDUCTANCE_UJ_PER_MK_S: i64 = 1_000;

fn reservoirs(hot_uj: i64, cold_uj: i64) -> ReservoirPair {
    ReservoirPair::new(
        ReservoirState::new(hot_uj, 4_000),
        ReservoirState::new(cold_uj, 6_000),
    )
}

#[test]
fn proposal_does_not_mutate_source_state() {
    let pair = reservoirs(20_000_000_000_000, 10_000_000_000_000);
    let before = StateSnapshot::of(&pair);

    let proposal = ThermalProposal::one_second(&pair, CONDUCTANCE_UJ_PER_MK_S)
        .expect("in-envelope state must propose");

    assert_eq!(StateSnapshot::of(&pair), before);
    assert!(proposal.transfer().delta_hot_uj() < 0);
}

#[test]
fn transfer_is_equal_and_opposite_exactly() {
    let pair = reservoirs(20_000_000_000_000, 10_000_000_000_000);

    let proposal =
        ThermalProposal::one_second(&pair, CONDUCTANCE_UJ_PER_MK_S).expect("in envelope");

    let delta_hot = proposal.transfer().delta_hot_uj();
    let delta_cold = proposal.transfer().delta_cold_uj();
    assert_eq!(delta_hot, -delta_cold);
}

#[test]
fn hotter_to_colder_matches_hand_computed_delta() {
    // Hand calculation (independent of implementation, integer arithmetic):
    // T_hot = E/C = 20e12/4000  = 5_000_000_000 mK
    // T_cold = E/C = 10e12/6000 = 1_666_666_666 mK (floor)
    // ΔT = 3_333_333_334 mK
    // Q = G·ΔT·t = 1000 · ΔT · 1 s = 3_333_333_334_000 uJ
    let pair = reservoirs(20_000_000_000_000, 10_000_000_000_000);

    let proposal =
        ThermalProposal::one_second(&pair, CONDUCTANCE_UJ_PER_MK_S).expect("in envelope");

    assert_eq!(proposal.transfer().delta_hot_uj(), -3_333_333_334_000);
    assert_eq!(proposal.transfer().delta_cold_uj(), 3_333_333_334_000);
}

#[test]
fn equilibrium_transfers_zero_energy() {
    // T = 20e12/5000 = 4.0e9 mK for both reservoirs.
    let pair = ReservoirPair::new(
        ReservoirState::new(20_000_000_000_000, 5_000),
        ReservoirState::new(20_000_000_000_000, 5_000),
    );

    let proposal =
        ThermalProposal::one_second(&pair, CONDUCTANCE_UJ_PER_MK_S).expect("equilibrium");

    assert_eq!(proposal.transfer().delta_hot_uj(), 0);
    assert_eq!(proposal.transfer().delta_cold_uj(), 0);
}

#[test]
fn zero_heat_capacity_is_outside_validity_range() {
    let pair = ReservoirPair::new(
        ReservoirState::new(20_000_000_000_000, 0),
        ReservoirState::new(10_000_000_000_000, 6_000),
    );

    let error = ThermalProposal::one_second(&pair, CONDUCTANCE_UJ_PER_MK_S)
        .err()
        .expect("zero capacity must be rejected");

    assert!(matches!(error, ThermalError::OutsideValidityRange));
}

#[test]
fn overflow_in_transfer_is_typed_failure_not_clamp() {
    let pair = ReservoirPair::new(
        ReservoirState::new(i64::MAX, 1),
        ReservoirState::new(i64::MIN + 1, 6_000),
    );

    let error = ThermalProposal::one_second(&pair, CONDUCTANCE_UJ_PER_MK_S)
        .err()
        .expect("overflow must be typed failure");

    assert!(matches!(error, ThermalError::Overflow));
}

#[derive(Debug, Eq, PartialEq)]
struct StateSnapshot([u8; 32]);

impl StateSnapshot {
    fn of(pair: &ReservoirPair) -> Self {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(b"hot");
        hasher.update(pair.hot().internal_energy_microjoule().to_be_bytes());
        hasher.update(
            pair.hot()
                .heat_capacity_microjoule_per_millikelvin()
                .to_be_bytes(),
        );
        hasher.update(b"cold");
        hasher.update(pair.cold().internal_energy_microjoule().to_be_bytes());
        hasher.update(
            pair.cold()
                .heat_capacity_microjoule_per_millikelvin()
                .to_be_bytes(),
        );
        Self(hasher.finalize().into())
    }
}
