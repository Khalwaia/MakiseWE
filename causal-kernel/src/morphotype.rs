use std::collections::BTreeMap;

use serde_json::Value;
use thiserror::Error;

use crate::articulation::JointSpec;

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
    blood_volume_mm3: i64,
    hb_tetramer_umol: i64,
    mean_arterial_pressure_mpa: i64,
    lung_diffusion_umol_per_s: i64,
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
            blood_volume_mm3: 5_000_000,
            hb_tetramer_umol: 11_500,
            mean_arterial_pressure_mpa: 12_400_000,
            lung_diffusion_umol_per_s: 300,
            anatomy_nodes: Vec::new(),
            organ_bindings: Vec::new(),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub const fn with_blood(
        awake_metabolism_uj_per_second: i64,
        asleep_metabolism_uj_per_second: i64,
        night_awake_metabolism_uj_per_second: i64,
        core_heat_capacity_uj_per_mk: i64,
        ambient_conductance_uj_per_mk_s: i64,
        blood_volume_mm3: i64,
        hb_tetramer_umol: i64,
        mean_arterial_pressure_mpa: i64,
        lung_diffusion_umol_per_s: i64,
    ) -> Self {
        Self {
            awake_metabolism_uj_per_second,
            asleep_metabolism_uj_per_second,
            night_awake_metabolism_uj_per_second,
            core_heat_capacity_uj_per_mk,
            ambient_conductance_uj_per_mk_s,
            blood_volume_mm3,
            hb_tetramer_umol,
            mean_arterial_pressure_mpa,
            lung_diffusion_umol_per_s,
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
    /// Blood: 5 L, Hb 150 g/L → 11 500 µmol tetramer, MAP 93 mmHg, diffusion 300 µmol/s.
    pub fn human() -> Self {
        Self::with_blood(
            95_000_000,
            75_000_000,
            88_000_000,
            216_380_000,
            5_600,
            5_000_000,
            11_500,
            12_400_000,
            300,
        )
    }

    /// Neko: fictional morphotype (`fictional_assumption` / `species_proxy`).
    /// Assumed ~30 kg body mass with fur-insulated surface. No empirical
    /// population exists; these magnitudes are declared placeholders whose
    /// only contract-tested properties are orderings relative to human.
    /// Blood volume ~3 L, lower Hb and diffusion.
    pub fn neko() -> Self {
        Self::with_blood(
            55_000_000,
            45_000_000,
            50_000_000,
            104_700_000,
            3_200,
            3_000_000,
            6_900,
            11_200_000,
            180,
        )
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

    pub fn blood_volume_mm3(&self) -> i64 {
        self.blood_volume_mm3
    }

    pub fn hb_tetramer_umol(&self) -> i64 {
        self.hb_tetramer_umol
    }

    pub fn mean_arterial_pressure_mpa(&self) -> i64 {
        self.mean_arterial_pressure_mpa
    }

    pub fn lung_diffusion_umol_per_s(&self) -> i64 {
        self.lung_diffusion_umol_per_s
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
    anatomy_joints: Vec<JointSpec>,
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
        let anatomy_joints = parse_anatomy_joints(&anatomy_nodes, &anatomy_edges, graph)?;
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

        let runtime_parameters = parse_runtime_parameters(object)?
            .with_package(anatomy_nodes.clone(), organ_bindings.clone());
        Ok(Self {
            morphotype_id,
            anatomy_nodes,
            anatomy_edges,
            anatomy_joints,
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

    /// Declared articulated joints in deterministic edge order.
    pub fn anatomy_joints(&self) -> &[JointSpec] {
        &self.anatomy_joints
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

/// Extracts declared joints from anatomy edges. A `joint` declaration is
/// only valid on an `articulated-joint-of` edge, and vice versa; both
/// endpoints must reference known anatomy nodes. Order follows the edge
/// array so the runtime skeleton is deterministic.
fn parse_anatomy_joints(
    nodes: &[AnatomyNode],
    edges: &[AnatomyEdge],
    graph: &serde_json::Map<String, Value>,
) -> Result<Vec<JointSpec>, MorphotypeError> {
    let raw_edges = graph
        .get("edges")
        .and_then(Value::as_array)
        .ok_or(MorphotypeError::InvalidJson)?;
    let known_nodes = nodes
        .iter()
        .map(|node| (node.node_id.as_str(), ()))
        .collect::<BTreeMap<_, ()>>();
    let mut joints = Vec::new();
    for (edge, raw) in edges.iter().zip(raw_edges) {
        let joint = raw.get("joint");
        if (edge.relation == "articulated-joint-of") != joint.is_some() {
            return Err(MorphotypeError::InvalidJson);
        }
        let Some(joint) = joint else { continue };
        if !known_nodes.contains_key(edge.from.as_str())
            || !known_nodes.contains_key(edge.to.as_str())
        {
            return Err(MorphotypeError::UnknownAnatomyNode(format!(
                "{}->{}",
                edge.from, edge.to
            )));
        }
        let limit_min_urad = joint
            .get("limit_min_urad")
            .and_then(Value::as_i64)
            .ok_or(MorphotypeError::InvalidJson)?;
        let limit_max_urad = joint
            .get("limit_max_urad")
            .and_then(Value::as_i64)
            .ok_or(MorphotypeError::InvalidJson)?;
        let driven_inertia_mgm2 = joint
            .get("driven_inertia_mgm2")
            .and_then(Value::as_i64)
            .filter(|inertia| *inertia >= 1)
            .ok_or(MorphotypeError::InvalidJson)?;
        if limit_min_urad > limit_max_urad {
            return Err(MorphotypeError::InvalidJson);
        }
        joints.push(
            JointSpec::new(
                edge.from.clone(),
                edge.to.clone(),
                limit_min_urad,
                limit_max_urad,
                driven_inertia_mgm2,
            )
            .map_err(|_| MorphotypeError::InvalidJson)?,
        );
    }
    Ok(joints)
}

fn parse_runtime_parameters(
    object: &serde_json::Map<String, Value>,
) -> Result<Morphotype, MorphotypeError> {
    let params = object
        .get("physiological_parameters")
        .and_then(Value::as_array)
        .ok_or(MorphotypeError::InvalidJson)?;
    let find = |id: &str| -> Result<i64, MorphotypeError> {
        let entry = params
            .iter()
            .find(|p| p.get("parameter_id").and_then(Value::as_str) == Some(id))
            .ok_or_else(|| {
                MorphotypeError::UnknownMorphotypeParameters(format!(
                    "missing physiological parameter {id}"
                ))
            })?;
        entry
            .get("value")
            .and_then(Value::as_f64)
            .filter(|v| v.is_finite())
            .map(|v| v as i64)
            .filter(|v| *v > 0)
            .ok_or(MorphotypeError::InvalidJson)
    };
    let find_optional = |id: &str, default: i64| -> i64 {
        params
            .iter()
            .find(|p| p.get("parameter_id").and_then(Value::as_str) == Some(id))
            .and_then(|e| e.get("value").and_then(Value::as_f64))
            .filter(|v| v.is_finite() && *v > 0.0)
            .map(|v| v as i64)
            .unwrap_or(default)
    };
    let awake = find("awake-metabolism-uj-per-s")?;
    let asleep = find("asleep-metabolism-uj-per-s")?;
    let night_awake = find("night-awake-metabolism-uj-per-s")?;
    let heat_capacity = find("core-heat-capacity-uj-per-mk")?;
    let conductance = find("ambient-conductance-uj-per-mk-s")?;
    // Phase 3: blood params optional for backward compat with minimal fixtures.
    let blood_volume = find_optional("blood-volume-mm3", 5_000_000);
    let hb = find_optional("hb-tetramer-umol", 11_500);
    let map = find_optional("mean-arterial-pressure-mpa", 12_400_000);
    let diffusion = find_optional("lung-diffusion-umol-per-s", 300);
    Ok(Morphotype::with_blood(
        awake,
        asleep,
        night_awake,
        heat_capacity,
        conductance,
        blood_volume,
        hb,
        map,
        diffusion,
    ))
}
