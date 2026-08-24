use std::collections::BTreeMap;

use serde_json::Value;
use thiserror::Error;

/// Declared morphotype parameters. Data, not behavior: each value has units,
/// provenance `expert_estimate`, and can be replaced via mechanism artifacts
/// without changing code paths.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Morphotype {
    awake_metabolism_uj_per_second: i64,
    asleep_metabolism_uj_per_second: i64,
    night_awake_metabolism_uj_per_second: i64,
    core_heat_capacity_uj_per_mk: i64,
    ambient_conductance_uj_per_mk_s: i64,
    anatomy_nodes: Vec<AnatomyNode>,
    organ_bindings: Vec<OrganBinding>,
}

impl Morphotype {
    pub const fn new(
        awake_metabolism_uj_per_second: i64,
        asleep_metabolism_uj_per_second: i64,
        night_awake_metabolism_uj_per_second: i64,
        core_heat_capacity_uj_per_mk: i64,
        ambient_conductance_uj_per_mk_s: i64,
    ) -> Self {
        Self {
            awake_metabolism_uj_per_second,
            asleep_metabolism_uj_per_second,
            night_awake_metabolism_uj_per_second,
            core_heat_capacity_uj_per_mk,
            ambient_conductance_uj_per_mk_s,
            anatomy_nodes: Vec::new(),
            organ_bindings: Vec::new(),
        }
    }

    fn with_package(
        mut self,
        anatomy_nodes: Vec<AnatomyNode>,
        organ_bindings: Vec<OrganBinding>,
    ) -> Self {
        self.anatomy_nodes = anatomy_nodes;
        self.organ_bindings = organ_bindings;
        self
    }

    /// Human baseline parameters. Provenance: `expert_estimate` anchored
    /// to published values summarized in docs/research/biology-realism.md:
    /// tissue specific heat 3490 J/(kg·K) × 62 kg reference mass →
    /// 216_380_000 µJ/mK core heat capacity; whole-body passive
    /// conductance ≈ 5.6 W/K inside the published 4–10 W/K band for
    /// radiation + convection; metabolic rates from circadian.rs. The
    /// conductance is tuned so the passive equilibrium at a 20 °C room
    /// lands at ≈310.1 K.
    pub fn human() -> Self {
        Self::new(95_000_000, 75_000_000, 88_000_000, 216_380_000, 5_600)
    }

    /// Neko: fictional morphotype (`fictional_assumption` / `species_proxy`).
    /// Assumed ~30 kg body mass with fur-insulated surface. No empirical
    /// population exists; these magnitudes are declared placeholders whose
    /// only contract-tested properties are orderings relative to human.
    pub fn neko() -> Self {
        Self::new(55_000_000, 45_000_000, 50_000_000, 104_700_000, 3_200)
    }

    pub fn awake_metabolism_uj_per_second(&self) -> i64 {
        self.awake_metabolism_uj_per_second
    }

    pub fn asleep_metabolism_uj_per_second(&self) -> i64 {
        self.asleep_metabolism_uj_per_second
    }

    pub fn night_awake_metabolism_uj_per_second(&self) -> i64 {
        self.night_awake_metabolism_uj_per_second
    }

    pub fn core_heat_capacity_uj_per_mk(&self) -> i64 {
        self.core_heat_capacity_uj_per_mk
    }

    pub fn ambient_conductance_uj_per_mk_s(&self) -> i64 {
        self.ambient_conductance_uj_per_mk_s
    }

    pub fn anatomy_nodes(&self) -> &[AnatomyNode] {
        &self.anatomy_nodes
    }

    pub fn organ_bindings(&self) -> &[OrganBinding] {
        &self.organ_bindings
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AnatomyNode {
    pub node_id: String,
    pub kind: String,
    pub count: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AnatomyEdge {
    pub from: String,
    pub to: String,
    pub relation: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OrganBinding {
    pub anatomy_node_id: String,
    pub mechanism_id: String,
    pub mechanism_digest: String,
    pub resolution_id: String,
}

#[derive(Debug, Error, Eq, PartialEq)]
pub enum MorphotypeError {
    #[error("morphotype JSON is not a valid definition")]
    InvalidJson,
    #[error("unsupported morphotype schema version")]
    UnsupportedSchemaVersion,
    #[error("morphotype must be an independent root definition")]
    NotRootDefinition,
    #[error("anatomy binding references unknown anatomy node: {0}")]
    UnknownAnatomyNode(String),
    #[error("no declared runtime parameters registered for morphotype: {0}")]
    UnknownMorphotypeParameters(String),
}

/// Runtime view of a validated MorphotypeDefinition fixture. This slice binds
/// declared graph and mechanism data; it does not synthesize behavior.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MorphotypeDefinition {
    morphotype_id: String,
    anatomy_nodes: Vec<AnatomyNode>,
    anatomy_edges: Vec<AnatomyEdge>,
    organ_bindings: Vec<OrganBinding>,
    runtime_parameters: Morphotype,
}

impl MorphotypeDefinition {
    pub fn from_fixture(json: &str) -> Result<Self, MorphotypeError> {
        let value: Value = serde_json::from_str(json).map_err(|_| MorphotypeError::InvalidJson)?;
        if value.get("schema_version").and_then(Value::as_str)
            != Some("makise.morphotype-definition.v1")
        {
            return Err(MorphotypeError::UnsupportedSchemaVersion);
        }
        if value.get("root_definition").and_then(Value::as_bool) != Some(true) {
            return Err(MorphotypeError::NotRootDefinition);
        }

        let object = value.as_object().ok_or(MorphotypeError::InvalidJson)?;
        let morphotype_id = required_id(object.get("morphotype_id"))?;
        let graph = object
            .get("anatomy_graph")
            .and_then(Value::as_object)
            .ok_or(MorphotypeError::InvalidJson)?;
        let anatomy_nodes = graph
            .get("nodes")
            .and_then(Value::as_array)
            .ok_or(MorphotypeError::InvalidJson)?
            .iter()
            .map(|node| {
                Ok(AnatomyNode {
                    node_id: required_id(node.get("node_id"))?,
                    kind: required_id(node.get("kind"))?,
                    count: node
                        .get("count")
                        .and_then(Value::as_i64)
                        .filter(|count| *count >= 1)
                        .ok_or(MorphotypeError::InvalidJson)?,
                })
            })
            .collect::<Result<Vec<_>, MorphotypeError>>()?;
        let anatomy_edges = graph
            .get("edges")
            .and_then(Value::as_array)
            .ok_or(MorphotypeError::InvalidJson)?
            .iter()
            .map(|edge| {
                Ok(AnatomyEdge {
                    from: required_id(edge.get("from"))?,
                    to: required_id(edge.get("to"))?,
                    relation: required_id(edge.get("relation"))?,
                })
            })
            .collect::<Result<Vec<_>, MorphotypeError>>()?;
        let organ_bindings = object
            .get("organ_bindings")
            .and_then(Value::as_array)
            .ok_or(MorphotypeError::InvalidJson)?
            .iter()
            .map(|binding| {
                Ok(OrganBinding {
                    anatomy_node_id: required_id(binding.get("anatomy_node_id"))?,
                    mechanism_id: required_id(binding.get("mechanism_id"))?,
                    mechanism_digest: binding
                        .get("mechanism_digest")
                        .and_then(Value::as_str)
                        .filter(|digest| digest.starts_with("sha256:") && digest.len() == 71)
                        .ok_or(MorphotypeError::InvalidJson)?
                        .to_owned(),
                    resolution_id: required_id(binding.get("resolution_id"))?,
                })
            })
            .collect::<Result<Vec<_>, MorphotypeError>>()?;

        let known_nodes = anatomy_nodes
            .iter()
            .map(|node| (node.node_id.clone(), ()))
            .collect::<BTreeMap<String, ()>>();
        for binding in &organ_bindings {
            if !known_nodes.contains_key(&binding.anatomy_node_id) {
                return Err(MorphotypeError::UnknownAnatomyNode(
                    binding.anatomy_node_id.clone(),
                ));
            }
        }

        let runtime_parameters = default_runtime_parameters_for(&morphotype_id)?
            .with_package(anatomy_nodes.clone(), organ_bindings.clone());
        Ok(Self {
            morphotype_id,
            anatomy_nodes,
            anatomy_edges,
            organ_bindings,
            runtime_parameters,
        })
    }

    pub fn morphotype_id(&self) -> &str {
        &self.morphotype_id
    }

    pub fn anatomy_nodes(&self) -> &[AnatomyNode] {
        &self.anatomy_nodes
    }

    pub fn anatomy_edges(&self) -> &[AnatomyEdge] {
        &self.anatomy_edges
    }

    pub fn organ_bindings(&self) -> &[OrganBinding] {
        &self.organ_bindings
    }

    pub fn binding_for_anatomy_node(&self, node_id: &str) -> Option<&OrganBinding> {
        self.organ_bindings
            .iter()
            .find(|binding| binding.anatomy_node_id == node_id)
    }

    pub fn runtime_parameters(&self) -> &Morphotype {
        &self.runtime_parameters
    }
}

fn required_id(value: Option<&Value>) -> Result<String, MorphotypeError> {
    value
        .and_then(Value::as_str)
        .map(str::to_owned)
        .filter(|value| {
            let bytes = value.as_bytes();
            !bytes.is_empty()
                && bytes[0].is_ascii_lowercase()
                && bytes[1..]
                    .iter()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
        })
        .ok_or(MorphotypeError::InvalidJson)
}

/// Declared runtime parameters are keyed by morphotype identity. An
/// unregistered id must fail admission instead of silently binding human
/// baseline values.
fn default_runtime_parameters_for(morphotype_id: &str) -> Result<Morphotype, MorphotypeError> {
    match morphotype_id {
        "human-v1" => Ok(Morphotype::human()),
        "neko-v1" => Ok(Morphotype::neko()),
        other => Err(MorphotypeError::UnknownMorphotypeParameters(
            other.to_owned(),
        )),
    }
}
