use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::{
    Affordance, ObjectCondition, ObjectPlacement, PlacementRelation, Result, WorldDefinition,
    WorldError,
};

pub const COMMAND_SCHEMA_VERSION: u32 = 1;
pub const EVENT_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TimeStatus {
    Normal,
    TimeAnomaly,
    SafeStop,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Resource {
    Movement,
    Hands,
    Vision,
    Attention,
    Speech,
    Hearing,
    Background,
}

impl Resource {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "movement" => Some(Self::Movement),
            "hands" => Some(Self::Hands),
            "vision" => Some(Self::Vision),
            "attention" => Some(Self::Attention),
            "speech" => Some(Self::Speech),
            "hearing" => Some(Self::Hearing),
            "background" => Some(Self::Background),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Movement => "movement",
            Self::Hands => "hands",
            Self::Vision => "vision",
            Self::Attention => "attention",
            Self::Speech => "speech",
            Self::Hearing => "hearing",
            Self::Background => "background",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Activity {
    pub activity_id: String,
    pub action_id: String,
    pub target_id: String,
    pub started_at_ms: i64,
    pub completes_at_ms: i64,
    pub resources: BTreeSet<Resource>,
    pub completion: ActivityCompletion,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ActivityCompletion {
    MoveTo {
        anchor_id: String,
    },
    SetObjectPower {
        object_id: String,
        powered: bool,
    },
    SetObjectOpen {
        object_id: String,
        open: bool,
    },
    SetObjectPlacement {
        object_id: String,
        placement: ObjectPlacement,
    },
    SetObjectCondition {
        object_id: String,
        condition: ObjectCondition,
    },
    SetObjectCleanliness {
        object_id: String,
        base_condition: ObjectCondition,
        cleanliness_permille: u16,
    },
    ConsumeObjectQuantity {
        object_id: String,
        base_condition: ObjectCondition,
        amount: u64,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct PassiveConditionUpdate {
    pub object_id: String,
    pub condition: ObjectCondition,
}

fn is_zero_i64(value: &i64) -> bool {
    *value == 0
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldState {
    identity_id: String,
    world_definition_hash: String,
    world_version: u64,
    last_event_seq: u64,
    last_utc_ms: i64,
    time_status: TimeStatus,
    agent_anchor_id: String,
    activities: BTreeMap<String, Activity>,
    reserved_resources: BTreeMap<Resource, String>,
    object_power: BTreeMap<String, bool>,
    #[serde(default)]
    object_open: BTreeMap<String, bool>,
    #[serde(default)]
    object_placements: BTreeMap<String, ObjectPlacement>,
    #[serde(default)]
    object_conditions: BTreeMap<String, ObjectCondition>,
    #[serde(default, skip_serializing_if = "is_zero_i64")]
    passive_updated_at_ms: i64,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    passive_remainders: BTreeMap<String, i64>,
}

impl WorldState {
    pub(crate) fn empty(identity_id: String, definition_hash: String) -> Self {
        Self {
            identity_id,
            world_definition_hash: definition_hash,
            world_version: 0,
            last_event_seq: 0,
            last_utc_ms: 0,
            time_status: TimeStatus::Normal,
            agent_anchor_id: String::new(),
            activities: BTreeMap::new(),
            reserved_resources: BTreeMap::new(),
            object_power: BTreeMap::new(),
            object_open: BTreeMap::new(),
            object_placements: BTreeMap::new(),
            object_conditions: BTreeMap::new(),
            passive_updated_at_ms: 0,
            passive_remainders: BTreeMap::new(),
        }
    }

    pub fn identity_id(&self) -> &str {
        &self.identity_id
    }

    pub fn world_definition_hash(&self) -> &str {
        &self.world_definition_hash
    }

    pub fn world_version(&self) -> u64 {
        self.world_version
    }

    pub fn last_event_seq(&self) -> u64 {
        self.last_event_seq
    }

    pub fn last_utc_ms(&self) -> i64 {
        self.last_utc_ms
    }

    pub fn time_status(&self) -> &TimeStatus {
        &self.time_status
    }

    pub fn agent_anchor_id(&self) -> &str {
        &self.agent_anchor_id
    }

    pub fn activities(&self) -> impl Iterator<Item = &Activity> {
        self.activities.values()
    }

    pub(crate) fn passive_updated_at_ms(&self) -> i64 {
        if self.passive_updated_at_ms == 0 {
            self.last_utc_ms
        } else {
            self.passive_updated_at_ms
        }
    }

    pub(crate) fn passive_remainder(&self, key: &str) -> i64 {
        self.passive_remainders
            .get(key)
            .copied()
            .unwrap_or_default()
    }

    pub(crate) fn passive_remainders(&self) -> &BTreeMap<String, i64> {
        &self.passive_remainders
    }

    pub fn state_hash(&self) -> Result<String> {
        use sha2::{Digest, Sha256};
        let bytes = serde_json::to_vec(self)?;
        let digest = Sha256::digest(bytes);
        Ok(digest.iter().map(|byte| format!("{byte:02x}")).collect())
    }

    pub(crate) fn apply(&mut self, envelope: &PersistedEvent) -> Result<()> {
        let expected_seq = self.last_event_seq + 1;
        if envelope.event_seq != expected_seq {
            return Err(WorldError::EventSequenceGap {
                expected: expected_seq,
                actual: envelope.event_seq,
            });
        }
        let expected_version = self.world_version + 1;
        if envelope.world_version != expected_version {
            return Err(WorldError::WorldVersionGap {
                expected: expected_version,
                actual: envelope.world_version,
            });
        }

        match &envelope.payload {
            DomainEvent::AgentAwakened { anchor_id } => {
                if !self.agent_anchor_id.is_empty() {
                    return Err(WorldError::StateInvariant("agent awakened twice".into()));
                }
                self.agent_anchor_id = anchor_id.clone();
            }
            DomainEvent::ActivityScheduled { activity } => {
                if self.activities.contains_key(&activity.activity_id) {
                    return Err(WorldError::StateInvariant(format!(
                        "duplicate activity {}",
                        activity.activity_id
                    )));
                }
                for resource in &activity.resources {
                    if self.reserved_resources.contains_key(resource) {
                        return Err(WorldError::StateInvariant(format!(
                            "resource {} is already reserved",
                            resource.as_str()
                        )));
                    }
                    self.reserved_resources
                        .insert(resource.clone(), activity.activity_id.clone());
                }
                self.activities
                    .insert(activity.activity_id.clone(), activity.clone());
            }
            DomainEvent::ActivityCompleted { activity_id } => {
                let activity = self.activities.remove(activity_id).ok_or_else(|| {
                    WorldError::StateInvariant(format!("unknown activity {activity_id}"))
                })?;
                for resource in &activity.resources {
                    self.reserved_resources.remove(resource);
                }
                match activity.completion {
                    ActivityCompletion::MoveTo { anchor_id } => {
                        self.agent_anchor_id = anchor_id;
                    }
                    ActivityCompletion::SetObjectPower { object_id, powered } => {
                        self.object_power.insert(object_id, powered);
                    }
                    ActivityCompletion::SetObjectOpen { object_id, open } => {
                        self.object_open.insert(object_id, open);
                    }
                    ActivityCompletion::SetObjectPlacement {
                        object_id,
                        placement,
                    } => {
                        self.object_placements.insert(object_id, placement);
                    }
                    ActivityCompletion::SetObjectCondition {
                        object_id,
                        condition,
                    } => {
                        self.object_conditions.insert(object_id, condition);
                    }
                    ActivityCompletion::SetObjectCleanliness {
                        object_id,
                        base_condition,
                        cleanliness_permille,
                    } => {
                        self.object_conditions
                            .entry(object_id)
                            .or_insert(base_condition)
                            .cleanliness_permille = Some(cleanliness_permille);
                    }
                    ActivityCompletion::ConsumeObjectQuantity {
                        object_id,
                        base_condition,
                        amount,
                    } => {
                        let condition = self
                            .object_conditions
                            .entry(object_id)
                            .or_insert(base_condition);
                        let quantity = condition.quantity.as_mut().ok_or_else(|| {
                            WorldError::StateInvariant(
                                "quantity disappeared before activity completion".into(),
                            )
                        })?;
                        quantity.amount = quantity.amount.saturating_sub(amount);
                    }
                }
            }
            DomainEvent::PassiveConditionsAdvanced {
                from_utc_ms,
                to_utc_ms,
                updates,
                remainders,
            } => {
                if *from_utc_ms != self.passive_updated_at_ms() {
                    return Err(WorldError::StateInvariant(format!(
                        "passive clock mismatch: expected {}, got {from_utc_ms}",
                        self.passive_updated_at_ms()
                    )));
                }
                if to_utc_ms < from_utc_ms || *to_utc_ms != envelope.occurred_at_ms {
                    return Err(WorldError::StateInvariant(
                        "invalid passive evolution interval".into(),
                    ));
                }
                let mut seen = BTreeSet::new();
                for update in updates {
                    if !seen.insert(update.object_id.as_str()) {
                        return Err(WorldError::StateInvariant(format!(
                            "duplicate passive update for {}",
                            update.object_id
                        )));
                    }
                    self.object_conditions
                        .insert(update.object_id.clone(), update.condition.clone());
                }
                self.passive_remainders = remainders.clone();
                self.passive_updated_at_ms = *to_utc_ms;
            }
            DomainEvent::DowntimeObserved { .. } => {}
            DomainEvent::TimeAnomalyDetected { .. } => {
                self.time_status = TimeStatus::TimeAnomaly;
            }
        }

        self.last_event_seq = envelope.event_seq;
        self.world_version = envelope.world_version;
        self.last_utc_ms = envelope.occurred_at_ms;
        Ok(())
    }

    pub(crate) fn resource_owner(&self, resource: &Resource) -> Option<&str> {
        self.reserved_resources.get(resource).map(String::as_str)
    }

    pub(crate) fn due_activities(&self, now_ms: i64) -> Vec<Activity> {
        self.activities
            .values()
            .filter(|activity| activity.completes_at_ms <= now_ms)
            .cloned()
            .collect()
    }

    pub(crate) fn object_power(&self, object_id: &str, definition: &WorldDefinition) -> bool {
        self.object_power
            .get(object_id)
            .copied()
            .unwrap_or_else(|| definition.initial_object_power(object_id))
    }

    pub(crate) fn object_open(&self, object_id: &str, definition: &WorldDefinition) -> bool {
        self.object_open
            .get(object_id)
            .copied()
            .unwrap_or_else(|| definition.initial_object_open(object_id))
    }

    pub(crate) fn object_placement(
        &self,
        object_id: &str,
        definition: &WorldDefinition,
    ) -> Option<ObjectPlacement> {
        self.object_placements
            .get(object_id)
            .cloned()
            .or_else(|| definition.initial_object_placement(object_id))
    }

    pub(crate) fn object_condition(
        &self,
        object_id: &str,
        definition: &WorldDefinition,
    ) -> ObjectCondition {
        self.object_conditions
            .get(object_id)
            .cloned()
            .unwrap_or_else(|| definition.initial_object_condition(object_id))
    }

    pub(crate) fn object_placements(
        &self,
        definition: &WorldDefinition,
    ) -> BTreeMap<String, ObjectPlacement> {
        definition
            .initial_object_placements()
            .map(|(id, placement)| {
                let effective = self
                    .object_placements
                    .get(&id)
                    .cloned()
                    .unwrap_or(placement);
                (id, effective)
            })
            .collect()
    }

    pub(crate) fn object_is_at_anchor(
        &self,
        object_id: &str,
        anchor_id: &str,
        definition: &WorldDefinition,
    ) -> bool {
        self.object_placement(object_id, definition)
            .is_some_and(|placement| placement.anchor_id == anchor_id)
    }

    pub(crate) fn object_is_visible(&self, object_id: &str, definition: &WorldDefinition) -> bool {
        let mut current = object_id.to_owned();
        let mut visited = BTreeSet::new();
        loop {
            if !visited.insert(current.clone()) {
                return false;
            }
            let Some(placement) = self.object_placement(&current, definition) else {
                return false;
            };
            match placement.relation {
                PlacementRelation::Anchor => return true,
                PlacementRelation::Surface => {
                    let Some(parent) = placement.parent_object_id else {
                        return false;
                    };
                    current = parent;
                }
                PlacementRelation::Container => {
                    let Some(parent) = placement.parent_object_id else {
                        return false;
                    };
                    if definition.object_has_component(&parent, "openable")
                        && !self.object_open(&parent, definition)
                    {
                        return false;
                    }
                    current = parent;
                }
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandEnvelope {
    pub command_id: String,
    pub identity_id: String,
    pub agent_id: String,
    pub expected_world_version: u64,
    pub schema_version: u32,
    pub decision_id: String,
    pub issued_at_ms: i64,
    pub ttl_ms: i64,
    pub payload: CommandPayload,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CommandPayload {
    MoveTo {
        target_anchor_id: String,
    },
    Perform {
        action_id: String,
        target_id: String,
        parameters: BTreeMap<String, String>,
    },
    Inspect {
        target_id: String,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommandStatus {
    Committed,
    AlreadyCommitted,
    RejectedPrecondition,
    ResourceConflict,
    StaleWorld,
    ExpiredDecision,
    Unauthorized,
    InvalidArgument,
    TemporarilyUnavailable,
    InternalError,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandResult {
    pub command_id: String,
    pub status: CommandStatus,
    pub committed_world_version: u64,
    pub first_event_seq: u64,
    pub last_event_seq: u64,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub suggested_recovery: Vec<Affordance>,
}

impl CommandResult {
    pub(crate) fn rejected(
        command_id: &str,
        status: CommandStatus,
        state: &WorldState,
        code: &str,
        message: impl Into<String>,
    ) -> Self {
        Self {
            command_id: command_id.into(),
            status,
            committed_world_version: state.world_version,
            first_event_seq: 0,
            last_event_seq: 0,
            error_code: Some(code.into()),
            error_message: Some(message.into()),
            suggested_recovery: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersistedEvent {
    pub event_id: String,
    pub event_seq: u64,
    pub world_version: u64,
    pub event_schema_version: u32,
    pub occurred_at_ms: i64,
    pub causation_command_id: Option<String>,
    pub(crate) payload: DomainEvent,
}

impl PersistedEvent {
    pub fn event_type(&self) -> &'static str {
        self.payload.event_type()
    }

    pub fn payload_json(&self) -> Result<Vec<u8>> {
        Ok(serde_json::to_vec(&self.payload)?)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub(crate) enum DomainEvent {
    AgentAwakened {
        anchor_id: String,
    },
    ActivityScheduled {
        activity: Activity,
    },
    ActivityCompleted {
        activity_id: String,
    },
    PassiveConditionsAdvanced {
        from_utc_ms: i64,
        to_utc_ms: i64,
        updates: Vec<PassiveConditionUpdate>,
        remainders: BTreeMap<String, i64>,
    },
    DowntimeObserved {
        from_utc_ms: i64,
        to_utc_ms: i64,
    },
    TimeAnomalyDetected {
        previous_utc_ms: i64,
        observed_utc_ms: i64,
        monotonic_elapsed_ms: i64,
    },
}

impl DomainEvent {
    pub(crate) fn event_type(&self) -> &'static str {
        match self {
            Self::AgentAwakened { .. } => "agent_awakened",
            Self::ActivityScheduled { .. } => "activity_scheduled",
            Self::ActivityCompleted { .. } => "activity_completed",
            Self::PassiveConditionsAdvanced { .. } => "passive_conditions_advanced",
            Self::DowntimeObserved { .. } => "downtime_observed",
            Self::TimeAnomalyDetected { .. } => "time_anomaly_detected",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ClockSample {
    pub utc_ms: i64,
    pub monotonic_elapsed_ms: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObservedObject {
    pub object_id: String,
    pub name: String,
    pub observed_properties: BTreeMap<String, String>,
    pub affordances: Vec<Affordance>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActivityView {
    pub activity_id: String,
    pub action_id: String,
    pub completes_at_ms: i64,
    pub reserved_resources: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PerceptionWindow {
    pub perception_id: String,
    pub world_version: u64,
    pub location_id: String,
    pub location_name: String,
    pub anchor_id: String,
    pub environment_cues: Vec<String>,
    pub observed_objects: Vec<ObservedObject>,
    pub affordances: Vec<Affordance>,
    pub activities: Vec<ActivityView>,
    pub significant_changes: Vec<String>,
}

impl PerceptionWindow {
    pub(crate) fn build(state: &WorldState, definition: &WorldDefinition) -> Result<Self> {
        let (location_id, location_name) = definition
            .location_for_anchor(state.agent_anchor_id())
            .ok_or_else(|| WorldError::StateInvariant("agent is at an unknown anchor".into()))?;
        let mut observed_objects = definition
            .observed_objects()
            .filter(|object| {
                state.object_is_at_anchor(&object.id, state.agent_anchor_id(), definition)
                    && state.object_is_visible(&object.id, definition)
            })
            .map(|object| {
                let mut observed_properties = object.observed_properties.clone();
                let mut affordances = object.actions.clone();
                if definition.object_has_component(&object.id, "powerable")
                    || affordances
                        .iter()
                        .any(|action| action.action_id == "object.toggle_power")
                {
                    let powered = state.object_power(&object.id, definition);
                    observed_properties
                        .insert("power".into(), if powered { "on" } else { "off" }.into());
                    for action in &mut affordances {
                        if action.action_id == "object.toggle_power" {
                            action.description = if powered {
                                "Выключить"
                            } else {
                                "Включить"
                            }
                            .into();
                        }
                    }
                }
                if definition.object_has_component(&object.id, "openable") {
                    let open = state.object_open(&object.id, definition);
                    observed_properties
                        .insert("open".into(), if open { "open" } else { "closed" }.into());
                    for action in &mut affordances {
                        if action.action_id == "object.toggle_open" {
                            action.description = if open {
                                "Закрыть"
                            } else {
                                "Открыть"
                            }
                            .into();
                        }
                    }
                }
                let condition = state.object_condition(&object.id, definition);
                if let Some(value) = condition.charge_permille {
                    observed_properties.insert("charge_permille".into(), value.to_string());
                }
                if let Some(value) = condition.cleanliness_permille {
                    observed_properties.insert("cleanliness_permille".into(), value.to_string());
                    if value >= 1_000 {
                        affordances.retain(|action| action.action_id != "object.clean");
                    }
                }
                if let Some(quantity) = condition.quantity {
                    observed_properties
                        .insert("quantity_amount".into(), quantity.amount.to_string());
                    observed_properties
                        .insert("quantity_unit".into(), quantity.unit.as_str().into());
                    if quantity.amount == 0 {
                        affordances.retain(|action| action.action_id != "object.consume_quantity");
                    }
                } else {
                    affordances.retain(|action| action.action_id != "object.consume_quantity");
                }
                if let Some(value) = condition.temperature_millicelsius {
                    observed_properties
                        .insert("temperature_millicelsius".into(), value.to_string());
                }
                let placement = state.object_placement(&object.id, definition);
                if definition.object_has_component(&object.id, "chargeable") {
                    observed_properties.insert(
                        "receiving_power".into(),
                        definition
                            .object_receives_placement_power(placement.as_ref())
                            .to_string(),
                    );
                }
                if let Some(placement) = placement {
                    observed_properties.insert("anchor_id".into(), placement.anchor_id);
                    observed_properties.insert(
                        "placement_relation".into(),
                        match placement.relation {
                            PlacementRelation::Anchor => "anchor",
                            PlacementRelation::Surface => "surface",
                            PlacementRelation::Container => "container",
                        }
                        .into(),
                    );
                    if let Some(parent) = placement.parent_object_id {
                        observed_properties.insert("parent_object_id".into(), parent);
                    }
                    if let Some(slot) = placement.slot_id {
                        observed_properties.insert("slot_id".into(), slot);
                    }
                }
                ObservedObject {
                    object_id: object.id.clone(),
                    name: object.name.clone(),
                    observed_properties,
                    affordances,
                }
            })
            .collect::<Vec<_>>();
        observed_objects.sort_by(|left, right| left.object_id.cmp(&right.object_id));
        let mut environment_cues = definition.sensory_cues(state.agent_anchor_id()).to_vec();
        for object in &observed_objects {
            let powered = object
                .observed_properties
                .get("power")
                .is_some_and(|value| value == "on");
            let open = object
                .observed_properties
                .get("open")
                .is_some_and(|value| value == "open");
            if powered && definition.object_has_template(&object.object_id, "template.light") {
                environment_cues.push(format!("Свет: {} включён.", object.name));
            }
            if powered && definition.object_has_component(&object.object_id, "sound_emitter") {
                environment_cues.push(format!("Звук: слышно, как работает {}.", object.name));
            }
            if open && definition.object_has_template(&object.object_id, "template.window") {
                environment_cues.push("Звук: открытое окно пропускает звуки снаружи.".into());
                environment_cues
                    .push("Температура: через открытое окно поступает наружный воздух.".into());
            }
            if open && definition.object_has_template(&object.object_id, "template.refrigerator") {
                environment_cues
                    .push("Температура: из открытого холодильника идёт холодный воздух.".into());
            }
        }
        let activities = state
            .activities()
            .map(|activity| ActivityView {
                activity_id: activity.activity_id.clone(),
                action_id: activity.action_id.clone(),
                completes_at_ms: activity.completes_at_ms,
                reserved_resources: activity
                    .resources
                    .iter()
                    .map(|resource| resource.as_str().to_owned())
                    .collect(),
            })
            .collect();

        Ok(Self {
            perception_id: format!("perception-{}", state.world_version()),
            world_version: state.world_version(),
            location_id: location_id.into(),
            location_name: location_name.into(),
            anchor_id: state.agent_anchor_id().into(),
            environment_cues,
            observed_objects,
            affordances: definition.movement_affordances(state.agent_anchor_id()),
            activities,
            significant_changes: Vec::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn passive_defaults_preserve_legacy_state_shape() {
        let mut state = WorldState::empty("test-makise".into(), "definition".into());
        let event = PersistedEvent {
            event_id: "event-1".into(),
            event_seq: 1,
            world_version: 1,
            event_schema_version: 1,
            occurred_at_ms: 1_000_000,
            causation_command_id: None,
            payload: DomainEvent::AgentAwakened {
                anchor_id: "bed".into(),
            },
        };
        state.apply(&event).unwrap();

        let serialized = serde_json::to_value(&state).unwrap();
        assert!(serialized.get("passive_updated_at_ms").is_none());
        assert!(serialized.get("passive_remainders").is_none());
        assert_eq!(state.passive_updated_at_ms(), 1_000_000);
    }
}
