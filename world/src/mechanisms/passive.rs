use crate::WorldDefinition;
use crate::definition_stage4::PassiveEffect;
use crate::domain::{DomainEvent, PassiveConditionUpdate, WorldState};

const EVENT_INTERVAL_MS: i64 = 60_000;
const MILLIS_PER_HOUR: i128 = 3_600_000;

pub(crate) fn propose_transition(
    state: &WorldState,
    definition: &WorldDefinition,
    to_utc_ms: i64,
    force: bool,
) -> Option<DomainEvent> {
    let from_utc_ms = state.passive_updated_at_ms();
    let elapsed_ms = to_utc_ms.saturating_sub(from_utc_ms);
    if elapsed_ms <= 0 || (!force && elapsed_ms < EVENT_INTERVAL_MS) {
        return None;
    }

    let mut has_effects = false;
    let mut updates = Vec::new();
    let mut remainders = state.passive_remainders().clone();
    for (object_id, effects) in definition.passive_objects() {
        has_effects = true;
        let original = state.object_condition(object_id, definition);
        let mut condition = original.clone();
        for effect in effects {
            let active = is_active(state, definition, object_id, effect);
            let key = format!("{object_id}:{}", effect.id());
            let carry = state.passive_remainder(&key);
            let remainder = match effect {
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
                    remainder
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
                    remainder
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
                    remainder
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
    let remainder_changed = remainders != *state.passive_remainders();
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

fn is_active(
    state: &WorldState,
    definition: &WorldDefinition,
    object_id: &str,
    effect: &PassiveEffect,
) -> bool {
    let activation = effect.activation();
    activation
        .power
        .is_none_or(|expected| state.object_power(object_id, definition) == expected)
        && activation
            .open
            .is_none_or(|expected| state.object_open(object_id, definition) == expected)
        && activation.powered_placement.is_none_or(|expected| {
            let placement = state.object_placement(object_id, definition);
            definition.object_receives_placement_power(placement.as_ref()) == expected
        })
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
