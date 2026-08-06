use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use crate::WorldDefinition;
use crate::definition_stage4::PassiveEffect;
use crate::domain::{
    Activity, ActivityCompletion, COMMAND_SCHEMA_VERSION, ClockSample, CommandEnvelope,
    CommandPayload, CommandResult, CommandStatus, DomainEvent, PassiveConditionUpdate,
    PerceptionWindow, PersistedEvent, Resource, TimeStatus, WeatherObservation, WorldState,
};
use crate::store::{EventStore, digest};
use crate::{ObjectPlacement, PathGuard, PlacementRelation, Result, WorldError};

const PASSIVE_EVENT_INTERVAL_MS: i64 = 60_000;
const MILLIS_PER_HOUR: i128 = 3_600_000;
const CLOCK_CHECKPOINT_INTERVAL_MS: i64 = 5_000;
const MIN_RECORDED_DOWNTIME_MS: i64 = 1_000;
const MAX_CLOCK_DRIFT_MS: i64 = 60_000;
const MAX_COMMAND_TTL_MS: i64 = 300_000;
const PRIMARY_AGENT_ID: &str = "makise";

pub struct WorldEngine {
    definition: WorldDefinition,
    state: WorldState,
    store: EventStore,
    last_clock_utc_ms: i64,
    last_clock_checkpoint_utc_ms: i64,
}

impl WorldEngine {
    pub fn open(
        database_path: impl AsRef<Path>,
        identity_id: &str,
        definition: WorldDefinition,
        initial_anchor_id: &str,
        now_ms: i64,
        guard: &PathGuard,
    ) -> Result<Self> {
        validate_external_id("identity_id", identity_id)?;
        if !definition.contains_anchor(initial_anchor_id) {
            return Err(WorldError::InvalidDefinition(format!(
                "initial anchor {initial_anchor_id} does not exist"
            )));
        }
        let mut store = EventStore::open(database_path, identity_id, definition.hash(), guard)?;
        let mut state = store.load_state(identity_id, definition.hash())?;
        if state.last_event_seq() == 0 {
            let mut events = vec![DomainEvent::AgentAwakened {
                anchor_id: initial_anchor_id.into(),
            }];
            if definition.passive_objects().next().is_some() {
                events.push(DomainEvent::PassiveConditionsAdvanced {
                    from_utc_ms: now_ms,
                    to_utc_ms: now_ms,
                    updates: Vec::new(),
                    remainders: BTreeMap::new(),
                });
            }
            state = store.commit_system_events(&state, now_ms, events)?;
            store.force_snapshot(&state, now_ms)?;
        }
        let last_clock_utc_ms = store.clock_checkpoint_utc_ms()?.max(state.last_utc_ms());
        Ok(Self {
            definition,
            state,
            store,
            last_clock_utc_ms,
            last_clock_checkpoint_utc_ms: last_clock_utc_ms,
        })
    }

    pub fn state(&self) -> &WorldState {
        &self.state
    }

    pub fn definition(&self) -> &WorldDefinition {
        &self.definition
    }

    pub fn database_path(&self) -> &Path {
        self.store.database_path()
    }

    pub fn perception(&self) -> Result<PerceptionWindow> {
        PerceptionWindow::build(&self.state, &self.definition)
    }

    pub fn command_result(&self, command_id: &str) -> Result<Option<CommandResult>> {
        Ok(self
            .store
            .find_command(command_id)?
            .map(|(_, result)| result))
    }

    pub fn events_after(&self, after_seq: u64) -> Result<Vec<PersistedEvent>> {
        self.store.events_after(after_seq)
    }

    pub fn observe_weather(
        &mut self,
        observation: WeatherObservation,
        received_at_ms: i64,
    ) -> Result<bool> {
        observation.validate(received_at_ms)?;
        if self
            .state
            .weather_observation()
            .is_some_and(|current| current.observed_at_ms >= observation.observed_at_ms)
        {
            return Ok(false);
        }
        self.state = self.store.commit_system_events(
            &self.state,
            received_at_ms,
            vec![DomainEvent::WeatherObserved { observation }],
        )?;
        Ok(true)
    }

    pub fn execute_command(
        &mut self,
        command: &CommandEnvelope,
        now_ms: i64,
    ) -> Result<CommandResult> {
        let request_json = serde_json::to_vec(command)?;
        let request_hash = digest(&request_json);

        if let Some((stored_hash, mut stored_result)) =
            self.store.find_command(&command.command_id)?
        {
            if stored_hash != request_hash {
                return Ok(CommandResult::rejected(
                    &command.command_id,
                    CommandStatus::InvalidArgument,
                    &self.state,
                    "COMMAND_ID_COLLISION",
                    "command_id was already used with a different payload",
                ));
            }
            if stored_result.status == CommandStatus::Committed {
                stored_result.status = CommandStatus::AlreadyCommitted;
            }
            return Ok(stored_result);
        }

        if let Some(result) = self.validate_envelope(command, now_ms) {
            self.store
                .record_rejection(&request_hash, &result, now_ms)?;
            return Ok(result);
        }

        let events = match self.build_command_events(command, now_ms) {
            Ok(events) => events,
            Err(result) => {
                let result = *result;
                self.store
                    .record_rejection(&request_hash, &result, now_ms)?;
                return Ok(result);
            }
        };
        let (state, result) = self.store.commit_command(
            &self.state,
            &command.command_id,
            &request_hash,
            now_ms,
            events,
        )?;
        self.state = state;
        Ok(result)
    }

    pub fn tick(&mut self, sample: ClockSample) -> Result<()> {
        if self.state.time_status() != &TimeStatus::Normal {
            return Ok(());
        }
        if sample.monotonic_elapsed_ms < 0 {
            return Err(WorldError::StateInvariant(
                "monotonic elapsed time cannot be negative".into(),
            ));
        }
        let expected_utc = self
            .last_clock_utc_ms
            .saturating_add(sample.monotonic_elapsed_ms);
        let drift = sample.utc_ms.saturating_sub(expected_utc).abs();
        if drift > MAX_CLOCK_DRIFT_MS {
            self.state = self.store.commit_system_events(
                &self.state,
                sample.utc_ms,
                vec![DomainEvent::TimeAnomalyDetected {
                    previous_utc_ms: self.last_clock_utc_ms,
                    observed_utc_ms: sample.utc_ms,
                    monotonic_elapsed_ms: sample.monotonic_elapsed_ms,
                }],
            )?;
            self.last_clock_utc_ms = sample.utc_ms;
            return Ok(());
        }

        self.complete_due_activities(sample.utc_ms, false)?;
        self.last_clock_utc_ms = sample.utc_ms;
        if sample
            .utc_ms
            .saturating_sub(self.last_clock_checkpoint_utc_ms)
            >= CLOCK_CHECKPOINT_INTERVAL_MS
        {
            self.checkpoint_clock(sample.utc_ms)?;
        }
        Ok(())
    }

    pub fn resume_after_downtime(&mut self, now_ms: i64) -> Result<()> {
        if self.state.time_status() != &TimeStatus::Normal {
            return Ok(());
        }
        let previous = self.last_clock_utc_ms;
        if now_ms < previous {
            self.state = self.store.commit_system_events(
                &self.state,
                now_ms,
                vec![DomainEvent::TimeAnomalyDetected {
                    previous_utc_ms: previous,
                    observed_utc_ms: now_ms,
                    monotonic_elapsed_ms: 0,
                }],
            )?;
        } else {
            let downtime_ms = now_ms.saturating_sub(previous);
            let record_downtime = downtime_ms >= MIN_RECORDED_DOWNTIME_MS;
            self.complete_due_activities(now_ms, record_downtime)?;
            if record_downtime {
                self.state = self.store.commit_system_events(
                    &self.state,
                    now_ms,
                    vec![DomainEvent::DowntimeObserved {
                        from_utc_ms: previous,
                        to_utc_ms: now_ms,
                    }],
                )?;
            }
        }
        self.last_clock_utc_ms = now_ms;
        self.checkpoint_clock(now_ms)
    }

    pub fn checkpoint_clock(&mut self, now_ms: i64) -> Result<()> {
        self.store.record_clock_checkpoint(now_ms)?;
        self.last_clock_checkpoint_utc_ms = now_ms;
        Ok(())
    }

    pub fn snapshot(&mut self, now_ms: i64) -> Result<()> {
        self.store.force_snapshot(&self.state, now_ms)?;
        self.last_clock_checkpoint_utc_ms = now_ms;
        Ok(())
    }

    fn validate_envelope(&self, command: &CommandEnvelope, now_ms: i64) -> Option<CommandResult> {
        let reject = |status, code, message: String| {
            CommandResult::rejected(&command.command_id, status, &self.state, code, message)
        };
        if validate_external_id("command_id", &command.command_id).is_err()
            || validate_external_id("decision_id", &command.decision_id).is_err()
        {
            return Some(reject(
                CommandStatus::InvalidArgument,
                "INVALID_ID",
                "command_id and decision_id must be stable non-empty IDs".into(),
            ));
        }
        if command.identity_id != self.state.identity_id() || command.agent_id != PRIMARY_AGENT_ID {
            return Some(reject(
                CommandStatus::Unauthorized,
                "IDENTITY_MISMATCH",
                "command identity or agent does not match this world".into(),
            ));
        }
        if command.schema_version != COMMAND_SCHEMA_VERSION {
            return Some(reject(
                CommandStatus::InvalidArgument,
                "UNSUPPORTED_SCHEMA",
                format!("expected command schema version {COMMAND_SCHEMA_VERSION}"),
            ));
        }
        if command.ttl_ms <= 0 || command.ttl_ms > MAX_COMMAND_TTL_MS {
            return Some(reject(
                CommandStatus::InvalidArgument,
                "INVALID_TTL",
                format!("ttl_ms must be within 1..={MAX_COMMAND_TTL_MS}"),
            ));
        }
        if command.issued_at_ms.saturating_add(command.ttl_ms) < now_ms {
            return Some(reject(
                CommandStatus::ExpiredDecision,
                "EXPIRED_DECISION",
                "command TTL expired before execution".into(),
            ));
        }
        if command.expected_world_version != self.state.world_version() {
            return Some(reject(
                CommandStatus::StaleWorld,
                "STALE_WORLD",
                format!(
                    "expected world version {}, current version is {}",
                    command.expected_world_version,
                    self.state.world_version()
                ),
            ));
        }
        if self.state.time_status() != &TimeStatus::Normal {
            return Some(reject(
                CommandStatus::TemporarilyUnavailable,
                "WORLD_NOT_READY",
                format!("world time status is {:?}", self.state.time_status()),
            ));
        }
        None
    }

    fn build_command_events(
        &self,
        command: &CommandEnvelope,
        now_ms: i64,
    ) -> std::result::Result<Vec<DomainEvent>, Box<CommandResult>> {
        match &command.payload {
            CommandPayload::MoveTo { target_anchor_id } => {
                if !self.definition.contains_anchor(target_anchor_id) {
                    return Err(Box::new(self.command_error(
                        command,
                        CommandStatus::RejectedPrecondition,
                        "UNKNOWN_ANCHOR",
                        format!("anchor {target_anchor_id} does not exist"),
                    )));
                }
                if target_anchor_id == self.state.agent_anchor_id() {
                    return Err(Box::new(self.command_error(
                        command,
                        CommandStatus::RejectedPrecondition,
                        "ALREADY_THERE",
                        "agent is already at the requested anchor",
                    )));
                }
                let Some(duration_ms) = self
                    .definition
                    .movement_duration(self.state.agent_anchor_id(), target_anchor_id)
                else {
                    return Err(Box::new(self.command_error(
                        command,
                        CommandStatus::RejectedPrecondition,
                        "NO_PATH",
                        "target anchor is not reachable",
                    )));
                };
                self.ensure_resources(command, [Resource::Movement])?;
                Ok(vec![DomainEvent::ActivityScheduled {
                    activity: Activity {
                        activity_id: format!("activity-{}", command.command_id),
                        action_id: "world.move_to".into(),
                        target_id: target_anchor_id.clone(),
                        started_at_ms: now_ms,
                        completes_at_ms: now_ms.saturating_add(duration_ms),
                        resources: BTreeSet::from([Resource::Movement]),
                        completion: ActivityCompletion::MoveTo {
                            anchor_id: target_anchor_id.clone(),
                        },
                    },
                }])
            }
            CommandPayload::Perform {
                action_id,
                target_id,
                parameters,
            } => {
                let Some(affordance) = self.definition.object_action(target_id, action_id) else {
                    return Err(Box::new(self.command_error(
                        command,
                        CommandStatus::RejectedPrecondition,
                        "UNKNOWN_AFFORDANCE",
                        "object does not provide the requested action",
                    )));
                };
                let object_is_here = self.state.object_is_at_anchor(
                    target_id,
                    self.state.agent_anchor_id(),
                    &self.definition,
                ) && self.state.object_is_visible(target_id, &self.definition);
                if !object_is_here {
                    return Err(Box::new(self.command_error(
                        command,
                        CommandStatus::RejectedPrecondition,
                        "TARGET_NOT_REACHABLE",
                        "object is not available at the current anchor",
                    )));
                }
                let accepts_parameters = matches!(
                    action_id.as_str(),
                    "object.relocate" | "object.consume_quantity"
                );
                if !accepts_parameters && !parameters.is_empty() {
                    return Err(Box::new(self.command_error(
                        command,
                        CommandStatus::InvalidArgument,
                        "UNEXPECTED_PARAMETERS",
                        "this action does not accept parameters",
                    )));
                }
                let completion = match action_id.as_str() {
                    "object.toggle_power" => ActivityCompletion::SetObjectPower {
                        object_id: target_id.clone(),
                        powered: !self.state.object_power(target_id, &self.definition),
                    },
                    "object.toggle_open" => ActivityCompletion::SetObjectOpen {
                        object_id: target_id.clone(),
                        open: !self.state.object_open(target_id, &self.definition),
                    },
                    "object.relocate" => {
                        let placement = parse_relocation_parameters(parameters).map_err(
                            |(code, message)| {
                                Box::new(self.command_error(
                                    command,
                                    CommandStatus::InvalidArgument,
                                    code,
                                    message,
                                ))
                            },
                        )?;
                        if placement.anchor_id != self.state.agent_anchor_id() {
                            return Err(Box::new(self.command_error(
                                command,
                                CommandStatus::RejectedPrecondition,
                                "DESTINATION_NOT_REACHABLE",
                                "rearrangement destination must be at the current anchor",
                            )));
                        }
                        if self.definition.object_has_component(target_id, "powerable")
                            && self.state.object_power(target_id, &self.definition)
                        {
                            return Err(Box::new(self.command_error(
                                command,
                                CommandStatus::RejectedPrecondition,
                                "OBJECT_POWERED",
                                "powered object must be switched off before relocation",
                            )));
                        }
                        if let Some(parent_id) = placement.parent_object_id.as_deref() {
                            let parent_reachable =
                                self.state.object_is_at_anchor(
                                    parent_id,
                                    self.state.agent_anchor_id(),
                                    &self.definition,
                                ) && self.state.object_is_visible(parent_id, &self.definition);
                            if !parent_reachable {
                                return Err(Box::new(self.command_error(
                                    command,
                                    CommandStatus::RejectedPrecondition,
                                    "DESTINATION_NOT_REACHABLE",
                                    "placement parent is not available at the current anchor",
                                )));
                            }
                            if placement.relation == PlacementRelation::Container
                                && self.definition.object_has_component(parent_id, "openable")
                                && !self.state.object_open(parent_id, &self.definition)
                            {
                                return Err(Box::new(self.command_error(
                                    command,
                                    CommandStatus::RejectedPrecondition,
                                    "DESTINATION_CLOSED",
                                    "container must be open before placing an object inside",
                                )));
                            }
                        }
                        let placements = self.state.object_placements(&self.definition);
                        self.definition
                            .validate_placement_change(target_id, &placement, &placements)
                            .map_err(|violation| {
                                Box::new(self.command_error(
                                    command,
                                    CommandStatus::RejectedPrecondition,
                                    violation.code,
                                    violation.message,
                                ))
                            })?;
                        ActivityCompletion::SetObjectPlacement {
                            object_id: target_id.clone(),
                            placement,
                        }
                    }
                    "object.clean" => {
                        let condition = self.state.object_condition(target_id, &self.definition);
                        let Some(cleanliness) = condition.cleanliness_permille else {
                            return Err(Box::new(self.command_error(
                                command,
                                CommandStatus::RejectedPrecondition,
                                "NOT_CLEANABLE",
                                "object has no cleanliness state",
                            )));
                        };
                        if cleanliness >= 1_000 {
                            return Err(Box::new(self.command_error(
                                command,
                                CommandStatus::RejectedPrecondition,
                                "ALREADY_CLEAN",
                                "object is already clean",
                            )));
                        }
                        ActivityCompletion::SetObjectCleanliness {
                            object_id: target_id.clone(),
                            base_condition: condition,
                            cleanliness_permille: 1_000,
                        }
                    }
                    "object.consume_quantity" => {
                        let amount =
                            parse_quantity_amount(parameters).map_err(|(code, message)| {
                                Box::new(self.command_error(
                                    command,
                                    CommandStatus::InvalidArgument,
                                    code,
                                    message,
                                ))
                            })?;
                        let condition = self.state.object_condition(target_id, &self.definition);
                        let Some(quantity) = condition.quantity.as_ref() else {
                            return Err(Box::new(self.command_error(
                                command,
                                CommandStatus::RejectedPrecondition,
                                "NO_QUANTITY_STATE",
                                "object has no finite quantity",
                            )));
                        };
                        if amount > quantity.amount {
                            return Err(Box::new(self.command_error(
                                command,
                                CommandStatus::RejectedPrecondition,
                                "INSUFFICIENT_QUANTITY",
                                format!("requested {amount}, available {}", quantity.amount),
                            )));
                        }
                        ActivityCompletion::ConsumeObjectQuantity {
                            object_id: target_id.clone(),
                            base_condition: condition,
                            amount,
                        }
                    }
                    _ => {
                        return Err(Box::new(self.command_error(
                            command,
                            CommandStatus::InvalidArgument,
                            "ACTION_NOT_IMPLEMENTED",
                            "the action exists in data but has no executor",
                        )));
                    }
                };
                let resources = affordance
                    .required_resources
                    .iter()
                    .filter_map(|resource| Resource::parse(resource))
                    .collect::<BTreeSet<_>>();
                self.ensure_resources(command, resources.iter().cloned())?;
                let activity = Activity {
                    activity_id: format!("activity-{}", command.command_id),
                    action_id: action_id.clone(),
                    target_id: target_id.clone(),
                    started_at_ms: now_ms,
                    completes_at_ms: now_ms.saturating_add(affordance.duration_ms),
                    resources,
                    completion,
                };
                let activity_id = activity.activity_id.clone();
                let mut events = vec![DomainEvent::ActivityScheduled { activity }];
                if affordance.duration_ms == 0 {
                    events.push(DomainEvent::ActivityCompleted { activity_id });
                }
                Ok(events)
            }
            CommandPayload::Inspect { .. } => Err(Box::new(self.command_error(
                command,
                CommandStatus::InvalidArgument,
                "ACTION_NOT_IMPLEMENTED",
                "inspect will be added with subjective observation events",
            ))),
        }
    }

    fn ensure_resources(
        &self,
        command: &CommandEnvelope,
        resources: impl IntoIterator<Item = Resource>,
    ) -> std::result::Result<(), Box<CommandResult>> {
        for resource in resources {
            if let Some(owner) = self.state.resource_owner(&resource) {
                return Err(Box::new(self.command_error(
                    command,
                    CommandStatus::ResourceConflict,
                    "RESOURCE_CONFLICT",
                    format!("resource {} is reserved by {owner}", resource.as_str()),
                )));
            }
        }
        Ok(())
    }

    fn command_error(
        &self,
        command: &CommandEnvelope,
        status: CommandStatus,
        code: &str,
        message: impl Into<String>,
    ) -> CommandResult {
        CommandResult::rejected(&command.command_id, status, &self.state, code, message)
    }

    fn complete_due_activities(&mut self, now_ms: i64, force_final: bool) -> Result<()> {
        let mut due = self.state.due_activities(now_ms);
        due.sort_by(|left, right| {
            left.completes_at_ms
                .cmp(&right.completes_at_ms)
                .then_with(|| left.activity_id.cmp(&right.activity_id))
        });

        let mut start = 0;
        while start < due.len() {
            let completed_at_ms = due[start].completes_at_ms;
            self.advance_passive(completed_at_ms, true)?;
            let mut end = start + 1;
            while end < due.len() && due[end].completes_at_ms == completed_at_ms {
                end += 1;
            }
            let events = due[start..end]
                .iter()
                .map(|activity| DomainEvent::ActivityCompleted {
                    activity_id: activity.activity_id.clone(),
                })
                .collect();
            self.state = self
                .store
                .commit_system_events(&self.state, completed_at_ms, events)?;
            start = end;
        }

        self.advance_passive(now_ms, force_final)
    }

    fn advance_passive(&mut self, to_utc_ms: i64, force: bool) -> Result<()> {
        let Some(event) = self.build_passive_event(to_utc_ms, force) else {
            return Ok(());
        };
        self.state = self
            .store
            .commit_system_events(&self.state, to_utc_ms, vec![event])?;
        Ok(())
    }

    fn build_passive_event(&self, to_utc_ms: i64, force: bool) -> Option<DomainEvent> {
        let from_utc_ms = self.state.passive_updated_at_ms();
        let elapsed_ms = to_utc_ms.saturating_sub(from_utc_ms);
        if elapsed_ms <= 0 || (!force && elapsed_ms < PASSIVE_EVENT_INTERVAL_MS) {
            return None;
        }

        let mut has_effects = false;
        let mut updates = Vec::new();
        let mut remainders = self.state.passive_remainders().clone();
        for (object_id, effects) in self.definition.passive_objects() {
            has_effects = true;
            let original = self.state.object_condition(object_id, &self.definition);
            let mut condition = original.clone();
            for effect in effects {
                let active = self.passive_is_active(object_id, effect);
                let key = format!("{object_id}:{}", effect.id());
                let carry = self.state.passive_remainder(&key);
                let (_, remainder) = match effect {
                    PassiveEffect::Charge {
                        active_delta_per_hour_permille,
                        inactive_delta_per_hour_permille,
                        ..
                    } => {
                        let current = i128::from(condition.charge_permille?);
                        let rate = if active {
                            *active_delta_per_hour_permille
                        } else {
                            *inactive_delta_per_hour_permille
                        };
                        let (next, remainder) =
                            advance_linear(current, rate, elapsed_ms, carry, 0, 1_000);
                        condition.charge_permille = Some(next as u16);
                        (next, remainder)
                    }
                    PassiveEffect::Temperature {
                        active_target_millicelsius,
                        active_change_per_hour_millicelsius,
                        inactive_target_millicelsius,
                        inactive_change_per_hour_millicelsius,
                        ..
                    } => {
                        let current = i128::from(condition.temperature_millicelsius?);
                        let (target, change) = if active {
                            (
                                i128::from(*active_target_millicelsius),
                                *active_change_per_hour_millicelsius,
                            )
                        } else {
                            (
                                i128::from(*inactive_target_millicelsius),
                                *inactive_change_per_hour_millicelsius,
                            )
                        };
                        let rate = match target.cmp(&current) {
                            std::cmp::Ordering::Less => -change,
                            std::cmp::Ordering::Equal => 0,
                            std::cmp::Ordering::Greater => change,
                        };
                        let (next, remainder) = advance_linear(
                            current,
                            rate,
                            elapsed_ms,
                            carry,
                            current.min(target),
                            current.max(target),
                        );
                        condition.temperature_millicelsius = Some(next as i32);
                        (next, remainder)
                    }
                    PassiveEffect::QuantityConsumption {
                        active_amount_per_hour,
                        inactive_amount_per_hour,
                        ..
                    } => {
                        let quantity = condition.quantity.as_mut()?;
                        let current = i128::from(quantity.amount);
                        let amount_per_hour = if active {
                            *active_amount_per_hour
                        } else {
                            *inactive_amount_per_hour
                        };
                        let rate = -(amount_per_hour as i64);
                        let (next, remainder) =
                            advance_linear(current, rate, elapsed_ms, carry, 0, current);
                        quantity.amount = next as u64;
                        (next, remainder)
                    }
                };
                if remainder == 0 {
                    remainders.remove(&key);
                } else {
                    remainders.insert(key, remainder);
                }
            }
            if condition != original {
                updates.push(PassiveConditionUpdate {
                    object_id: object_id.to_owned(),
                    condition,
                });
            }
        }

        if !has_effects {
            return None;
        }
        let remainder_changed = remainders != *self.state.passive_remainders();
        if !force && updates.is_empty() && !remainder_changed {
            return None;
        }
        Some(DomainEvent::PassiveConditionsAdvanced {
            from_utc_ms,
            to_utc_ms,
            updates,
            remainders,
        })
    }

    fn passive_is_active(&self, object_id: &str, effect: &PassiveEffect) -> bool {
        let activation = effect.activation();
        activation
            .power
            .is_none_or(|expected| self.state.object_power(object_id, &self.definition) == expected)
            && activation.open.is_none_or(|expected| {
                self.state.object_open(object_id, &self.definition) == expected
            })
            && activation.powered_placement.is_none_or(|expected| {
                let placement = self.state.object_placement(object_id, &self.definition);
                self.definition
                    .object_receives_placement_power(placement.as_ref())
                    == expected
            })
    }
}

fn advance_linear(
    current: i128,
    rate_per_hour: i64,
    elapsed_ms: i64,
    previous_remainder: i64,
    minimum: i128,
    maximum: i128,
) -> (i128, i64) {
    if rate_per_hour == 0
        || (rate_per_hour > 0 && current >= maximum)
        || (rate_per_hour < 0 && current <= minimum)
    {
        return (current.clamp(minimum, maximum), 0);
    }

    let mut carry = i128::from(previous_remainder);
    if carry != 0 && carry.signum() != i128::from(rate_per_hour).signum() {
        carry = 0;
    }
    let numerator = i128::from(rate_per_hour) * i128::from(elapsed_ms) + carry;
    let raw = current + numerator / MILLIS_PER_HOUR;
    let next = raw.clamp(minimum, maximum);
    let remainder = if next == raw {
        (numerator % MILLIS_PER_HOUR) as i64
    } else {
        0
    };
    (next, remainder)
}
fn validate_external_id(kind: &str, value: &str) -> Result<()> {
    if value.is_empty()
        || value.len() > 128
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.' | b':'))
    {
        return Err(WorldError::StateInvariant(format!(
            "{kind} is not a valid stable ID"
        )));
    }
    Ok(())
}

fn parse_relocation_parameters(
    parameters: &BTreeMap<String, String>,
) -> std::result::Result<ObjectPlacement, (&'static str, String)> {
    let allowed = BTreeSet::from(["relation", "anchor_id", "parent_object_id", "slot_id"]);
    if let Some(key) = parameters
        .keys()
        .find(|key| !allowed.contains(key.as_str()))
    {
        return Err((
            "UNKNOWN_PARAMETER",
            format!("object.relocate does not accept parameter {key}"),
        ));
    }
    let anchor_id = required_parameter(parameters, "anchor_id")?;
    validate_parameter_id("anchor_id", anchor_id)?;
    let relation = match required_parameter(parameters, "relation")? {
        "anchor" => PlacementRelation::Anchor,
        "surface" => PlacementRelation::Surface,
        "container" => PlacementRelation::Container,
        value => {
            return Err((
                "INVALID_PLACEMENT",
                format!("unknown placement relation {value}"),
            ));
        }
    };
    let parent_object_id = parameters.get("parent_object_id").cloned();
    let slot_id = parameters.get("slot_id").cloned();
    match relation {
        PlacementRelation::Anchor if parent_object_id.is_some() || slot_id.is_some() => {
            return Err((
                "INVALID_PLACEMENT",
                "anchor placement cannot declare parent_object_id or slot_id".into(),
            ));
        }
        PlacementRelation::Surface | PlacementRelation::Container => {
            let Some(parent) = parent_object_id.as_deref() else {
                return Err((
                    "MISSING_PARAMETER",
                    "parent_object_id is required for placed objects".into(),
                ));
            };
            let Some(slot) = slot_id.as_deref() else {
                return Err((
                    "MISSING_PARAMETER",
                    "slot_id is required for placed objects".into(),
                ));
            };
            validate_parameter_id("parent_object_id", parent)?;
            validate_parameter_id("slot_id", slot)?;
        }
        PlacementRelation::Anchor => {}
    }
    Ok(ObjectPlacement {
        anchor_id: anchor_id.into(),
        relation,
        parent_object_id,
        slot_id,
    })
}

fn parse_quantity_amount(
    parameters: &BTreeMap<String, String>,
) -> std::result::Result<u64, (&'static str, String)> {
    if let Some(key) = parameters.keys().find(|key| key.as_str() != "amount") {
        return Err((
            "UNKNOWN_PARAMETER",
            format!("object.consume_quantity does not accept parameter {key}"),
        ));
    }
    let value = parameters.get("amount").ok_or_else(|| {
        (
            "MISSING_PARAMETER",
            "amount is required for object.consume_quantity".into(),
        )
    })?;
    let amount = value.parse::<u64>().map_err(|_| {
        (
            "INVALID_PARAMETER",
            "amount must be a positive integer".into(),
        )
    })?;
    if amount == 0 {
        return Err((
            "INVALID_PARAMETER",
            "amount must be greater than zero".into(),
        ));
    }
    Ok(amount)
}

fn required_parameter<'a>(
    parameters: &'a BTreeMap<String, String>,
    name: &'static str,
) -> std::result::Result<&'a str, (&'static str, String)> {
    parameters.get(name).map(String::as_str).ok_or_else(|| {
        (
            "MISSING_PARAMETER",
            format!("{name} is required for object.relocate"),
        )
    })
}

fn validate_parameter_id(
    name: &str,
    value: &str,
) -> std::result::Result<(), (&'static str, String)> {
    validate_external_id(name, value)
        .map_err(|_| ("INVALID_PARAMETER", format!("{name} is not a stable ID")))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn definition() -> WorldDefinition {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../world-packages/test-room-v1/manifest.json");
        WorldDefinition::load(path, &PathGuard::default()).unwrap()
    }

    fn open(temp: &tempfile::TempDir, now_ms: i64) -> WorldEngine {
        WorldEngine::open(
            temp.path().join("world.db"),
            "test-makise",
            definition(),
            "bed",
            now_ms,
            &PathGuard::default(),
        )
        .unwrap()
    }

    fn move_command(engine: &WorldEngine, id: &str, now_ms: i64) -> CommandEnvelope {
        CommandEnvelope {
            command_id: id.into(),
            identity_id: "test-makise".into(),
            agent_id: "makise".into(),
            expected_world_version: engine.state().world_version(),
            schema_version: COMMAND_SCHEMA_VERSION,
            decision_id: format!("decision-{id}"),
            issued_at_ms: now_ms,
            ttl_ms: 30_000,
            payload: CommandPayload::MoveTo {
                target_anchor_id: "work_desk".into(),
            },
        }
    }

    #[test]
    fn duplicate_command_is_not_executed_twice() {
        let temp = tempfile::tempdir().unwrap();
        let mut engine = open(&temp, 1_000_000);
        let command = move_command(&engine, "cmd-1", 1_000_010);
        let first = engine.execute_command(&command, 1_000_010).unwrap();
        let version = engine.state().world_version();
        let duplicate = engine.execute_command(&command, 1_000_020).unwrap();

        assert_eq!(first.status, CommandStatus::Committed);
        assert_eq!(duplicate.status, CommandStatus::AlreadyCommitted);
        assert_eq!(engine.state().world_version(), version);
        assert_eq!(first.first_event_seq, duplicate.first_event_seq);
    }

    #[test]
    fn stale_decision_is_rejected_without_state_change() {
        let temp = tempfile::tempdir().unwrap();
        let mut engine = open(&temp, 2_000_000);
        let mut command = move_command(&engine, "cmd-stale", 2_000_010);
        command.expected_world_version = 0;
        let version = engine.state().world_version();

        let result = engine.execute_command(&command, 2_000_010).unwrap();
        assert_eq!(result.status, CommandStatus::StaleWorld);
        assert_eq!(engine.state().world_version(), version);
    }

    #[test]
    fn durable_activity_survives_restart_and_replays_identically() {
        let temp = tempfile::tempdir().unwrap();
        let before_hash;
        {
            let mut engine = open(&temp, 3_000_000);
            let command = move_command(&engine, "cmd-move", 3_000_010);
            engine.execute_command(&command, 3_000_010).unwrap();
            assert_eq!(engine.state().agent_anchor_id(), "bed");
            before_hash = engine.state().state_hash().unwrap();
        }

        let mut recovered = open(&temp, 3_001_000);
        assert_eq!(recovered.state().state_hash().unwrap(), before_hash);
        recovered.resume_after_downtime(3_005_000).unwrap();
        assert_eq!(recovered.state().agent_anchor_id(), "work_desk");
        let completed_hash = recovered.state().state_hash().unwrap();
        drop(recovered);

        let replayed = open(&temp, 3_005_000);
        assert_eq!(replayed.state().state_hash().unwrap(), completed_hash);
    }

    #[test]
    fn wall_clock_jump_enters_time_anomaly() {
        let temp = tempfile::tempdir().unwrap();
        let mut engine = open(&temp, 4_000_000);
        engine
            .tick(ClockSample {
                utc_ms: 4_300_000,
                monotonic_elapsed_ms: 1_000,
            })
            .unwrap();
        assert_eq!(engine.state().time_status(), &TimeStatus::TimeAnomaly);
    }

    #[test]
    fn perception_does_not_expose_hidden_object_properties() {
        let temp = tempfile::tempdir().unwrap();
        let mut engine = open(&temp, 5_000_000);
        let command = move_command(&engine, "cmd-perception", 5_000_010);
        engine.execute_command(&command, 5_000_010).unwrap();
        engine
            .tick(ClockSample {
                utc_ms: 5_005_000,
                monotonic_elapsed_ms: 5_000,
            })
            .unwrap();
        let json = serde_json::to_string(&engine.perception().unwrap()).unwrap();
        assert!(json.contains("desk_lamp"));
        assert!(!json.contains("factory_serial"));
        assert!(!json.contains("MAKISE-SECRET-001"));
    }

    #[test]
    fn command_id_collision_is_rejected() {
        let temp = tempfile::tempdir().unwrap();
        let mut engine = open(&temp, 6_000_000);
        let command = move_command(&engine, "cmd-collision", 6_000_010);
        engine.execute_command(&command, 6_000_010).unwrap();
        let mut collision = command.clone();
        collision.payload = CommandPayload::Inspect {
            target_id: "bed".into(),
        };
        let result = engine.execute_command(&collision, 6_000_020).unwrap();
        assert_eq!(result.status, CommandStatus::InvalidArgument);
        assert_eq!(result.error_code.as_deref(), Some("COMMAND_ID_COLLISION"));
    }
    fn weather_observation(observed_at_ms: i64) -> WeatherObservation {
        WeatherObservation {
            source: "open_meteo".into(),
            observed_at_ms,
            temperature_millicelsius: 21_500,
            relative_humidity_permille: 620,
            precipitation_micrometers: 200,
            snowfall_micrometers: 0,
            weather_code: 61,
            cloud_cover_permille: 750,
            wind_speed_mm_per_s: 3_400,
            wind_direction_degrees: 240,
            is_day: true,
        }
    }

    #[test]
    fn weather_observation_is_durable_and_deduplicated() {
        let temp = tempfile::tempdir().unwrap();
        let mut engine = open(&temp, 8_000_000);
        let observation = weather_observation(8_000_010);

        assert!(
            engine
                .observe_weather(observation.clone(), 8_000_020)
                .unwrap()
        );
        let version = engine.state().world_version();
        let state_hash = engine.state().state_hash().unwrap();
        assert!(
            !engine
                .observe_weather(observation.clone(), 8_000_030)
                .unwrap()
        );
        assert_eq!(engine.state().world_version(), version);
        assert_eq!(
            engine.events_after(0).unwrap().last().unwrap().event_type(),
            "weather_observed"
        );
        drop(engine);

        let recovered = open(&temp, 8_000_040);
        assert_eq!(recovered.state().weather_observation(), Some(&observation));
        assert_eq!(recovered.state().state_hash().unwrap(), state_hash);
    }

    #[test]
    fn invalid_weather_observation_is_rejected_without_state_change() {
        let temp = tempfile::tempdir().unwrap();
        let mut engine = open(&temp, 9_000_000);
        let version = engine.state().world_version();
        let mut observation = weather_observation(9_000_010);
        observation.relative_humidity_permille = 1_001;

        assert!(matches!(
            engine.observe_weather(observation, 9_000_020),
            Err(WorldError::InvalidWeatherObservation(_))
        ));
        assert_eq!(engine.state().world_version(), version);
        assert!(engine.state().weather_observation().is_none());
    }

    #[test]
    fn clock_checkpoint_distinguishes_idle_runtime_from_real_downtime() {
        let temp = tempfile::tempdir().unwrap();
        {
            let mut engine = open(&temp, 7_000_000);
            engine
                .tick(ClockSample {
                    utc_ms: 7_010_000,
                    monotonic_elapsed_ms: 10_000,
                })
                .unwrap();
            assert_eq!(engine.state().last_event_seq(), 1);
        }

        let mut recovered = open(&temp, 7_010_500);
        recovered.resume_after_downtime(7_010_500).unwrap();
        assert_eq!(recovered.state().last_event_seq(), 1);
        recovered.resume_after_downtime(7_012_000).unwrap();
        assert_eq!(recovered.state().last_event_seq(), 2);
    }
}
