use makise_causal_kernel::{CognitiveDisposition, CognitiveGate, CortexProposal, Intention};

fn sample_proposal(id: &str) -> CortexProposal {
    CortexProposal::new(
        id,
        "consciousness-makise",
        "frame-001",
        "drink water from available cup",
    )
}

#[test]
fn accepted_intention_creates_durable_state_without_physical_delta() {
    let gate = CognitiveGate::new();
    let proposal = sample_proposal("accept-drink");
    let disposition = gate.evaluate(&proposal);

    assert_eq!(
        disposition,
        CognitiveDisposition::Accepted {
            reasons: vec!["feasible".into()]
        }
    );

    // Gate creates intention as separate cognitive state, NOT physical mutation.
    let intention: Intention = gate
        .adopt_intention(&proposal)
        .expect("accepted proposal must produce intention");
    assert_eq!(intention.proposal_id(), "accept-drink");
    assert_eq!(intention.description(), "drink water from available cup");
    assert!(!intention.contains_physical_delta());
}

#[test]
fn rejected_proposal_produces_no_intention() {
    let mut gate = CognitiveGate::new();
    gate.reject_reason("infeasible");
    let proposal = sample_proposal("rejected-climb");

    let disposition = gate.evaluate(&proposal);
    assert!(matches!(disposition, CognitiveDisposition::Rejected { .. }));

    let result = gate.adopt_intention(&proposal);
    assert!(
        result.is_err(),
        "rejected proposal must not create intention"
    );
}

#[test]
fn deferred_proposal_records_reconsideration_trigger() {
    let mut gate = CognitiveGate::new();
    gate.defer_with_trigger("fatigue outside validity range");
    let proposal = sample_proposal("deferred-balance");

    let disposition = gate.evaluate(&proposal);
    match &disposition {
        CognitiveDisposition::Deferred { reconsideration } => {
            assert_eq!(reconsideration, "fatigue outside validity range");
        }
        _ => panic!("expected Deferred, got {disposition:?}"),
    }

    assert!(gate.adopt_intention(&proposal).is_err());
}
