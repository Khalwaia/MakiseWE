use makise_causal_kernel::{ArtifactBundle, MechanismContract, ProgramAbi};

const CONTRACT: &[u8] =
    include_bytes!("../../contracts/fixtures/mechanisms/two-reservoir-thermal-exchange.json");
const PROGRAM: &[u8] = include_bytes!(
    "../../contracts/fixtures/mechanisms/two-reservoir-thermal-exchange.program.json"
);

#[test]
fn incomplete_dependency_contract_cannot_be_admitted() {
    let mut contract: serde_json::Value = serde_json::from_slice(CONTRACT).unwrap();
    contract
        .as_object_mut()
        .unwrap()
        .remove("authoritative_state_variables");
    let parsed = MechanismContract::from_json(&serde_json::to_vec(&contract).unwrap());
    if let Ok(contract) = parsed {
        assert!(
            ArtifactBundle::new(contract, PROGRAM.to_vec(), ProgramAbi::ThermalExchangeV1)
                .admit()
                .is_err(),
            "missing unitful state contract must prevent admission"
        );
    }
}

#[test]
fn unknown_executable_field_cannot_be_silently_ignored() {
    use sha2::{Digest, Sha256};
    let mut program: serde_json::Value = serde_json::from_slice(PROGRAM).unwrap();
    program["ignored_operation"] = "mutate_state".into();
    let bytes = serde_json::to_vec(&program).unwrap();
    let mut contract: serde_json::Value = serde_json::from_slice(CONTRACT).unwrap();
    contract["content_digest"] = format!("sha256:{:x}", Sha256::digest(&bytes)).into();
    let contract = MechanismContract::from_json(&serde_json::to_vec(&contract).unwrap()).unwrap();
    assert!(
        ArtifactBundle::new(contract, bytes, ProgramAbi::ThermalExchangeV1)
            .admit()
            .is_err()
    );
}

#[test]
fn every_required_contract_field_is_checked_at_admission() {
    let schema: serde_json::Value = serde_json::from_slice(include_bytes!(
        "../../contracts/schemas/mechanism-contract-v1.schema.json"
    ))
    .unwrap();
    for field in schema["required"].as_array().unwrap() {
        let mut contract: serde_json::Value = serde_json::from_slice(CONTRACT).unwrap();
        contract
            .as_object_mut()
            .unwrap()
            .remove(field.as_str().unwrap());
        if let Ok(contract) = MechanismContract::from_json(&serde_json::to_vec(&contract).unwrap())
        {
            assert!(
                ArtifactBundle::new(contract, PROGRAM.to_vec(), ProgramAbi::ThermalExchangeV1)
                    .admit()
                    .is_err(),
                "missing {field}"
            );
        }
    }
}

#[test]
fn nested_contract_errors_prevent_admission() {
    let original: serde_json::Value = serde_json::from_slice(CONTRACT).unwrap();
    let mut invalid = Vec::new();
    let mut contract = original.clone();
    contract["parameters"][0]["unit"] = "".into();
    invalid.push(contract);
    let mut contract = original.clone();
    contract["authoritative_state_variables"][0]["dimension_kind"] = "health_score".into();
    invalid.push(contract);
    let mut contract = original.clone();
    contract["failure_policy"]["missing_artifact"] = "ignore".into();
    invalid.push(contract);
    let mut contract = original;
    contract["unrecognized"] = true.into();
    invalid.push(contract);
    for contract in invalid {
        let contract =
            MechanismContract::from_json(&serde_json::to_vec(&contract).unwrap()).unwrap();
        assert!(
            ArtifactBundle::new(contract, PROGRAM.to_vec(), ProgramAbi::ThermalExchangeV1)
                .admit()
                .is_err()
        );
    }
}

#[test]
fn rejected_dependency_leaves_no_event_state_or_receipt() {
    use makise_causal_kernel::{
        CommitRequest, EventCursor, EventQuery, OpenSpec, ReservoirPair, ReservoirState,
        StorageLocation, TimelineId, WorldEngine, WorldId,
    };
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("timeline.sqlite");
    let spec = OpenSpec::new(
        WorldId::new("world").unwrap(),
        TimelineId::new("timeline").unwrap(),
    );
    let (mut engine, _) = WorldEngine::open(spec.clone(), StorageLocation::sqlite(&path)).unwrap();
    let mut contract: serde_json::Value = serde_json::from_slice(CONTRACT).unwrap();
    contract
        .as_object_mut()
        .unwrap()
        .remove("uncertainty_model");
    let invalid = MechanismContract::from_json(&serde_json::to_vec(&contract).unwrap()).unwrap();
    let pair = ReservoirPair::new(
        ReservoirState::new(20_000_000, 10),
        ReservoirState::new(10_000_000, 10),
    );
    assert!(
        engine
            .commit(CommitRequest::thermal_exchange(
                "same-id",
                0,
                pair.clone(),
                ArtifactBundle::new(invalid, PROGRAM.to_vec(), ProgramAbi::ThermalExchangeV1)
            ))
            .is_err()
    );
    assert!(
        engine
            .events(EventQuery::new(EventCursor::start(), 10).unwrap())
            .unwrap()
            .events()
            .is_empty()
    );
    assert!(engine.organism().is_none());
    drop(engine);
    let (mut engine, _) = WorldEngine::open(spec, StorageLocation::sqlite(&path)).unwrap();
    let valid = MechanismContract::from_json(CONTRACT).unwrap();
    let receipt = engine
        .commit(CommitRequest::thermal_exchange(
            "same-id",
            0,
            pair,
            ArtifactBundle::new(valid, PROGRAM.to_vec(), ProgramAbi::ThermalExchangeV1),
        ))
        .unwrap();
    assert!(!receipt.replayed_request());
    assert_eq!(receipt.timeline_version(), 1);
    assert_eq!(
        engine.fast_replay().unwrap(),
        engine.audit_replay().unwrap()
    );
}

#[test]
fn malformed_unicode_digest_is_a_typed_rejection_not_a_panic() {
    let mut contract: serde_json::Value = serde_json::from_slice(CONTRACT).unwrap();
    // Correct byte length, but the first two-byte hex slice bisects UTF-8.
    contract["content_digest"] = format!("sha256:€{}", "0".repeat(61)).into();
    assert!(MechanismContract::from_json(&serde_json::to_vec(&contract).unwrap()).is_err());
}
