use serde_json::Value;
use sha2::{Digest, Sha256};
use thiserror::Error;

const CONTRACT_SCHEMA: &str = "makise.mechanism-contract.v1";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProgramAbi {
    ThermalExchangeV1,
    RenalFluidV1,
    Unknown,
}

impl ProgramAbi {
    pub(crate) fn from_program_bytes(program: &[u8]) -> Self {
        match serde_json::from_slice::<Value>(program) {
            Ok(Value::Object(object))
                if object.get("abi").and_then(Value::as_str) == Some("thermal-exchange-v1")
                    && object.len() == 2
                    && object
                        .get("conductance_uj_per_mk_s")
                        .and_then(Value::as_i64)
                        .is_some_and(|value| value > 0) =>
            {
                Self::ThermalExchangeV1
            }
            Ok(value) if renal_parameters(&value).is_some() => Self::RenalFluidV1,
            _ => Self::Unknown,
        }
    }

    fn canonical_name(self) -> &'static str {
        match self {
            Self::ThermalExchangeV1 => "thermal-exchange-v1",
            Self::RenalFluidV1 => "renal-fluid-v1",
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

        if !content_digest.starts_with("sha256:")
            || content_digest.len() != 7 + 64
            || !content_digest.is_ascii()
        {
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
    #[error("mechanism contract does not conform to the complete versioned schema")]
    InvalidContract,
    #[error("program bytes do not match contract content_digest")]
    ProgramDigestMismatch,
    #[error("program ABI is not supported by this kernel")]
    UnsupportedProgramAbi,
    #[error("renal proposal violates its amount bounds or overflows")]
    InvalidRenalProposal,
}

#[derive(Clone, Debug)]
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
        static SCHEMA: std::sync::OnceLock<jsonschema::Validator> = std::sync::OnceLock::new();
        let schema = SCHEMA.get_or_init(|| {
            let schema: Value = serde_json::from_str(include_str!(
                "../../contracts/schemas/mechanism-contract-v1.schema.json"
            ))
            .expect("embedded mechanism schema is JSON");
            jsonschema::validator_for(&schema).expect("embedded mechanism schema is valid")
        });
        let contract: Value = serde_json::from_str(&self.contract.json_text)
            .map_err(|_| AdmissionError::InvalidContract)?;
        if !schema.is_valid(&contract) {
            return Err(AdmissionError::InvalidContract);
        }
        if self.abi == ProgramAbi::RenalFluidV1 {
            let program: Value = serde_json::from_slice(&self.program)
                .map_err(|_| AdmissionError::UnsupportedProgramAbi)?;
            let (water, sodium, _, _) =
                renal_parameters(&program).ok_or(AdmissionError::UnsupportedProgramAbi)?;
            let parameter_matches = |id: &str, unit: &str, value: i64| {
                contract["parameters"].as_array().is_some_and(|parameters| {
                    parameters.iter().any(|parameter| {
                        parameter["parameter_id"] == id
                            && parameter["unit"] == unit
                            && parameter["value"].as_i64() == Some(value)
                    })
                })
            };
            if self.mechanism_id() != "renal.fluid-electrolyte"
                || !parameter_matches("max-corrective-urine-water-mm3-per-s", "mm3_per_s", water)
                || !parameter_matches(
                    "max-corrective-urine-sodium-umol-per-s",
                    "umol_per_s",
                    sodium,
                )
            {
                return Err(AdmissionError::InvalidContract);
            }
            // This ABI supports the complete bounded reference contract only.
            // Parameter alternatives retain all other declared semantics.
            let mut reference: Value = serde_json::from_slice(include_bytes!(
                "../../contracts/fixtures/mechanisms/renal-corrective-excretion-v1.json"
            ))
            .expect("embedded renal contract is JSON");
            reference["content_digest"] = contract["content_digest"].clone();
            reference["parameters"][0]["value"] = water.into();
            reference["parameters"][1]["value"] = sodium.into();
            if contract != reference {
                return Err(AdmissionError::InvalidContract);
            }
        }
        let contract_digest: [u8; 32] = Sha256::digest(self.contract.json_text.as_bytes()).into();
        Ok(AdmissionRecord {
            contract_digest,
            program_digest: actual_digest,
            mechanism_id: self.contract.mechanism_id.clone(),
            abi: self.abi,
        })
    }

    pub fn program_bytes(&self) -> &[u8] {
        &self.program
    }

    /// Produces a candidate only; authoritative application remains in commit.
    pub fn propose_renal_second(
        &self,
        state: &crate::renal::RenalState,
    ) -> Result<crate::renal::RenalState, AdmissionError> {
        self.admit()?;
        let program: Value = serde_json::from_slice(&self.program)
            .map_err(|_| AdmissionError::UnsupportedProgramAbi)?;
        let (water_rate, sodium_rate, water_baseline, sodium_baseline) =
            renal_parameters(&program).ok_or(AdmissionError::UnsupportedProgramAbi)?;
        let water = (state.total_body_water_mm3() - water_baseline).clamp(0, water_rate);
        let sodium = (state.plasma_sodium_umol() - sodium_baseline).clamp(0, sodium_rate);
        crate::renal::RenalState::new(
            state.total_body_water_mm3() - water,
            state.plasma_mm3() - water,
            state.plasma_sodium_umol() - sodium,
            state
                .urine_water_mm3()
                .checked_add(water)
                .ok_or(AdmissionError::InvalidRenalProposal)?,
            state
                .urine_sodium_umol()
                .checked_add(sodium)
                .ok_or(AdmissionError::InvalidRenalProposal)?,
        )
        .map_err(|_| AdmissionError::InvalidRenalProposal)
    }

    pub fn mechanism_id(&self) -> &str {
        self.contract.mechanism_id()
    }

    pub fn thermal_exchange_conductance_uj_per_mk_s(&self) -> Option<i64> {
        let value: Value = serde_json::from_slice(&self.program).ok()?;
        value.get("conductance_uj_per_mk_s").and_then(Value::as_i64)
    }

    pub fn mutate_last_program_byte(&mut self) {
        if let Some(last) = self.program.last_mut() {
            *last = last.wrapping_add(1);
        }
    }
}

fn renal_parameters(value: &Value) -> Option<(i64, i64, i64, i64)> {
    let object = value.as_object()?;
    if object.len() != 5 || object.get("abi")?.as_str()? != "renal-fluid-v1" {
        return None;
    }
    let water = object.get("water_rate_mm3_per_s")?.as_i64()?;
    let sodium = object.get("sodium_rate_umol_per_s")?.as_i64()?;
    let water_baseline = object.get("baseline_body_water_mm3")?.as_i64()?;
    let sodium_baseline = object.get("baseline_sodium_umol")?.as_i64()?;
    // ABI v1 is the bounded resting compatibility model, not a general solver.
    if !(14..=20).contains(&water)
        || !(1..=2).contains(&sodium)
        || water_baseline != crate::renal::BASELINE_TOTAL_BODY_WATER_MM3
        || sodium_baseline != crate::renal::BASELINE_PLASMA_SODIUM_UMOL
    {
        return None;
    }
    Some((water, sodium, water_baseline, sodium_baseline))
}
