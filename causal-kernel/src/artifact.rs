use serde_json::Value;
use sha2::{Digest, Sha256};
use thiserror::Error;

const CONTRACT_SCHEMA: &str = "makise.mechanism-contract.v1";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProgramAbi {
    ThermalExchangeV1,
    Unknown,
}

impl ProgramAbi {
    fn from_program_bytes(program: &[u8]) -> Self {
        match serde_json::from_slice::<Value>(program) {
            Ok(Value::Object(object))
                if object.get("abi").and_then(Value::as_str) == Some("thermal-exchange-v1") =>
            {
                Self::ThermalExchangeV1
            }
            _ => Self::Unknown,
        }
    }

    fn canonical_name(self) -> &'static str {
        match self {
            Self::ThermalExchangeV1 => "thermal-exchange-v1",
            Self::Unknown => "unknown",
        }
    }
}

impl std::fmt::Display for ProgramAbi {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.canonical_name())
    }
}

#[derive(Debug, Error)]
pub enum ContractParseError {
    #[error("contract is not valid JSON: {0}")]
    MalformedJson(#[from] serde_json::Error),
    #[error("contract schema_version must be {CONTRACT_SCHEMA}")]
    WrongSchema,
    #[error("contract field `{0}` is required and must be a non-empty string")]
    MissingString(&'static str),
    #[error("contract field `content_digest` must be `sha256:` followed by 64 hex characters")]
    MalformedContentDigest,
    #[error("contract requires at least one conservation rule")]
    MissingConservationRules,
    #[error("contract requires a failure policy")]
    MissingFailurePolicy,
    #[error("contract requires at least one validation scenario")]
    MissingValidationScenario,
}

#[derive(Clone, Debug)]
pub struct MechanismContract {
    json_text: String,
    mechanism_id: String,
    declared_content_digest: [u8; 32],
}

impl MechanismContract {
    pub fn from_json(json_bytes: &[u8]) -> Result<Self, ContractParseError> {
        let value: Value = serde_json::from_slice(json_bytes)?;
        let object = value.as_object().ok_or(ContractParseError::WrongSchema)?;

        if object.get("schema_version").and_then(Value::as_str) != Some(CONTRACT_SCHEMA) {
            return Err(ContractParseError::WrongSchema);
        }
        let mechanism_id = require_string(object, "mechanism_id")?;
        let content_digest = require_string(object, "content_digest")?;

        if !content_digest.starts_with("sha256:") || content_digest.len() != 7 + 64 {
            return Err(ContractParseError::MalformedContentDigest);
        }
        let mut digest = [0u8; 32];
        for index in 0..32 {
            let byte_hex = &content_digest[7 + index * 2..7 + (index + 1) * 2];
            digest[index] = u8::from_str_radix(byte_hex, 16)
                .map_err(|_| ContractParseError::MalformedContentDigest)?;
        }

        let has_conservation = object
            .get("conservation_rules")
            .and_then(Value::as_array)
            .is_some_and(|rules| !rules.is_empty());
        if !has_conservation {
            return Err(ContractParseError::MissingConservationRules);
        }
        if !object.contains_key("failure_policy") {
            return Err(ContractParseError::MissingFailurePolicy);
        }
        let has_validation = object
            .get("validation_scenarios")
            .and_then(Value::as_array)
            .is_some_and(|scenarios| !scenarios.is_empty());
        if !has_validation {
            return Err(ContractParseError::MissingValidationScenario);
        }

        Ok(Self {
            json_text: String::from_utf8_lossy(json_bytes).into_owned(),
            mechanism_id,
            declared_content_digest: digest,
        })
    }

    pub fn mechanism_id(&self) -> &str {
        &self.mechanism_id
    }

    fn declared_program_digest(&self) -> [u8; 32] {
        self.declared_content_digest
    }
}

fn require_string(
    object: &serde_json::Map<String, Value>,
    field: &'static str,
) -> Result<String, ContractParseError> {
    match object.get(field).and_then(Value::as_str) {
        Some(value) if !value.is_empty() => Ok(value.to_owned()),
        _ => Err(ContractParseError::MissingString(field)),
    }
}

#[derive(Clone, Copy, Debug, Error, Eq, PartialEq)]
pub enum AdmissionError {
    #[error("program bytes do not match contract content_digest")]
    ProgramDigestMismatch,
    #[error("program ABI is not supported by this kernel")]
    UnsupportedProgramAbi,
}

pub struct ArtifactBundle {
    contract: MechanismContract,
    program: Vec<u8>,
    abi: ProgramAbi,
}

pub struct AdmissionRecord {
    contract_digest: [u8; 32],
    program_digest: [u8; 32],
    mechanism_id: String,
    abi: ProgramAbi,
}

impl AdmissionRecord {
    pub fn mechanism_id(&self) -> &str {
        &self.mechanism_id
    }

    pub fn program_abi(&self) -> &ProgramAbi {
        &self.abi
    }

    pub fn contract_digest(&self) -> &[u8; 32] {
        &self.contract_digest
    }

    pub fn program_digest(&self) -> &[u8; 32] {
        &self.program_digest
    }
}

impl ArtifactBundle {
    pub fn new(contract: MechanismContract, program: Vec<u8>, abi: ProgramAbi) -> Self {
        Self {
            contract,
            program,
            abi,
        }
    }

    pub fn admit(&self) -> Result<AdmissionRecord, AdmissionError> {
        let actual_digest: [u8; 32] = Sha256::digest(&self.program).into();
        if actual_digest != self.contract.declared_program_digest() {
            return Err(AdmissionError::ProgramDigestMismatch);
        }
        if self.abi == ProgramAbi::Unknown
            || ProgramAbi::from_program_bytes(&self.program) != self.abi
        {
            return Err(AdmissionError::UnsupportedProgramAbi);
        }
        let contract_digest: [u8; 32] = Sha256::digest(self.contract.json_text.as_bytes()).into();
        Ok(AdmissionRecord {
            contract_digest,
            program_digest: actual_digest,
            mechanism_id: self.contract.mechanism_id.clone(),
            abi: self.abi,
        })
    }

    pub fn mutate_last_program_byte(&mut self) {
        if let Some(last) = self.program.last_mut() {
            *last = last.wrapping_add(1);
        }
    }
}
