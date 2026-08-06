use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::path::Path;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{PathGuard, Result, WorldError};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct WorldPackage {
    schema_version: u32,
    world_id: String,
    locations: Vec<LocationDefinition>,
    connections: Vec<ConnectionDefinition>,
    objects: Vec<ObjectDefinition>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct LocationDefinition {
    id: String,
    name: String,
    anchors: Vec<AnchorDefinition>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct AnchorDefinition {
    id: String,
    name: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ConnectionDefinition {
    from: String,
    to: String,
    duration_ms: i64,
    bidirectional: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ObjectDefinition {
    id: String,
    name: String,
    anchor_id: String,
    #[serde(default)]
    observed_properties: BTreeMap<String, String>,
    #[serde(default)]
    hidden_properties: BTreeMap<String, String>,
    #[serde(default)]
    actions: Vec<ObjectActionDefinition>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ObjectActionDefinition {
    action_id: String,
    description: String,
    duration_ms: i64,
    #[serde(default)]
    required_resources: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Affordance {
    pub action_id: String,
    pub target_id: String,
    pub description: String,
    pub duration_ms: i64,
    pub required_resources: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct ObservedObjectDefinition {
    pub id: String,
    pub name: String,
    pub observed_properties: BTreeMap<String, String>,
    pub actions: Vec<Affordance>,
}

#[derive(Clone, Debug)]
pub struct WorldDefinition {
    world_id: String,
    hash: String,
    anchors: BTreeMap<String, String>,
    anchor_locations: BTreeMap<String, (String, String)>,
    connections: BTreeMap<(String, String), i64>,
    objects_by_anchor: BTreeMap<String, Vec<ObservedObjectDefinition>>,
}

impl WorldDefinition {
    pub fn load(manifest_path: impl AsRef<Path>, guard: &PathGuard) -> Result<Self> {
        let safe_path = guard.validate(manifest_path)?;
        let bytes = std::fs::read(&safe_path)?;
        let package: WorldPackage = serde_json::from_slice(&bytes).map_err(|error| {
            WorldError::InvalidDefinition(format!("{}: {error}", safe_path.display()))
        })?;
        Self::from_package(package)
    }

    fn from_package(package: WorldPackage) -> Result<Self> {
        if package.schema_version != 1 {
            return Err(WorldError::InvalidDefinition(format!(
                "unsupported schema_version {}",
                package.schema_version
            )));
        }
        require_id("world_id", &package.world_id)?;

        let canonical = serde_json::to_vec(&package)?;
        let hash = hex_digest(&canonical);
        let mut anchors = BTreeMap::new();
        let mut anchor_locations = BTreeMap::new();
        let mut location_ids = BTreeSet::new();

        for location in &package.locations {
            require_id("location.id", &location.id)?;
            if !location_ids.insert(location.id.clone()) {
                return Err(WorldError::InvalidDefinition(format!(
                    "duplicate location {}",
                    location.id
                )));
            }
            if location.anchors.is_empty() {
                return Err(WorldError::InvalidDefinition(format!(
                    "location {} has no anchors",
                    location.id
                )));
            }
            for anchor in &location.anchors {
                require_id("anchor.id", &anchor.id)?;
                if anchors
                    .insert(anchor.id.clone(), anchor.name.clone())
                    .is_some()
                {
                    return Err(WorldError::InvalidDefinition(format!(
                        "duplicate anchor {}",
                        anchor.id
                    )));
                }
                anchor_locations.insert(
                    anchor.id.clone(),
                    (location.id.clone(), location.name.clone()),
                );
            }
        }
        if anchors.is_empty() {
            return Err(WorldError::InvalidDefinition("world has no anchors".into()));
        }

        let mut connections = BTreeMap::new();
        for connection in &package.connections {
            if !anchors.contains_key(&connection.from) || !anchors.contains_key(&connection.to) {
                return Err(WorldError::InvalidDefinition(format!(
                    "connection {} -> {} references an unknown anchor",
                    connection.from, connection.to
                )));
            }
            if connection.from == connection.to || connection.duration_ms <= 0 {
                return Err(WorldError::InvalidDefinition(format!(
                    "connection {} -> {} has invalid duration or endpoints",
                    connection.from, connection.to
                )));
            }
            insert_connection(
                &mut connections,
                &connection.from,
                &connection.to,
                connection.duration_ms,
            )?;
            if connection.bidirectional {
                insert_connection(
                    &mut connections,
                    &connection.to,
                    &connection.from,
                    connection.duration_ms,
                )?;
            }
        }
        validate_reachability(
            anchors.keys().next().expect("anchors checked"),
            &anchors,
            &connections,
        )?;

        let mut object_ids = BTreeSet::new();
        let mut objects_by_anchor: BTreeMap<String, Vec<ObservedObjectDefinition>> =
            BTreeMap::new();
        for object in package.objects {
            require_id("object.id", &object.id)?;
            if !object_ids.insert(object.id.clone()) {
                return Err(WorldError::InvalidDefinition(format!(
                    "duplicate object {}",
                    object.id
                )));
            }
            if !anchors.contains_key(&object.anchor_id) {
                return Err(WorldError::InvalidDefinition(format!(
                    "object {} references unknown anchor {}",
                    object.id, object.anchor_id
                )));
            }
            let mut action_ids = BTreeSet::new();
            let actions = object
                .actions
                .into_iter()
                .map(|action| {
                    require_id("action.action_id", &action.action_id)?;
                    if action.duration_ms < 0 || !action_ids.insert(action.action_id.clone()) {
                        return Err(WorldError::InvalidDefinition(format!(
                            "object {} has duplicate action or invalid duration",
                            object.id
                        )));
                    }
                    let mut unique_resources = BTreeSet::new();
                    for resource in &action.required_resources {
                        if !is_known_resource(resource)
                            || !unique_resources.insert(resource.as_str())
                        {
                            return Err(WorldError::InvalidDefinition(format!(
                                "object {} action {} has an unknown or duplicate resource {}",
                                object.id, action.action_id, resource
                            )));
                        }
                    }
                    Ok(Affordance {
                        action_id: action.action_id,
                        target_id: object.id.clone(),
                        description: action.description,
                        duration_ms: action.duration_ms,
                        required_resources: action.required_resources,
                    })
                })
                .collect::<Result<Vec<_>>>()?;

            // Hidden properties are parsed and validated but deliberately not retained in the
            // perception-facing definition. Runtime systems will own them in a separate component.
            for key in object.hidden_properties.keys() {
                require_id("hidden property", key)?;
            }
            objects_by_anchor
                .entry(object.anchor_id)
                .or_default()
                .push(ObservedObjectDefinition {
                    id: object.id,
                    name: object.name,
                    observed_properties: object.observed_properties,
                    actions,
                });
        }

        Ok(Self {
            world_id: package.world_id,
            hash,
            anchors,
            anchor_locations,
            connections,
            objects_by_anchor,
        })
    }

    pub fn world_id(&self) -> &str {
        &self.world_id
    }

    pub fn hash(&self) -> &str {
        &self.hash
    }

    pub fn contains_anchor(&self, anchor_id: &str) -> bool {
        self.anchors.contains_key(anchor_id)
    }

    pub fn location_for_anchor(&self, anchor_id: &str) -> Option<(&str, &str)> {
        self.anchor_locations
            .get(anchor_id)
            .map(|(id, name)| (id.as_str(), name.as_str()))
    }

    pub fn movement_duration(&self, from: &str, to: &str) -> Option<i64> {
        self.connections
            .get(&(from.to_owned(), to.to_owned()))
            .copied()
    }

    pub fn movement_affordances(&self, from: &str) -> Vec<Affordance> {
        self.connections
            .iter()
            .filter(|((source, _), _)| source == from)
            .map(|((_, target), duration_ms)| Affordance {
                action_id: "world.move_to".into(),
                target_id: target.clone(),
                description: format!("Перейти к {}", self.anchors[target]),
                duration_ms: *duration_ms,
                required_resources: vec!["movement".into()],
            })
            .collect()
    }

    pub fn objects_at(&self, anchor_id: &str) -> &[ObservedObjectDefinition] {
        self.objects_by_anchor
            .get(anchor_id)
            .map(Vec::as_slice)
            .unwrap_or_default()
    }

    pub fn object_action(&self, target_id: &str, action_id: &str) -> Option<&Affordance> {
        self.objects_by_anchor
            .values()
            .flatten()
            .find(|object| object.id == target_id)
            .and_then(|object| {
                object
                    .actions
                    .iter()
                    .find(|action| action.action_id == action_id)
            })
    }
}

fn require_id(kind: &str, value: &str) -> Result<()> {
    let valid = !value.is_empty()
        && value.len() <= 96
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'_' | b'-' | b'.')
        });
    if valid {
        Ok(())
    } else {
        Err(WorldError::InvalidDefinition(format!(
            "{kind} must be a non-empty lowercase stable ID: {value:?}"
        )))
    }
}

fn insert_connection(
    connections: &mut BTreeMap<(String, String), i64>,
    from: &str,
    to: &str,
    duration_ms: i64,
) -> Result<()> {
    if connections
        .insert((from.to_owned(), to.to_owned()), duration_ms)
        .is_some()
    {
        return Err(WorldError::InvalidDefinition(format!(
            "duplicate connection {from} -> {to}"
        )));
    }
    Ok(())
}

fn validate_reachability(
    start: &str,
    anchors: &BTreeMap<String, String>,
    connections: &BTreeMap<(String, String), i64>,
) -> Result<()> {
    let mut visited = BTreeSet::from([start.to_owned()]);
    let mut queue = VecDeque::from([start.to_owned()]);
    while let Some(current) = queue.pop_front() {
        for (from, to) in connections.keys() {
            if from == &current && visited.insert(to.clone()) {
                queue.push_back(to.clone());
            }
        }
    }
    if visited.len() != anchors.len() {
        let missing = anchors
            .keys()
            .filter(|anchor| !visited.contains(*anchor))
            .cloned()
            .collect::<Vec<_>>();
        return Err(WorldError::InvalidDefinition(format!(
            "anchors are not reachable from {start}: {}",
            missing.join(", ")
        )));
    }
    Ok(())
}

fn is_known_resource(value: &str) -> bool {
    matches!(
        value,
        "movement" | "hands" | "vision" | "attention" | "speech" | "hearing" | "background"
    )
}

fn hex_digest(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}
