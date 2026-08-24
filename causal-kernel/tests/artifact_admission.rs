use makise_causal_kernel::{
    AdmissionError, ArtifactBundle, ContractParseError, MechanismContract, ProgramAbi,
};

const PROGRAM_ABI: ProgramAbi = ProgramAbi::ThermalExchangeV1;

fn contract_json() -> String {
    String::from(
        r#"{
  "schema_version": "makise.mechanism-contract.v1",
  "mechanism_id": "thermal.two-reservoir-exchange",
  "version": "0.1.0",
  "content_digest": "PLACEHOLDER",
  "causal_inputs": [
    { "port_id": "hot-energy-input", "variable": "reservoir.hot.internal-energy", "unit": "uJ", "dimension_kind": "physical_quantity" }
  ],
  "causal_outputs": [
    { "port_id": "cold-energy-output", "variable": "reservoir.cold.internal-energy", "unit": "uJ", "dimension_kind": "physical_quantity" }
  ],
  "read_set": ["reservoir.hot.internal-energy", "reservoir.cold.internal-energy"],
  "write_set": ["reservoir.hot.internal-energy", "reservoir.cold.internal-energy"],
  "conservation_rules": [
    { "quantity": "energy.total", "unit": "uJ", "tolerance": { "value": 0, "unit": "uJ" } }
  ],
  "validity_range": {
    "conditions": ["Two finite thermal reservoirs with positive heat capacity"],
    "exclusions": ["No biological realism claim"]
  },
  "failure_policy": {
    "invalid_input": "reject_transition"
  },
  "validation_scenarios": [
    { "scenario_id": "schema-conservation-example", "evidence_kind": "schema_only" }
  ]
}"#,
    )
}

fn program_bytes() -> Vec<u8> {
    br#"{"abi":"thermal-exchange-v1","conductance_uj_per_mk_s":1000}"#.to_vec()
}

fn valid_bundle() -> ArtifactBundle {
    let program = program_bytes();
    let contract_json = contract_json().replace("PLACEHOLDER", &program_digest_hex(&program));
    let contract = MechanismContract::from_json(contract_json.as_bytes()).expect("valid contract");
    ArtifactBundle::new(contract, program, PROGRAM_ABI)
}

fn program_digest_hex(program: &[u8]) -> String {
    format!("sha256:{}", hex_digest(program))
}

fn hex_digest(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[test]
fn valid_thermal_bundle_is_admitted() {
    let bundle = valid_bundle();
    let admission = bundle.admit().expect("valid bundle must be admitted");

    assert_eq!(admission.mechanism_id(), "thermal.two-reservoir-exchange");
    assert_eq!(admission.program_abi(), &PROGRAM_ABI);
}

#[test]
fn mutated_program_byte_is_rejected_by_content_digest() {
    let mut bundle = valid_bundle();
    bundle.mutate_last_program_byte();

    let error = bundle.admit().err().expect("mutation must be detected");

    assert!(matches!(error, AdmissionError::ProgramDigestMismatch));
}

#[test]
fn wrong_declared_contract_digest_is_rejected() {
    let program = program_bytes();
    let wrong = format!("sha256:{}", "0".repeat(64));
    let contract_json = contract_json().replace("PLACEHOLDER", &wrong);
    let contract =
        MechanismContract::from_json(contract_json.as_bytes()).expect("parses despite bad digest");

    let error = ArtifactBundle::new(contract, program, PROGRAM_ABI)
        .admit()
        .err()
        .expect("wrong declared digest must be rejected");

    assert!(matches!(error, AdmissionError::ProgramDigestMismatch));
}

#[test]
fn incomplete_contract_without_conservation_is_rejected() {
    let program = program_bytes();
    let contract_json = contract_json()
        .replace("PLACEHOLDER", &program_digest_hex(&program))
        .replace(
            "  \"conservation_rules\": [\n    { \"quantity\": \"energy.total\", \"unit\": \"uJ\", \"tolerance\": { \"value\": 0, \"unit\": \"uJ\" } }\n  ],\n",
            "",
        );
    let error = MechanismContract::from_json(contract_json.as_bytes())
        .expect_err("missing conservation must be rejected at parse");

    assert!(matches!(
        error,
        ContractParseError::MissingConservationRules
    ));
}

#[test]
fn unknown_program_abi_is_rejected_before_storage() {
    let program = br#"{"abi":"unknown-opcode-v9"}"#.to_vec();
    let contract_json = contract_json().replace("PLACEHOLDER", &program_digest_hex(&program));
    let contract =
        MechanismContract::from_json(contract_json.as_bytes()).expect("valid contract JSON");

    let error = ArtifactBundle::new(contract, program, ProgramAbi::Unknown)
        .admit()
        .err()
        .expect("unknown ABI must be rejected before storage");

    assert!(matches!(error, AdmissionError::UnsupportedProgramAbi));
}
