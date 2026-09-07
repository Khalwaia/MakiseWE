use makise_causal_kernel::{
    AdmissionError, ArtifactBundle, ContractParseError, MechanismContract, ProgramAbi,
};

const PROGRAM_ABI: ProgramAbi = ProgramAbi::ThermalExchangeV1;

fn contract_json() -> String {
    let mut value: serde_json::Value = serde_json::from_slice(include_bytes!(
        "../../contracts/fixtures/mechanisms/two-reservoir-thermal-exchange.json"
    ))
    .expect("reference contract");
    value["content_digest"] = "PLACEHOLDER".into();
    serde_json::to_string_pretty(&value).expect("contract JSON")
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
    let mut value: serde_json::Value = serde_json::from_str(
        &contract_json().replace("PLACEHOLDER", &program_digest_hex(&program)),
    )
    .unwrap();
    value.as_object_mut().unwrap().remove("conservation_rules");
    let contract_json = value.to_string();
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
