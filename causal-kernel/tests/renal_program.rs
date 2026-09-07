use makise_causal_kernel::{ArtifactBundle, MechanismContract, ProgramAbi, RenalState};
use sha2::{Digest, Sha256};

fn bundle(water_rate: i64) -> ArtifactBundle {
    let program = serde_json::json!({
        "abi": "renal-fluid-v1",
        "water_rate_mm3_per_s": water_rate,
        "sodium_rate_umol_per_s": 1,
        "baseline_body_water_mm3": 42_000_000,
        "baseline_sodium_umol": 420_000
    })
    .to_string()
    .into_bytes();
    let mut contract: serde_json::Value = serde_json::from_slice(include_bytes!(
        "../../contracts/fixtures/mechanisms/renal-corrective-excretion-v1.json"
    ))
    .unwrap();
    contract["content_digest"] = format!("sha256:{:x}", Sha256::digest(&program)).into();
    contract["parameters"][0]["value"] = water_rate.into();
    ArtifactBundle::new(
        MechanismContract::from_json(&serde_json::to_vec(&contract).unwrap()).unwrap(),
        program,
        ProgramAbi::RenalFluidV1,
    )
}

#[test]
fn archived_rates_drive_conservative_renal_proposals() {
    let state = RenalState::new(42_250_000, 3_250_000, 455_000, 0, 0).unwrap();
    let proposed = bundle(17).propose_renal_second(&state).unwrap();
    assert_eq!(
        proposed,
        RenalState::new(42_249_983, 3_249_983, 454_999, 17, 1).unwrap()
    );
    let slower = bundle(14).propose_renal_second(&state).unwrap();
    assert_eq!(
        slower,
        RenalState::new(42_249_986, 3_249_986, 454_999, 14, 1).unwrap()
    );
    assert_eq!(state.total_body_water_mm3(), 42_250_000);
}

#[test]
fn renal_program_cannot_be_admitted_under_a_thermal_contract() {
    let program = bundle(17).program_bytes().to_vec();
    let mut contract: serde_json::Value = serde_json::from_slice(include_bytes!(
        "../../contracts/fixtures/mechanisms/two-reservoir-thermal-exchange.json"
    ))
    .unwrap();
    contract["content_digest"] = format!("sha256:{:x}", Sha256::digest(&program)).into();
    let artifact = ArtifactBundle::new(
        MechanismContract::from_json(&serde_json::to_vec(&contract).unwrap()).unwrap(),
        program,
        ProgramAbi::RenalFluidV1,
    );
    assert!(artifact.admit().is_err());
}

#[test]
fn invalid_programs_and_overflow_do_not_produce_a_candidate() {
    for rate in [0, 13, 21, i64::MAX] {
        assert!(bundle(rate).admit().is_err());
    }
    let artifact = bundle(17);
    let mut program: serde_json::Value = serde_json::from_slice(artifact.program_bytes()).unwrap();
    program["unknown"] = true.into();
    let bytes = program.to_string().into_bytes();
    let mut contract: serde_json::Value = serde_json::from_slice(include_bytes!(
        "../../contracts/fixtures/mechanisms/renal-corrective-excretion-v1.json"
    ))
    .unwrap();
    contract["content_digest"] = format!("sha256:{:x}", Sha256::digest(&bytes)).into();
    assert!(
        ArtifactBundle::new(
            MechanismContract::from_json(&contract.to_string().into_bytes()).unwrap(),
            bytes,
            ProgramAbi::RenalFluidV1
        )
        .admit()
        .is_err()
    );
    let state = RenalState::new(42_250_000, 3_250_000, 455_000, i64::MAX, 0).unwrap();
    assert!(artifact.propose_renal_second(&state).is_err());
    assert_eq!(state.urine_water_mm3(), i64::MAX);
    assert_eq!(
        artifact
            .propose_renal_second(&RenalState::baseline())
            .unwrap(),
        RenalState::baseline()
    );
}

#[test]
fn shipped_program_is_executable_and_contract_bounds_cannot_drift() {
    let program = include_bytes!(
        "../../contracts/fixtures/mechanisms/renal-corrective-excretion-v1.program.json"
    )
    .to_vec();
    let bytes =
        include_bytes!("../../contracts/fixtures/mechanisms/renal-corrective-excretion-v1.json");
    let valid = ArtifactBundle::new(
        MechanismContract::from_json(bytes).unwrap(),
        program.clone(),
        ProgramAbi::RenalFluidV1,
    );
    assert!(valid.admit().is_ok());
    let mut contract: serde_json::Value = serde_json::from_slice(bytes).unwrap();
    contract["authoritative_state_variables"][0]["validity_range"]["max"] = 90_000_000.into();
    let invalid = ArtifactBundle::new(
        MechanismContract::from_json(&contract.to_string().into_bytes()).unwrap(),
        program,
        ProgramAbi::RenalFluidV1,
    );
    assert!(invalid.admit().is_err());
}
