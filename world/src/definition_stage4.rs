use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::path::{Component, Path};

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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    initial_anchor_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    metadata: Option<WorldMetadataDefinition>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    map: Option<MapDefinition>,
    #[serde(default, skip_serializing_if = "TopologyPolicyDefinition::is_default")]
    topology: TopologyPolicyDefinition,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    object_templates: Vec<ObjectTemplateDefinition>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct WorldMetadataDefinition {
    city: String,
    timezone: String,
    floor: i32,
    area_m2: u32,
    weather_fallback: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    weather: Option<WeatherDefinition>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct WeatherDefinition {
    provider: String,
    latitude_e6: i32,
    longitude_e6: i32,
    poll_interval_ms: i64,
    stale_after_ms: i64,
    fallback_after_ms: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WeatherSite {
    pub provider: String,
    pub latitude_e6: i32,
    pub longitude_e6: i32,
    pub timezone: String,
    pub poll_interval_ms: i64,
    pub stale_after_ms: i64,
    pub fallback_after_ms: i64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct MapDefinition {
    asset: String,
    width: u32,
    height: u32,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct TopologyPolicyDefinition {
    #[serde(default)]
    require_strong_connectivity: bool,
}

impl TopologyPolicyDefinition {
    fn is_default(&self) -> bool {
        !self.require_strong_connectivity
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct LocationDefinition {
    id: String,
    name: String,
    anchors: Vec<AnchorDefinition>,
    #[serde(default, skip_serializing_if = "SensoryDefinition::is_empty")]
    sensory: SensoryDefinition,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct AnchorDefinition {
    id: String,
    name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    map_point: Option<MapPointDefinition>,
    #[serde(default, skip_serializing_if = "SensoryDefinition::is_empty")]
    sensory: SensoryDefinition,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct SensoryDefinition {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    light: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    sound: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    temperature: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    smell: Vec<String>,
}

impl SensoryDefinition {
    fn is_empty(&self) -> bool {
        self.light.is_empty()
            && self.sound.is_empty()
            && self.temperature.is_empty()
            && self.smell.is_empty()
    }

    fn descriptions(&self) -> impl Iterator<Item = String> + '_ {
        [
            ("Свет", &self.light),
            ("Звук", &self.sound),
            ("Температура", &self.temperature),
            ("Запах", &self.smell),
        ]
        .into_iter()
        .flat_map(|(kind, descriptions)| {
            descriptions
                .iter()
                .map(move |description| format!("{kind}: {description}"))
        })
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct MapPointDefinition {
    x: u32,
    y: u32,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ConnectionDefinition {
    from: String,
    to: String,
    duration_ms: i64,
    bidirectional: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    passage_id: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ObjectTemplateDefinition {
    id: String,
    name: String,
    dimensions_mm: DimensionsDefinition,
    mass_g: u64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    components: Vec<String>,
    #[serde(default)]
    placement_requires_power: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    slots: Vec<PlacementSlotDefinition>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    observed_properties: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    hidden_properties: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    actions: Vec<ObjectActionDefinition>,
    #[serde(
        default,
        skip_serializing_if = "InitialObjectStateDefinition::is_empty"
    )]
    initial_state: InitialObjectStateDefinition,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    passive_effects: Vec<PassiveEffect>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct DimensionsDefinition {
    width: u32,
    depth: u32,
    height: u32,
}

impl DimensionsDefinition {
    fn is_valid(&self) -> bool {
        self.width > 0 && self.depth > 0 && self.height > 0
    }

    fn fits_inside(&self, limit: &Self) -> bool {
        self.width <= limit.width && self.depth <= limit.depth && self.height <= limit.height
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct PlacementSlotDefinition {
    id: String,
    kind: PlacementRelation,
    capacity: u32,
    max_total_mass_g: u64,
    max_item_dimensions_mm: DimensionsDefinition,
    #[serde(default)]
    provides_power: bool,
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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    components: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    template_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    placement: Option<PlacementDefinition>,
    #[serde(
        default,
        skip_serializing_if = "InitialObjectStateDefinition::is_empty"
    )]
    initial_state: InitialObjectStateDefinition,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    passive_effects: Vec<PassiveEffect>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PassiveActivation {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub power: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub open: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub powered_placement: Option<bool>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum PassiveEffect {
    Charge {
        id: String,
        #[serde(default)]
        when: PassiveActivation,
        active_delta_per_hour_permille: i64,
        #[serde(default)]
        inactive_delta_per_hour_permille: i64,
    },
    Temperature {
        id: String,
        #[serde(default)]
        when: PassiveActivation,
        active_target_millicelsius: i32,
        active_change_per_hour_millicelsius: i64,
        inactive_target_millicelsius: i32,
        inactive_change_per_hour_millicelsius: i64,
    },
    QuantityConsumption {
        id: String,
        #[serde(default)]
        when: PassiveActivation,
        active_amount_per_hour: u64,
        #[serde(default)]
        inactive_amount_per_hour: u64,
    },
}

impl PassiveEffect {
    pub(crate) fn id(&self) -> &str {
        match self {
            Self::Charge { id, .. }
            | Self::Temperature { id, .. }
            | Self::QuantityConsumption { id, .. } => id,
        }
    }

    fn property(&self) -> &'static str {
        match self {
            Self::Charge { .. } => "charge_permille",
            Self::Temperature { .. } => "temperature_millicelsius",
            Self::QuantityConsumption { .. } => "quantity",
        }
    }

    pub(crate) fn activation(&self) -> &PassiveActivation {
        match self {
            Self::Charge { when, .. }
            | Self::Temperature { when, .. }
            | Self::QuantityConsumption { when, .. } => when,
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct InitialObjectStateDefinition {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    power: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    open: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    charge_permille: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    cleanliness_permille: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    quantity: Option<ObjectQuantity>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    temperature_millicelsius: Option<i32>,
}

impl InitialObjectStateDefinition {
    fn is_empty(&self) -> bool {
        self.power.is_none()
            && self.open.is_none()
            && self.charge_permille.is_none()
            && self.cleanliness_permille.is_none()
            && self.quantity.is_none()
            && self.temperature_millicelsius.is_none()
    }

    fn merged_with(&self, overrides: &Self) -> Self {
        Self {
            power: overrides.power.or(self.power),
            open: overrides.open.or(self.open),
            charge_permille: overrides.charge_permille.or(self.charge_permille),
            cleanliness_permille: overrides.cleanliness_permille.or(self.cleanliness_permille),
            quantity: overrides.quantity.clone().or_else(|| self.quantity.clone()),
            temperature_millicelsius: overrides
                .temperature_millicelsius
                .or(self.temperature_millicelsius),
        }
    }

    fn condition(&self, components: &BTreeSet<String>) -> ObjectCondition {
        ObjectCondition {
            charge_permille: self
                .charge_permille
                .or_else(|| components.contains("chargeable").then_some(1_000)),
            cleanliness_permille: self
                .cleanliness_permille
                .or_else(|| components.contains("cleanable").then_some(1_000)),
            quantity: self.quantity.clone(),
            temperature_millicelsius: self.temperature_millicelsius,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum QuantityUnit {
    Count,
    Serving,
    Milliliter,
    Gram,
}

impl QuantityUnit {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Count => "count",
            Self::Serving => "serving",
            Self::Milliliter => "milliliter",
            Self::Gram => "gram",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ObjectQuantity {
    pub amount: u64,
    pub unit: QuantityUnit,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ObjectCondition {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub charge_permille: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cleanliness_permille: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quantity: Option<ObjectQuantity>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub temperature_millicelsius: Option<i32>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct PlacementDefinition {
    relation: PlacementRelation,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    parent_object_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    slot_id: Option<String>,
}

impl Default for PlacementDefinition {
    fn default() -> Self {
        Self {
            relation: PlacementRelation::Anchor,
            parent_object_id: None,
            slot_id: None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PlacementRelation {
    Anchor,
    Surface,
    Container,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct ObjectPlacement {
    pub anchor_id: String,
    pub relation: PlacementRelation,
    pub parent_object_id: Option<String>,
    pub slot_id: Option<String>,
}

impl ObjectPlacement {
    fn from_record(object: &ObjectRecord) -> Self {
        Self {
            anchor_id: object.anchor_id.clone(),
            relation: object.placement.relation,
            parent_object_id: object.placement.parent_object_id.clone(),
            slot_id: object.placement.slot_id.clone(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PlacementViolation {
    pub code: &'static str,
    pub message: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ObjectActionDefinition {
    action_id: String,
    description: String,
    duration_ms: i64,
    #[serde(default)]
    required_resources: Vec<String>,
    #[serde(default = "default_interruptibility")]
    interruptibility: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    preconditions: Vec<ActionPreconditionDefinition>,
}

fn default_interruptibility() -> String {
    "immediate".into()
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ActionPreconditionDefinition {
    property: String,
    equals: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Affordance {
    pub action_id: String,
    pub target_id: String,
    pub description: String,
    pub duration_ms: i64,
    pub required_resources: Vec<String>,
    #[serde(default = "empty_parameters_schema")]
    pub parameters_schema_json: String,
}

#[derive(Clone, Debug)]
pub struct ObservedObjectDefinition {
    pub id: String,
    pub name: String,
    pub observed_properties: BTreeMap<String, String>,
    pub actions: Vec<Affordance>,
}

#[derive(Clone, Debug)]
struct ValidatedTemplate {
    dimensions_mm: DimensionsDefinition,
    mass_g: u64,
    components: BTreeSet<String>,
    placement_requires_power: bool,
    slots: BTreeMap<String, PlacementSlotDefinition>,
    observed_properties: BTreeMap<String, String>,
    actions: Vec<ObjectActionDefinition>,
    initial_state: InitialObjectStateDefinition,
    passive_effects: BTreeMap<String, PassiveEffect>,
}

#[derive(Clone, Debug)]
struct ObjectRecord {
    template_id: Option<String>,
    anchor_id: String,
    placement: PlacementDefinition,
    dimensions_mm: Option<DimensionsDefinition>,
    mass_g: u64,
    components: BTreeSet<String>,
    placement_requires_power: bool,
    initial_power: bool,
    initial_open: bool,
    initial_condition: ObjectCondition,
    passive_effects: Vec<PassiveEffect>,
}

type ObjectCatalog = BTreeMap<String, ObjectRecord>;
type PerceptionObjectIndex = BTreeMap<String, Vec<ObservedObjectDefinition>>;
type ValidatedObjects = (ObjectCatalog, PerceptionObjectIndex);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConnectionView {
    pub from: String,
    pub to: String,
    pub duration_ms: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObjectView {
    pub id: String,
    pub template_id: Option<String>,
    pub anchor_id: String,
    pub placement_relation: PlacementRelation,
    pub parent_object_id: Option<String>,
    pub slot_id: Option<String>,
    pub components: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct WorldDefinition {
    world_id: String,
    hash: String,
    initial_anchor_id: String,
    locations: BTreeMap<String, String>,
    anchors: BTreeMap<String, String>,
    anchor_locations: BTreeMap<String, (String, String)>,
    connections: BTreeMap<(String, String), i64>,
    templates: BTreeMap<String, ValidatedTemplate>,
    objects: BTreeMap<String, ObjectRecord>,
    objects_by_anchor: BTreeMap<String, Vec<ObservedObjectDefinition>>,
    sensory_by_anchor: BTreeMap<String, Vec<String>>,
    weather_site: Option<WeatherSite>,
}

impl WorldDefinition {
    pub fn load(manifest_path: impl AsRef<Path>, guard: &PathGuard) -> Result<Self> {
        let safe_path = guard.validate(manifest_path)?;
        let bytes = std::fs::read(&safe_path)?;
        let package: WorldPackage = serde_json::from_slice(&bytes).map_err(|error| {
            WorldError::InvalidDefinition(format!("{}: {error}", safe_path.display()))
        })?;
        let map_bytes = validate_map_asset(&package, &safe_path, guard)?;
        Self::from_package(package, map_bytes.as_deref())
    }

    fn from_package(mut package: WorldPackage, map_bytes: Option<&[u8]>) -> Result<Self> {
        if package.schema_version != 1 {
            return Err(WorldError::InvalidDefinition(format!(
                "unsupported schema_version {}",
                package.schema_version
            )));
        }
        require_id("world_id", &package.world_id)?;
        validate_metadata(package.metadata.as_ref())?;
        normalize_package(&mut package);

        let mut canonical = serde_json::to_vec(&package)?;
        if let Some(bytes) = map_bytes {
            canonical.extend_from_slice(b"\0map-asset\0");
            canonical.extend_from_slice(bytes);
        }
        let hash = hex_digest(&canonical);
        let mut locations = BTreeMap::new();
        let mut anchors = BTreeMap::new();
        let mut anchor_locations = BTreeMap::new();
        let mut sensory_by_anchor = BTreeMap::new();
        let weather_site = package.metadata.as_ref().and_then(|metadata| {
            metadata.weather.as_ref().map(|weather| WeatherSite {
                provider: weather.provider.clone(),
                latitude_e6: weather.latitude_e6,
                longitude_e6: weather.longitude_e6,
                timezone: metadata.timezone.clone(),
                poll_interval_ms: weather.poll_interval_ms,
                stale_after_ms: weather.stale_after_ms,
                fallback_after_ms: weather.fallback_after_ms,
            })
        });
        let weather_fallback = package
            .metadata
            .as_ref()
            .map(|metadata| format!("Погода (локальный fallback): {}", metadata.weather_fallback));

        for location in &package.locations {
            require_id("location.id", &location.id)?;
            require_name("location.name", &location.name)?;
            validate_sensory(&location.sensory)?;
            if locations
                .insert(location.id.clone(), location.name.clone())
                .is_some()
            {
                return invalid(format!("duplicate location {}", location.id));
            }
            if location.anchors.is_empty() {
                return invalid(format!("location {} has no anchors", location.id));
            }
            for anchor in &location.anchors {
                require_id("anchor.id", &anchor.id)?;
                require_name("anchor.name", &anchor.name)?;
                validate_sensory(&anchor.sensory)?;
                validate_map_point(anchor, package.map.as_ref())?;
                if anchors
                    .insert(anchor.id.clone(), anchor.name.clone())
                    .is_some()
                {
                    return invalid(format!("duplicate anchor {}", anchor.id));
                }
                anchor_locations.insert(
                    anchor.id.clone(),
                    (location.id.clone(), location.name.clone()),
                );
                let cues = location
                    .sensory
                    .descriptions()
                    .chain(anchor.sensory.descriptions())
                    .chain(weather_fallback.iter().cloned())
                    .collect();
                sensory_by_anchor.insert(anchor.id.clone(), cues);
            }
        }
        if anchors.is_empty() {
            return invalid("world has no anchors");
        }

        let initial_anchor_id = package
            .initial_anchor_id
            .clone()
            .unwrap_or_else(|| anchors.keys().next().expect("anchors checked").clone());
        if !anchors.contains_key(&initial_anchor_id) {
            return invalid(format!("initial anchor {initial_anchor_id} does not exist"));
        }

        let mut connections = BTreeMap::new();
        for connection in &package.connections {
            if let Some(passage_id) = &connection.passage_id {
                require_id("connection.passage_id", passage_id)?;
            }
            if !anchors.contains_key(&connection.from) || !anchors.contains_key(&connection.to) {
                return invalid(format!(
                    "connection {} -> {} references an unknown anchor",
                    connection.from, connection.to
                ));
            }
            if connection.from == connection.to || connection.duration_ms <= 0 {
                return invalid(format!(
                    "connection {} -> {} has invalid duration or endpoints",
                    connection.from, connection.to
                ));
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
        validate_reachability(&initial_anchor_id, &anchors, &connections)?;
        if package.topology.require_strong_connectivity {
            for anchor_id in anchors.keys() {
                validate_reachability(anchor_id, &anchors, &connections)?;
            }
        }

        let templates = validate_templates(&package.object_templates)?;
        let (objects, objects_by_anchor) = validate_objects(package.objects, &anchors, &templates)?;

        Ok(Self {
            world_id: package.world_id,
            hash,
            initial_anchor_id,
            locations,
            anchors,
            anchor_locations,
            connections,
            templates,
            objects,
            objects_by_anchor,
            sensory_by_anchor,
            weather_site,
        })
    }

    pub fn world_id(&self) -> &str {
        &self.world_id
    }

    pub fn hash(&self) -> &str {
        &self.hash
    }

    pub fn initial_anchor_id(&self) -> &str {
        &self.initial_anchor_id
    }

    pub fn location_ids(&self) -> impl ExactSizeIterator<Item = &str> {
        self.locations.keys().map(String::as_str)
    }

    pub fn anchor_ids(&self) -> impl ExactSizeIterator<Item = &str> {
        self.anchors.keys().map(String::as_str)
    }

    pub fn template_ids(&self) -> impl ExactSizeIterator<Item = &str> {
        self.templates.keys().map(String::as_str)
    }

    pub fn objects(&self) -> impl ExactSizeIterator<Item = ObjectView> + '_ {
        self.objects.iter().map(|(id, object)| ObjectView {
            id: id.clone(),
            template_id: object.template_id.clone(),
            anchor_id: object.anchor_id.clone(),
            placement_relation: object.placement.relation,
            parent_object_id: object.placement.parent_object_id.clone(),
            slot_id: object.placement.slot_id.clone(),
            components: object.components.iter().cloned().collect(),
        })
    }

    pub fn connections(&self) -> impl ExactSizeIterator<Item = ConnectionView> + '_ {
        self.connections
            .iter()
            .map(|((from, to), duration_ms)| ConnectionView {
                from: from.clone(),
                to: to.clone(),
                duration_ms: *duration_ms,
            })
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
        shortest_path_duration(from, to, &self.anchors, &self.connections)
    }

    pub fn movement_affordances(&self, from: &str) -> Vec<Affordance> {
        self.anchors
            .iter()
            .filter(|(target, _)| target.as_str() != from)
            .filter_map(|(target, name)| {
                self.movement_duration(from, target)
                    .map(|duration_ms| Affordance {
                        action_id: "world.move_to".into(),
                        target_id: target.clone(),
                        description: format!("Перейти к {name}"),
                        duration_ms,
                        required_resources: vec!["movement".into()],
                        parameters_schema_json: empty_parameters_schema(),
                    })
            })
            .collect()
    }

    pub fn objects_at(&self, anchor_id: &str) -> &[ObservedObjectDefinition] {
        self.objects_by_anchor
            .get(anchor_id)
            .map(Vec::as_slice)
            .unwrap_or_default()
    }

    pub(crate) fn observed_objects(&self) -> impl Iterator<Item = &ObservedObjectDefinition> {
        self.objects_by_anchor.values().flatten()
    }

    pub fn sensory_cues(&self, anchor_id: &str) -> &[String] {
        self.sensory_by_anchor
            .get(anchor_id)
            .map(Vec::as_slice)
            .unwrap_or_default()
    }

    pub fn weather_site(&self) -> Option<&WeatherSite> {
        self.weather_site.as_ref()
    }

    pub(crate) fn initial_object_placement(&self, object_id: &str) -> Option<ObjectPlacement> {
        self.objects
            .get(object_id)
            .map(ObjectPlacement::from_record)
    }

    pub(crate) fn initial_object_placements(
        &self,
    ) -> impl Iterator<Item = (String, ObjectPlacement)> + '_ {
        self.objects
            .iter()
            .map(|(id, object)| (id.clone(), ObjectPlacement::from_record(object)))
    }

    pub(crate) fn validate_placement_change(
        &self,
        object_id: &str,
        candidate: &ObjectPlacement,
        current: &BTreeMap<String, ObjectPlacement>,
    ) -> std::result::Result<(), PlacementViolation> {
        let object = self.objects.get(object_id).ok_or_else(|| {
            placement_violation(
                "UNKNOWN_OBJECT",
                format!("object {object_id} does not exist"),
            )
        })?;
        if !object.components.contains("movable") {
            return Err(placement_violation(
                "OBJECT_NOT_MOVABLE",
                format!("object {object_id} is not movable"),
            ));
        }
        if current.get(object_id) == Some(candidate) {
            return Err(placement_violation(
                "ALREADY_PLACED",
                "object already has the requested placement",
            ));
        }
        let mut placements = current.clone();
        placements.insert(object_id.into(), candidate.clone());
        self.validate_runtime_placements(&placements)
    }

    pub(crate) fn initial_object_power(&self, object_id: &str) -> bool {
        self.objects
            .get(object_id)
            .is_some_and(|object| object.initial_power)
    }

    pub(crate) fn initial_object_open(&self, object_id: &str) -> bool {
        self.objects
            .get(object_id)
            .is_some_and(|object| object.initial_open)
    }

    pub(crate) fn initial_object_condition(&self, object_id: &str) -> ObjectCondition {
        self.objects
            .get(object_id)
            .map(|object| object.initial_condition.clone())
            .unwrap_or_default()
    }

    pub(crate) fn passive_objects(&self) -> impl Iterator<Item = (&str, &[PassiveEffect])> + '_ {
        self.objects
            .iter()
            .filter(|(_, object)| !object.passive_effects.is_empty())
            .map(|(id, object)| (id.as_str(), object.passive_effects.as_slice()))
    }

    pub(crate) fn object_receives_placement_power(
        &self,
        placement: Option<&ObjectPlacement>,
    ) -> bool {
        let Some(placement) = placement else {
            return false;
        };
        let (Some(parent_id), Some(slot_id)) = (
            placement.parent_object_id.as_deref(),
            placement.slot_id.as_deref(),
        ) else {
            return false;
        };
        self.objects
            .get(parent_id)
            .and_then(|parent| parent.template_id.as_deref())
            .and_then(|template_id| self.templates.get(template_id))
            .and_then(|template| template.slots.get(slot_id))
            .is_some_and(|slot| slot.provides_power)
    }

    pub(crate) fn object_has_component(&self, object_id: &str, component: &str) -> bool {
        self.objects
            .get(object_id)
            .is_some_and(|object| object.components.contains(component))
    }

    pub(crate) fn object_has_template(&self, object_id: &str, template_id: &str) -> bool {
        self.objects
            .get(object_id)
            .and_then(|object| object.template_id.as_deref())
            == Some(template_id)
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

    fn validate_runtime_placements(
        &self,
        placements: &BTreeMap<String, ObjectPlacement>,
    ) -> std::result::Result<(), PlacementViolation> {
        if placements.len() != self.objects.len()
            || self.objects.keys().any(|id| !placements.contains_key(id))
        {
            return Err(placement_violation(
                "PLACEMENT_STATE_INCOMPLETE",
                "placement state does not cover every world object",
            ));
        }
        for id in self.objects.keys() {
            let mut current = id.as_str();
            let mut visited = BTreeSet::new();
            loop {
                if !visited.insert(current) {
                    return Err(placement_violation(
                        "PLACEMENT_CYCLE",
                        format!("placement cycle contains object {current}"),
                    ));
                }
                let placement = placements.get(current).ok_or_else(|| {
                    placement_violation(
                        "UNKNOWN_PARENT",
                        format!("placement chain references unknown object {current}"),
                    )
                })?;
                if placement.relation == PlacementRelation::Anchor {
                    break;
                }
                current = placement.parent_object_id.as_deref().ok_or_else(|| {
                    placement_violation(
                        "INVALID_PLACEMENT",
                        format!("placed object {current} has no parent"),
                    )
                })?;
            }
        }
        let mut occupancy: BTreeMap<(String, String), (u32, u64)> = BTreeMap::new();
        for (id, object) in &self.objects {
            let placement = &placements[id];
            if !self.anchors.contains_key(&placement.anchor_id) {
                return Err(placement_violation(
                    "UNKNOWN_ANCHOR",
                    format!(
                        "object {id} references unknown anchor {}",
                        placement.anchor_id
                    ),
                ));
            }
            let has_parent = placement.parent_object_id.is_some();
            let has_slot = placement.slot_id.is_some();
            if placement.relation == PlacementRelation::Anchor {
                if has_parent || has_slot {
                    return Err(placement_violation(
                        "INVALID_PLACEMENT",
                        format!("anchor placement for {id} cannot declare parent or slot"),
                    ));
                }
                if object.placement_requires_power {
                    return Err(placement_violation(
                        "POWER_REQUIRED",
                        format!("object {id} requires a powered placement slot"),
                    ));
                }
                continue;
            }
            let Some(parent_id) = placement.parent_object_id.as_deref() else {
                return Err(placement_violation(
                    "INVALID_PLACEMENT",
                    format!("placed object {id} requires parent_object_id"),
                ));
            };
            let Some(slot_id) = placement.slot_id.as_deref() else {
                return Err(placement_violation(
                    "INVALID_PLACEMENT",
                    format!("placed object {id} requires slot_id"),
                ));
            };
            let parent = self.objects.get(parent_id).ok_or_else(|| {
                placement_violation(
                    "UNKNOWN_PARENT",
                    format!("object {id} references unknown parent {parent_id}"),
                )
            })?;
            let parent_placement = &placements[parent_id];
            if parent_placement.anchor_id != placement.anchor_id {
                return Err(placement_violation(
                    "ANCHOR_MISMATCH",
                    format!("object {id} and parent {parent_id} are at different anchors"),
                ));
            }
            let parent_template_id = parent.template_id.as_deref().ok_or_else(|| {
                placement_violation(
                    "UNKNOWN_SLOT",
                    format!("placement parent {parent_id} has no object template"),
                )
            })?;
            let slot = self.templates[parent_template_id]
                .slots
                .get(slot_id)
                .ok_or_else(|| {
                    placement_violation(
                        "UNKNOWN_SLOT",
                        format!("unknown placement slot {parent_id}.{slot_id}"),
                    )
                })?;
            if slot.kind != placement.relation {
                return Err(placement_violation(
                    "SLOT_TYPE_MISMATCH",
                    format!("slot {parent_id}.{slot_id} has incompatible placement type"),
                ));
            }
            let dimensions = object.dimensions_mm.as_ref().ok_or_else(|| {
                placement_violation(
                    "MISSING_PHYSICAL_MODEL",
                    format!("object {id} has no physical template"),
                )
            })?;
            if !dimensions.fits_inside(&slot.max_item_dimensions_mm) {
                return Err(placement_violation(
                    "DOES_NOT_FIT",
                    format!("object {id} does not fit in {parent_id}.{slot_id}"),
                ));
            }
            if object.placement_requires_power && !slot.provides_power {
                return Err(placement_violation(
                    "POWER_REQUIRED",
                    format!("object {id} requires a powered placement slot"),
                ));
            }
            let entry = occupancy
                .entry((parent_id.into(), slot_id.into()))
                .or_insert((0, 0));
            entry.0 = entry.0.checked_add(1).ok_or_else(|| {
                placement_violation("CAPACITY_EXCEEDED", "placement item count overflow")
            })?;
            entry.1 = entry.1.checked_add(object.mass_g).ok_or_else(|| {
                placement_violation("CAPACITY_EXCEEDED", "placement mass overflow")
            })?;
            if entry.0 > slot.capacity || entry.1 > slot.max_total_mass_g {
                return Err(placement_violation(
                    "CAPACITY_EXCEEDED",
                    format!("placement capacity exceeded at {parent_id}.{slot_id}"),
                ));
            }
        }
        Ok(())
    }
}

fn placement_violation(code: &'static str, message: impl Into<String>) -> PlacementViolation {
    PlacementViolation {
        code,
        message: message.into(),
    }
}

fn validate_templates(
    definitions: &[ObjectTemplateDefinition],
) -> Result<BTreeMap<String, ValidatedTemplate>> {
    let mut templates = BTreeMap::new();
    for template in definitions {
        require_id("template.id", &template.id)?;
        require_name("template.name", &template.name)?;
        if !template.dimensions_mm.is_valid() || template.mass_g == 0 {
            return invalid(format!(
                "template {} has invalid physical dimensions",
                template.id
            ));
        }
        let mut components = BTreeSet::new();
        for component in &template.components {
            if !is_known_component(component) || !components.insert(component.clone()) {
                return invalid(format!(
                    "template {} has unknown or duplicate component {component}",
                    template.id
                ));
            }
        }
        validate_properties("template observed property", &template.observed_properties)?;
        validate_properties("template hidden property", &template.hidden_properties)?;
        validate_initial_state(&template.id, &template.initial_state, &components)?;
        let passive_effects = validate_passive_effects(
            &template.id,
            template.passive_effects.clone(),
            &components,
            &template.initial_state,
        )?;
        let mut template_actions = template.actions.clone();
        add_component_actions(&template.name, &components, &mut template_actions);
        let actions = validate_actions(&template.id, &template_actions)?;
        let mut slots = BTreeMap::new();
        for slot in &template.slots {
            require_id("slot.id", &slot.id)?;
            if slot.kind == PlacementRelation::Anchor
                || slot.capacity == 0
                || slot.max_total_mass_g == 0
                || !slot.max_item_dimensions_mm.is_valid()
            {
                return invalid(format!(
                    "template {} has invalid slot {}",
                    template.id, slot.id
                ));
            }
            let required_component = match slot.kind {
                PlacementRelation::Surface => "surface",
                PlacementRelation::Container => "container",
                PlacementRelation::Anchor => unreachable!(),
            };
            if !components.contains(required_component) {
                return invalid(format!(
                    "template {} slot {} requires component {required_component}",
                    template.id, slot.id
                ));
            }
            if slots.insert(slot.id.clone(), slot.clone()).is_some() {
                return invalid(format!(
                    "template {} has duplicate slot {}",
                    template.id, slot.id
                ));
            }
        }
        let validated = ValidatedTemplate {
            dimensions_mm: template.dimensions_mm.clone(),
            mass_g: template.mass_g,
            components,
            placement_requires_power: template.placement_requires_power,
            slots,
            observed_properties: template.observed_properties.clone(),
            actions,
            initial_state: template.initial_state.clone(),
            passive_effects,
        };
        if templates.insert(template.id.clone(), validated).is_some() {
            return invalid(format!("duplicate template {}", template.id));
        }
    }
    Ok(templates)
}

fn validate_objects(
    definitions: Vec<ObjectDefinition>,
    anchors: &BTreeMap<String, String>,
    templates: &BTreeMap<String, ValidatedTemplate>,
) -> Result<ValidatedObjects> {
    let mut objects = BTreeMap::new();
    let mut observed = BTreeMap::new();
    for object in definitions {
        require_id("object.id", &object.id)?;
        require_name("object.name", &object.name)?;
        if !anchors.contains_key(&object.anchor_id) {
            return invalid(format!(
                "object {} references unknown anchor {}",
                object.id, object.anchor_id
            ));
        }
        validate_properties("object observed property", &object.observed_properties)?;
        validate_properties("object hidden property", &object.hidden_properties)?;
        let template = object
            .template_id
            .as_ref()
            .map(|id| {
                templates.get(id).ok_or_else(|| {
                    WorldError::InvalidDefinition(format!(
                        "object {} references unknown template {id}",
                        object.id
                    ))
                })
            })
            .transpose()?;
        let mut observed_properties = template
            .map(|value| value.observed_properties.clone())
            .unwrap_or_default();
        let components = template
            .map(|value| value.components.clone())
            .unwrap_or_default();
        let mut components = components;
        for component in &object.components {
            if !is_known_component(component) || !components.insert(component.clone()) {
                return invalid(format!(
                    "object {} has unknown or duplicate component {component}",
                    object.id
                ));
            }
        }
        observed_properties.extend(object.observed_properties);
        let mut actions = template
            .map(|value| value.actions.clone())
            .unwrap_or_default();
        actions.extend(object.actions);
        add_component_actions(&object.name, &components, &mut actions);
        let actions = validate_actions(&object.id, &actions)?
            .into_iter()
            .map(|action| {
                let parameters_schema_json = action_parameters_schema(&action.action_id).into();
                Affordance {
                    action_id: action.action_id,
                    target_id: object.id.clone(),
                    description: action.description,
                    duration_ms: action.duration_ms,
                    required_resources: action.required_resources,
                    parameters_schema_json,
                }
            })
            .collect();
        let placement = object.placement.unwrap_or_default();
        validate_placement_shape(&object.id, &placement)?;
        let initial_state = template
            .map(|value| value.initial_state.merged_with(&object.initial_state))
            .unwrap_or_else(|| object.initial_state.clone());
        validate_initial_state(&object.id, &initial_state, &components)?;
        let mut passive_effects = template
            .map(|value| value.passive_effects.clone())
            .unwrap_or_default();
        for effect in object.passive_effects.clone() {
            passive_effects.insert(effect.id().to_owned(), effect);
        }
        let passive_effects = validate_passive_effects(
            &object.id,
            passive_effects.into_values().collect(),
            &components,
            &initial_state,
        )?
        .into_values()
        .collect();
        let initial_condition = initial_state.condition(&components);
        let record = ObjectRecord {
            template_id: object.template_id,
            anchor_id: object.anchor_id.clone(),
            placement,
            dimensions_mm: template.map(|value| value.dimensions_mm.clone()),
            mass_g: template.map(|value| value.mass_g).unwrap_or(0),
            components,
            placement_requires_power: template
                .map(|value| value.placement_requires_power)
                .unwrap_or(false),
            initial_power: initial_state.power.unwrap_or(false),
            initial_open: initial_state.open.unwrap_or(false),
            initial_condition,
            passive_effects,
        };
        if objects.insert(object.id.clone(), record).is_some() {
            return invalid(format!("duplicate object {}", object.id));
        }
        observed.insert(
            object.id.clone(),
            ObservedObjectDefinition {
                id: object.id,
                name: object.name,
                observed_properties,
                actions,
            },
        );
    }
    validate_placements(&objects, templates)?;

    let mut objects_by_anchor: BTreeMap<String, Vec<ObservedObjectDefinition>> = BTreeMap::new();
    for (id, object) in &objects {
        objects_by_anchor
            .entry(object.anchor_id.clone())
            .or_default()
            .push(observed.remove(id).expect("observed object exists"));
    }
    for objects in objects_by_anchor.values_mut() {
        objects.sort_by(|left, right| left.id.cmp(&right.id));
    }
    Ok((objects, objects_by_anchor))
}

fn validate_placements(
    objects: &BTreeMap<String, ObjectRecord>,
    templates: &BTreeMap<String, ValidatedTemplate>,
) -> Result<()> {
    let mut occupancy: BTreeMap<(String, String), (u32, u64)> = BTreeMap::new();
    for (id, object) in objects {
        if object.placement.relation == PlacementRelation::Anchor {
            continue;
        }
        let parent_id = object
            .placement
            .parent_object_id
            .as_deref()
            .expect("placement shape validated");
        let slot_id = object
            .placement
            .slot_id
            .as_deref()
            .expect("placement shape validated");
        let parent = objects.get(parent_id).ok_or_else(|| {
            WorldError::InvalidDefinition(format!("object {id} has unknown parent {parent_id}"))
        })?;
        if parent.anchor_id != object.anchor_id {
            return invalid(format!(
                "object {id} and parent {parent_id} are at different anchors"
            ));
        }
        let parent_template_id = parent.template_id.as_deref().ok_or_else(|| {
            WorldError::InvalidDefinition(format!("placement parent {parent_id} has no template"))
        })?;
        let slot = templates[parent_template_id]
            .slots
            .get(slot_id)
            .ok_or_else(|| {
                WorldError::InvalidDefinition(format!(
                    "object {id} references unknown slot {parent_id}.{slot_id}"
                ))
            })?;
        if slot.kind != object.placement.relation {
            return invalid(format!("object {id} uses an incompatible placement slot"));
        }
        let dimensions = object.dimensions_mm.as_ref().ok_or_else(|| {
            WorldError::InvalidDefinition(format!("placed object {id} has no physical template"))
        })?;
        if !dimensions.fits_inside(&slot.max_item_dimensions_mm) {
            return invalid(format!("object {id} does not fit in {parent_id}.{slot_id}"));
        }
        if object.placement_requires_power && !slot.provides_power {
            return invalid(format!("object {id} requires a powered placement slot"));
        }
        let entry = occupancy
            .entry((parent_id.into(), slot_id.into()))
            .or_insert((0, 0));
        entry.0 = entry
            .0
            .checked_add(1)
            .ok_or_else(|| WorldError::InvalidDefinition("placement occupancy overflow".into()))?;
        entry.1 = entry
            .1
            .checked_add(object.mass_g)
            .ok_or_else(|| WorldError::InvalidDefinition("placement mass overflow".into()))?;
        if entry.0 > slot.capacity || entry.1 > slot.max_total_mass_g {
            return invalid(format!(
                "placement capacity exceeded at {parent_id}.{slot_id}"
            ));
        }
    }
    for id in objects.keys() {
        validate_placement_chain(id, objects)?;
    }
    Ok(())
}

fn validate_placement_chain(id: &str, objects: &BTreeMap<String, ObjectRecord>) -> Result<()> {
    let mut current = id;
    let mut visited = BTreeSet::new();
    loop {
        if !visited.insert(current.to_owned()) {
            return invalid(format!("placement cycle contains object {current}"));
        }
        let object = &objects[current];
        match object.placement.relation {
            PlacementRelation::Anchor => return Ok(()),
            PlacementRelation::Container | PlacementRelation::Surface => {
                current = object
                    .placement
                    .parent_object_id
                    .as_deref()
                    .expect("placement shape validated");
                if !objects.contains_key(current) {
                    return invalid(format!("object {id} has unknown parent {current}"));
                }
            }
        }
    }
}

fn validate_placement_shape(id: &str, placement: &PlacementDefinition) -> Result<()> {
    let has_parent = placement.parent_object_id.is_some();
    let has_slot = placement.slot_id.is_some();
    if placement.relation == PlacementRelation::Anchor {
        if has_parent || has_slot {
            return invalid(format!("anchor object {id} cannot declare a parent slot"));
        }
    } else if !has_parent || !has_slot {
        return invalid(format!(
            "placed object {id} requires parent_object_id and slot_id"
        ));
    }
    Ok(())
}

fn validate_actions(
    owner: &str,
    actions: &[ObjectActionDefinition],
) -> Result<Vec<ObjectActionDefinition>> {
    let mut ids = BTreeSet::new();
    for action in actions {
        require_id("action.action_id", &action.action_id)?;
        require_name("action.description", &action.description)?;
        if !matches!(
            action.action_id.as_str(),
            "object.toggle_power"
                | "object.toggle_open"
                | "object.relocate"
                | "object.clean"
                | "object.consume_quantity"
        ) {
            return invalid(format!(
                "{owner} action {} has no registered executor",
                action.action_id
            ));
        }
        if action.duration_ms < 0 || !ids.insert(action.action_id.clone()) {
            return invalid(format!("{owner} has duplicate action or invalid duration"));
        }
        if !matches!(
            action.interruptibility.as_str(),
            "immediate" | "safe_point" | "non_interruptible_until_step" | "background"
        ) {
            return invalid(format!(
                "{owner} action {} has invalid interruptibility",
                action.action_id
            ));
        }
        let mut resources = BTreeSet::new();
        for resource in &action.required_resources {
            if !is_known_resource(resource) || !resources.insert(resource) {
                return invalid(format!(
                    "{owner} action {} has unknown or duplicate resource {resource}",
                    action.action_id
                ));
            }
        }
        for precondition in &action.preconditions {
            require_id("action.precondition.property", &precondition.property)?;
            require_name("action.precondition.equals", &precondition.equals)?;
        }
    }
    Ok(actions.to_vec())
}

fn add_component_actions(
    object_name: &str,
    components: &BTreeSet<String>,
    actions: &mut Vec<ObjectActionDefinition>,
) {
    if components.contains("powerable")
        && !actions
            .iter()
            .any(|action| action.action_id == "object.toggle_power")
    {
        actions.push(ObjectActionDefinition {
            action_id: "object.toggle_power".into(),
            description: format!("Включить или выключить {object_name}"),
            duration_ms: 400,
            required_resources: vec!["hands".into(), "vision".into(), "attention".into()],
            interruptibility: "immediate".into(),
            preconditions: Vec::new(),
        });
    }
    if components.contains("openable")
        && !actions
            .iter()
            .any(|action| action.action_id == "object.toggle_open")
    {
        actions.push(ObjectActionDefinition {
            action_id: "object.toggle_open".into(),
            description: format!("Открыть или закрыть {object_name}"),
            duration_ms: 500,
            required_resources: vec!["hands".into(), "vision".into(), "attention".into()],
            interruptibility: "immediate".into(),
            preconditions: Vec::new(),
        });
    }
    if components.contains("movable")
        && !actions
            .iter()
            .any(|action| action.action_id == "object.relocate")
    {
        actions.push(ObjectActionDefinition {
            action_id: "object.relocate".into(),
            description: format!("Переставить {object_name}"),
            duration_ms: 1_200,
            required_resources: vec!["hands".into(), "vision".into(), "attention".into()],
            interruptibility: "safe_point".into(),
            preconditions: Vec::new(),
        });
    }
    if components.contains("cleanable")
        && !actions
            .iter()
            .any(|action| action.action_id == "object.clean")
    {
        actions.push(ObjectActionDefinition {
            action_id: "object.clean".into(),
            description: format!("Очистить {object_name}"),
            duration_ms: 20_000,
            required_resources: vec!["hands".into(), "vision".into(), "attention".into()],
            interruptibility: "safe_point".into(),
            preconditions: Vec::new(),
        });
    }
    if components.contains("quantity")
        && !actions
            .iter()
            .any(|action| action.action_id == "object.consume_quantity")
    {
        actions.push(ObjectActionDefinition {
            action_id: "object.consume_quantity".into(),
            description: format!("Взять часть из {object_name}"),
            duration_ms: 1_000,
            required_resources: vec!["hands".into(), "vision".into(), "attention".into()],
            interruptibility: "immediate".into(),
            preconditions: Vec::new(),
        });
    }
}

fn empty_parameters_schema() -> String {
    r#"{"additionalProperties":false,"type":"object"}"#.into()
}

fn action_parameters_schema(action_id: &str) -> &'static str {
    match action_id {
        "object.relocate" => {
            r#"{"additionalProperties":false,"properties":{"anchor_id":{"type":"string"},"parent_object_id":{"type":"string"},"relation":{"enum":["anchor","surface","container"],"type":"string"},"slot_id":{"type":"string"}},"required":["anchor_id","relation"],"type":"object"}"#
        }
        "object.consume_quantity" => {
            r#"{"additionalProperties":false,"properties":{"amount":{"maxLength":20,"pattern":"^[1-9][0-9]*$","type":"string"}},"required":["amount"],"type":"object"}"#
        }
        _ => r#"{"additionalProperties":false,"type":"object"}"#,
    }
}

fn validate_initial_state(
    owner: &str,
    state: &InitialObjectStateDefinition,
    components: &BTreeSet<String>,
) -> Result<()> {
    if state.power.is_some() && !components.contains("powerable") {
        return invalid(format!(
            "{owner} declares power state without powerable component"
        ));
    }
    if state.open.is_some() && !components.contains("openable") {
        return invalid(format!(
            "{owner} declares open state without openable component"
        ));
    }
    if state.charge_permille.is_some() && !components.contains("chargeable") {
        return invalid(format!(
            "{owner} declares charge state without chargeable component"
        ));
    }
    if state.charge_permille.is_some_and(|value| value > 1_000) {
        return invalid(format!("{owner} charge_permille exceeds 1000"));
    }
    if state.cleanliness_permille.is_some() && !components.contains("cleanable") {
        return invalid(format!(
            "{owner} declares cleanliness without cleanable component"
        ));
    }
    if state
        .cleanliness_permille
        .is_some_and(|value| value > 1_000)
    {
        return invalid(format!("{owner} cleanliness_permille exceeds 1000"));
    }
    if state.quantity.is_some() && !components.contains("quantity") {
        return invalid(format!(
            "{owner} declares quantity without quantity component"
        ));
    }
    if state.temperature_millicelsius.is_some()
        && !components.contains("heatable")
        && !components.contains("temperature_controlled")
    {
        return invalid(format!(
            "{owner} declares temperature without a thermal component"
        ));
    }
    if state
        .temperature_millicelsius
        .is_some_and(|value| !(-100_000..=500_000).contains(&value))
    {
        return invalid(format!("{owner} temperature is outside physical bounds"));
    }
    Ok(())
}

fn validate_passive_effects(
    owner: &str,
    effects: Vec<PassiveEffect>,
    components: &BTreeSet<String>,
    initial_state: &InitialObjectStateDefinition,
) -> Result<BTreeMap<String, PassiveEffect>> {
    const MAX_RATE: i64 = 1_000_000_000;
    let mut by_id = BTreeMap::new();
    let mut properties = BTreeSet::new();
    for effect in effects {
        require_id("passive_effect.id", effect.id())?;
        if !properties.insert(effect.property()) {
            return invalid(format!(
                "{owner} declares multiple passive effects for {}",
                effect.property()
            ));
        }
        let activation = effect.activation();
        if activation.power.is_some() && !components.contains("powerable") {
            return invalid(format!(
                "{owner} passive effect checks power without powerable component"
            ));
        }
        if activation.open.is_some() && !components.contains("openable") {
            return invalid(format!(
                "{owner} passive effect checks open without openable component"
            ));
        }
        match &effect {
            PassiveEffect::Charge {
                active_delta_per_hour_permille,
                inactive_delta_per_hour_permille,
                ..
            } => {
                if !components.contains("chargeable") || initial_state.charge_permille.is_none() {
                    return invalid(format!(
                        "{owner} passive charge requires chargeable component and initial charge"
                    ));
                }
                if (*active_delta_per_hour_permille == 0 && *inactive_delta_per_hour_permille == 0)
                    || active_delta_per_hour_permille.abs() > MAX_RATE
                    || inactive_delta_per_hour_permille.abs() > MAX_RATE
                {
                    return invalid(format!("{owner} has invalid passive charge rates"));
                }
            }
            PassiveEffect::Temperature {
                active_target_millicelsius,
                active_change_per_hour_millicelsius,
                inactive_target_millicelsius,
                inactive_change_per_hour_millicelsius,
                ..
            } => {
                if (!components.contains("heatable")
                    && !components.contains("temperature_controlled"))
                    || initial_state.temperature_millicelsius.is_none()
                {
                    return invalid(format!(
                        "{owner} passive temperature requires thermal component and initial temperature"
                    ));
                }
                if !(-100_000..=500_000).contains(active_target_millicelsius)
                    || !(-100_000..=500_000).contains(inactive_target_millicelsius)
                    || !(1..=MAX_RATE).contains(active_change_per_hour_millicelsius)
                    || !(1..=MAX_RATE).contains(inactive_change_per_hour_millicelsius)
                {
                    return invalid(format!("{owner} has invalid passive temperature values"));
                }
            }
            PassiveEffect::QuantityConsumption {
                active_amount_per_hour,
                inactive_amount_per_hour,
                ..
            } => {
                if !components.contains("quantity") || initial_state.quantity.is_none() {
                    return invalid(format!(
                        "{owner} passive quantity requires quantity component and initial quantity"
                    ));
                }
                if (*active_amount_per_hour == 0 && *inactive_amount_per_hour == 0)
                    || *active_amount_per_hour > MAX_RATE as u64
                    || *inactive_amount_per_hour > MAX_RATE as u64
                {
                    return invalid(format!("{owner} has invalid passive quantity rates"));
                }
            }
        }
        let id = effect.id().to_owned();
        if by_id.insert(id.clone(), effect).is_some() {
            return invalid(format!("{owner} has duplicate passive effect {id}"));
        }
    }
    Ok(by_id)
}

fn validate_properties(kind: &str, properties: &BTreeMap<String, String>) -> Result<()> {
    for key in properties.keys() {
        require_id(kind, key)?;
    }
    Ok(())
}

fn validate_metadata(metadata: Option<&WorldMetadataDefinition>) -> Result<()> {
    let Some(metadata) = metadata else {
        return Ok(());
    };
    require_name("metadata.city", &metadata.city)?;
    require_name("metadata.timezone", &metadata.timezone)?;
    require_name("metadata.weather_fallback", &metadata.weather_fallback)?;
    if metadata.area_m2 == 0 || metadata.floor < 0 {
        return invalid("metadata area or floor is invalid");
    }
    if let Some(weather) = &metadata.weather {
        if weather.provider != "open_meteo" {
            return invalid("metadata weather provider must be open_meteo");
        }
        if !(-90_000_000..=90_000_000).contains(&weather.latitude_e6)
            || !(-180_000_000..=180_000_000).contains(&weather.longitude_e6)
        {
            return invalid("metadata weather coordinates are invalid");
        }
        if !(60_000..=3_600_000).contains(&weather.poll_interval_ms)
            || !(weather.poll_interval_ms..=21_600_000).contains(&weather.stale_after_ms)
            || !(weather.stale_after_ms..=172_800_000).contains(&weather.fallback_after_ms)
        {
            return invalid("metadata weather intervals must satisfy poll <= stale <= fallback");
        }
    }
    Ok(())
}

fn validate_sensory(sensory: &SensoryDefinition) -> Result<()> {
    for description in sensory
        .light
        .iter()
        .chain(&sensory.sound)
        .chain(&sensory.temperature)
        .chain(&sensory.smell)
    {
        require_name("sensory description", description)?;
    }
    Ok(())
}

fn validate_map_point(anchor: &AnchorDefinition, map: Option<&MapDefinition>) -> Result<()> {
    match (&anchor.map_point, map) {
        (Some(point), Some(map)) if point.x <= map.width && point.y <= map.height => Ok(()),
        (Some(_), Some(_)) => invalid(format!(
            "anchor {} map point is outside the view box",
            anchor.id
        )),
        (None, Some(_)) => invalid(format!("anchor {} has no map point", anchor.id)),
        (Some(_), None) => invalid(format!(
            "anchor {} has a map point but no map asset",
            anchor.id
        )),
        (None, None) => Ok(()),
    }
}

fn validate_map_asset(
    package: &WorldPackage,
    manifest: &Path,
    guard: &PathGuard,
) -> Result<Option<Vec<u8>>> {
    let Some(map) = &package.map else {
        return Ok(None);
    };
    if map.width == 0 || map.height == 0 {
        return invalid("map width and height must be positive");
    }
    let relative = Path::new(&map.asset);
    if relative.is_absolute()
        || relative
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
        || relative.extension().and_then(|value| value.to_str()) != Some("svg")
    {
        return invalid("map asset must be a relative SVG path without traversal");
    }
    let package_root = manifest
        .parent()
        .ok_or_else(|| WorldError::InvalidDefinition("manifest has no package directory".into()))?;
    let asset = guard.validate(package_root.join(relative))?;
    let canonical_root = std::fs::canonicalize(package_root)?;
    let canonical_asset = std::fs::canonicalize(&asset)?;
    if !canonical_asset.starts_with(&canonical_root) || !canonical_asset.is_file() {
        return invalid("map asset escapes its world package");
    }
    let bytes = std::fs::read(canonical_asset)?;
    if bytes.is_empty() || bytes.len() > 1_048_576 {
        return invalid("map asset size is outside 1..=1048576 bytes");
    }
    Ok(Some(bytes))
}

fn normalize_package(package: &mut WorldPackage) {
    package
        .locations
        .sort_by(|left, right| left.id.cmp(&right.id));
    for location in &mut package.locations {
        location
            .anchors
            .sort_by(|left, right| left.id.cmp(&right.id));
    }
    package.connections.sort_by(|left, right| {
        (&left.from, &left.to, left.duration_ms, left.bidirectional).cmp(&(
            &right.from,
            &right.to,
            right.duration_ms,
            right.bidirectional,
        ))
    });
    package
        .object_templates
        .sort_by(|left, right| left.id.cmp(&right.id));
    for template in &mut package.object_templates {
        template.components.sort();
        template
            .passive_effects
            .sort_by(|left, right| left.id().cmp(right.id()));
        template.slots.sort_by(|left, right| left.id.cmp(&right.id));
        template
            .actions
            .sort_by(|left, right| left.action_id.cmp(&right.action_id));
        for action in &mut template.actions {
            action.required_resources.sort();
        }
    }
    package
        .objects
        .sort_by(|left, right| left.id.cmp(&right.id));
    for object in &mut package.objects {
        object.components.sort();
        object
            .passive_effects
            .sort_by(|left, right| left.id().cmp(right.id()));
        object
            .actions
            .sort_by(|left, right| left.action_id.cmp(&right.action_id));
        for action in &mut object.actions {
            action.required_resources.sort();
        }
    }
}

fn shortest_path_duration(
    from: &str,
    to: &str,
    anchors: &BTreeMap<String, String>,
    connections: &BTreeMap<(String, String), i64>,
) -> Option<i64> {
    if !anchors.contains_key(from) || !anchors.contains_key(to) {
        return None;
    }
    if from == to {
        return Some(0);
    }
    let mut distances = BTreeMap::from([(from.to_owned(), 0_i64)]);
    let mut visited = BTreeSet::new();
    loop {
        let (current, distance) = distances
            .iter()
            .filter(|(anchor, _)| !visited.contains(*anchor))
            .min_by_key(|(_, distance)| *distance)
            .map(|(anchor, distance)| (anchor.clone(), *distance))?;
        if current == to {
            return Some(distance);
        }
        visited.insert(current.clone());
        for ((source, target), edge_duration) in connections {
            if source != &current || visited.contains(target) {
                continue;
            }
            let candidate = distance.saturating_add(*edge_duration);
            distances
                .entry(target.clone())
                .and_modify(|known| *known = (*known).min(candidate))
                .or_insert(candidate);
        }
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
        invalid(format!(
            "{kind} must be a non-empty lowercase stable ID: {value:?}"
        ))
    }
}

fn require_name(kind: &str, value: &str) -> Result<()> {
    if value.trim().is_empty() || value.len() > 512 {
        invalid(format!("{kind} must be a non-empty bounded string"))
    } else {
        Ok(())
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
        return invalid(format!("duplicate connection {from} -> {to}"));
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
        return invalid(format!(
            "anchors are not reachable from {start}: {}",
            missing.join(", ")
        ));
    }
    Ok(())
}

fn is_known_resource(value: &str) -> bool {
    matches!(
        value,
        "movement" | "hands" | "vision" | "attention" | "speech" | "hearing" | "background"
    )
}

fn is_known_component(value: &str) -> bool {
    matches!(
        value,
        "powerable"
            | "chargeable"
            | "openable"
            | "lockable"
            | "container"
            | "surface"
            | "fillable"
            | "heatable"
            | "cleanable"
            | "sound_emitter"
            | "provides_interaction_anchor"
            | "quantity"
            | "temperature_controlled"
            | "movable"
    )
}

fn invalid<T>(message: impl Into<String>) -> Result<T> {
    Err(WorldError::InvalidDefinition(message.into()))
}

fn hex_digest(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}
