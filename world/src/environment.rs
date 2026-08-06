use crate::WorldDefinition;
use crate::domain::{EnvironmentReliability, EnvironmentState, LightLevel, WorldState};

const PERCEPTION_TIME_BUCKET_MS: i64 = 300_000;
const NOVOSIBIRSK_UTC_OFFSET_MS: i64 = 7 * 60 * 60_000;
const DEFAULT_STALE_AFTER_MS: i64 = 45 * 60_000;
const DEFAULT_FALLBACK_AFTER_MS: i64 = 6 * 60 * 60_000;

pub(crate) fn project_environment(
    state: &WorldState,
    definition: &WorldDefinition,
    location_id: &str,
    now_ms: i64,
) -> EnvironmentState {
    let computed_at_ms = now_ms.div_euclid(PERCEPTION_TIME_BUCKET_MS) * PERCEPTION_TIME_BUCKET_MS;
    let weather = weather_values(state, definition, computed_at_ms);
    let profile = definition
        .sensory_profile(state.agent_anchor_id())
        .cloned()
        .unwrap_or_default();

    let mut light_sources = Vec::new();
    let mut sounds = profile.sound;
    let mut smells = profile.smell;
    let mut has_window = false;
    let mut has_open_window = false;
    let mut has_window_covering = false;
    let mut has_open_window_covering = false;
    let mut open_refrigerator = false;
    let mut heat_delta_millicelsius = 0_i32;

    for object in definition.observed_objects() {
        let Some(placement) = state.object_placement(&object.id, definition) else {
            continue;
        };
        let Some((object_location_id, _)) = definition.location_for_anchor(&placement.anchor_id)
        else {
            continue;
        };
        if object_location_id != location_id {
            continue;
        }

        let powered = state.object_power(&object.id, definition);
        let open = state.object_open(&object.id, definition);
        let is_window = definition.object_has_template(&object.id, "template.window")
            || object
                .observed_properties
                .get("kind")
                .is_some_and(|kind| kind == "balcony");
        let is_covering = definition.object_has_template(&object.id, "template.window_covering");
        if is_window {
            has_window = true;
            has_open_window |= open;
        }
        if is_covering {
            has_window_covering = true;
            has_open_window_covering |= open;
        }
        if powered && definition.object_has_template(&object.id, "template.light") {
            light_sources.push(format!("electric:{}", object.id));
        }
        if powered && definition.object_has_component(&object.id, "sound_emitter") {
            push_unique(&mut sounds, format!("Работает {}.", object.name));
        }
        if powered && definition.object_has_template(&object.id, "template.refrigerator") {
            push_unique(&mut sounds, "Тихо работает компрессор холодильника.".into());
        }
        if open && definition.object_has_template(&object.id, "template.refrigerator") {
            open_refrigerator = true;
            push_unique(
                &mut smells,
                "Из открытого холодильника ощущается прохладный запах продуктов.".into(),
            );
        }
        if open && object.id == "object.utility_cabinet" {
            push_unique(
                &mut smells,
                "Из открытого шкафа пахнет чистящими средствами.".into(),
            );
        }
        if definition.object_has_component(&object.id, "heatable") {
            let condition = state.object_condition(&object.id, definition);
            let object_temperature =
                condition
                    .temperature_millicelsius
                    .unwrap_or(if powered { 60_000 } else { 22_000 });
            heat_delta_millicelsius = heat_delta_millicelsius
                .saturating_add(((object_temperature - 22_000) / 40).clamp(0, 1_500));
        }
    }
    light_sources.sort();
    heat_delta_millicelsius = heat_delta_millicelsius.clamp(0, 3_000);

    if has_window && weather.precipitation_micrometers > 0 {
        push_unique(&mut sounds, "Осадки слышны по стеклу.".into());
        push_unique(
            &mut smells,
            "В воздухе чувствуется влажность после осадков.".into(),
        );
    }
    if has_open_window {
        push_unique(
            &mut sounds,
            "Через открытое окно слышен городской фон.".into(),
        );
        push_unique(
            &mut smells,
            "Через открытое окно поступает наружный воздух.".into(),
        );
        if weather.wind_speed_mm_per_s >= 8_000 {
            push_unique(&mut sounds, "У открытого окна заметно шумит ветер.".into());
        }
    }

    let natural_light =
        weather.is_day && has_window && (!has_window_covering || has_open_window_covering);
    if natural_light {
        light_sources.push("daylight".into());
    }
    if !weather.is_day && location_id == "balcony" {
        light_sources.push("city_glow".into());
    }
    let light_level = if light_sources
        .iter()
        .any(|source| source.starts_with("electric:"))
    {
        LightLevel::Bright
    } else if natural_light {
        match weather.cloud_cover_permille {
            0..=499 => LightLevel::Bright,
            500..=849 => LightLevel::Comfortable,
            _ => LightLevel::Dim,
        }
    } else if light_sources.iter().any(|source| source == "city_glow")
        || (weather.is_day && matches!(location_id, "entryway" | "corridor"))
    {
        LightLevel::Dim
    } else {
        LightLevel::Dark
    };

    let mut perceived_temperature_millicelsius = match location_id {
        "bathroom" => 23_500,
        "entryway" => 21_500,
        "balcony" => blend(22_000, weather.outdoor_temperature_millicelsius, 1, 4),
        _ => 22_000,
    };
    if has_open_window {
        perceived_temperature_millicelsius = blend(
            perceived_temperature_millicelsius,
            weather.outdoor_temperature_millicelsius,
            1,
            4,
        );
    }
    perceived_temperature_millicelsius = perceived_temperature_millicelsius
        .saturating_add(heat_delta_millicelsius)
        .saturating_sub(if open_refrigerator { 300 } else { 0 })
        .clamp(-60_000, 60_000);

    let base_humidity = match location_id {
        "bathroom" => 550,
        "balcony" => weather.relative_humidity_permille,
        _ => 450,
    };
    let relative_humidity_permille = if has_open_window {
        blend_u16(base_humidity, weather.relative_humidity_permille, 1, 3)
    } else {
        base_humidity
    };

    if sounds.is_empty() {
        sounds.push("Тихий фон квартиры.".into());
    }
    if smells.is_empty() {
        smells.push("Нейтральный воздух без заметного запаха.".into());
    }

    EnvironmentState {
        reliability: weather.reliability,
        weather_observed_at_ms: weather.observed_at_ms,
        computed_at_ms,
        outdoor_temperature_millicelsius: weather.outdoor_temperature_millicelsius,
        perceived_temperature_millicelsius,
        outdoor_relative_humidity_permille: weather.relative_humidity_permille,
        perceived_relative_humidity_permille: relative_humidity_permille,
        precipitation_micrometers: weather.precipitation_micrometers,
        snowfall_micrometers: weather.snowfall_micrometers,
        weather_code: weather.weather_code,
        cloud_cover_permille: weather.cloud_cover_permille,
        wind_speed_mm_per_s: weather.wind_speed_mm_per_s,
        wind_direction_degrees: weather.wind_direction_degrees,
        is_day: weather.is_day,
        light_level,
        light_sources,
        sounds,
        smells,
    }
}

pub(crate) fn render_environment_cues(
    environment: &EnvironmentState,
    fallback_description: Option<&str>,
) -> Vec<String> {
    let reliability = match environment.reliability {
        EnvironmentReliability::Live => "live",
        EnvironmentReliability::Cached => "cached",
        EnvironmentReliability::SeasonalFallback => "seasonal_fallback",
    };
    let light_level = match environment.light_level {
        LightLevel::Dark => "dark",
        LightLevel::Dim => "dim",
        LightLevel::Comfortable => "comfortable",
        LightLevel::Bright => "bright",
    };
    let mut cues = vec![format!(
        "Погода ({reliability}): {}; снаружи {}, влажность {}%, ветер {} м/с.",
        weather_description(environment.weather_code),
        format_temperature(environment.outdoor_temperature_millicelsius),
        environment.outdoor_relative_humidity_permille / 10,
        environment.wind_speed_mm_per_s as f64 / 1_000.0,
    )];
    if environment.reliability == EnvironmentReliability::SeasonalFallback
        && let Some(description) = fallback_description
    {
        cues.push(format!("Источник fallback: {description}."));
    }
    let sources = if environment.light_sources.is_empty() {
        "нет активных источников".into()
    } else {
        environment.light_sources.join(", ")
    };
    cues.push(format!("Свет ({light_level}): {sources}."));
    cues.push(format!(
        "Температура: ощущается {}.",
        format_temperature(environment.perceived_temperature_millicelsius)
    ));
    cues.extend(
        environment
            .sounds
            .iter()
            .map(|sound| format!("Звук: {sound}")),
    );
    cues.extend(
        environment
            .smells
            .iter()
            .map(|smell| format!("Запах: {smell}")),
    );
    cues
}

#[derive(Clone, Copy)]
struct WeatherValues {
    reliability: EnvironmentReliability,
    observed_at_ms: Option<i64>,
    outdoor_temperature_millicelsius: i32,
    relative_humidity_permille: u16,
    precipitation_micrometers: u32,
    snowfall_micrometers: u32,
    weather_code: u16,
    cloud_cover_permille: u16,
    wind_speed_mm_per_s: u32,
    wind_direction_degrees: u16,
    is_day: bool,
}

fn weather_values(
    state: &WorldState,
    definition: &WorldDefinition,
    computed_at_ms: i64,
) -> WeatherValues {
    let (stale_after_ms, fallback_after_ms) = definition
        .weather_site()
        .map(|site| (site.stale_after_ms, site.fallback_after_ms))
        .unwrap_or((DEFAULT_STALE_AFTER_MS, DEFAULT_FALLBACK_AFTER_MS));
    if let Some(observation) = state.weather_observation() {
        let age_ms = computed_at_ms
            .saturating_sub(observation.observed_at_ms)
            .max(0);
        let reliability = if age_ms <= stale_after_ms {
            Some(EnvironmentReliability::Live)
        } else if age_ms <= fallback_after_ms {
            Some(EnvironmentReliability::Cached)
        } else {
            None
        };
        if let Some(reliability) = reliability {
            return WeatherValues {
                reliability,
                observed_at_ms: Some(observation.observed_at_ms),
                outdoor_temperature_millicelsius: observation.temperature_millicelsius,
                relative_humidity_permille: observation.relative_humidity_permille,
                precipitation_micrometers: observation.precipitation_micrometers,
                snowfall_micrometers: observation.snowfall_micrometers,
                weather_code: observation.weather_code,
                cloud_cover_permille: observation.cloud_cover_permille,
                wind_speed_mm_per_s: observation.wind_speed_mm_per_s,
                wind_direction_degrees: observation.wind_direction_degrees,
                is_day: observation.is_day,
            };
        }
    }
    seasonal_fallback(computed_at_ms)
}

fn seasonal_fallback(computed_at_ms: i64) -> WeatherValues {
    let local_ms = computed_at_ms.saturating_add(NOVOSIBIRSK_UTC_OFFSET_MS);
    let local_hour = local_ms.div_euclid(3_600_000).rem_euclid(24) as u8;
    let month = month_from_unix_days(local_ms.div_euclid(86_400_000));
    let monthly_temperature = [
        -17_000, -15_000, -7_000, 4_000, 12_000, 18_000, 20_000, 17_000, 10_000, 3_000, -8_000,
        -15_000,
    ][usize::from(month - 1)];
    let daily_offset = match local_hour {
        0..=5 => -3_000,
        6..=9 => -1_500,
        10..=17 => 2_000,
        18..=21 => 0,
        _ => -1_500,
    };
    let (sunrise, sunset) = match month {
        12 | 1 | 2 => (8, 17),
        3..=5 => (6, 20),
        6..=8 => (5, 22),
        _ => (7, 19),
    };
    WeatherValues {
        reliability: EnvironmentReliability::SeasonalFallback,
        observed_at_ms: None,
        outdoor_temperature_millicelsius: monthly_temperature + daily_offset,
        relative_humidity_permille: if matches!(month, 11 | 12 | 1 | 2) {
            700
        } else {
            600
        },
        precipitation_micrometers: 0,
        snowfall_micrometers: 0,
        weather_code: 3,
        cloud_cover_permille: 500,
        wind_speed_mm_per_s: 2_500,
        wind_direction_degrees: 0,
        is_day: local_hour >= sunrise && local_hour < sunset,
    }
}

fn month_from_unix_days(days_since_epoch: i64) -> u8 {
    let shifted = days_since_epoch + 719_468;
    let era = if shifted >= 0 {
        shifted
    } else {
        shifted - 146_096
    } / 146_097;
    let day_of_era = shifted - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    (month_prime + if month_prime < 10 { 3 } else { -9 }) as u8
}

fn blend(indoor: i32, outdoor: i32, outdoor_weight: i32, total_weight: i32) -> i32 {
    let indoor_weight = total_weight - outdoor_weight;
    ((i64::from(indoor) * i64::from(indoor_weight)
        + i64::from(outdoor) * i64::from(outdoor_weight))
        / i64::from(total_weight)) as i32
}

fn blend_u16(indoor: u16, outdoor: u16, outdoor_weight: u16, total_weight: u16) -> u16 {
    let indoor_weight = total_weight - outdoor_weight;
    ((u32::from(indoor) * u32::from(indoor_weight)
        + u32::from(outdoor) * u32::from(outdoor_weight))
        / u32::from(total_weight)) as u16
}

fn push_unique(values: &mut Vec<String>, value: String) {
    if !values.contains(&value) {
        values.push(value);
    }
}

fn weather_description(code: u16) -> &'static str {
    match code {
        0 => "ясно",
        1..=3 => "облачно",
        45 | 48 => "туман",
        51..=57 => "морось",
        61..=67 | 80..=82 => "дождь",
        71..=77 | 85 | 86 => "снег",
        95..=99 => "гроза",
        _ => "неопределённые погодные условия",
    }
}

fn format_temperature(millicelsius: i32) -> String {
    format!("{:.1} °C", f64::from(millicelsius) / 1_000.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn civil_month_conversion_covers_epoch_and_seasons() {
        assert_eq!(month_from_unix_days(0), 1);
        assert_eq!(month_from_unix_days(181), 7);
        assert_eq!(month_from_unix_days(334), 12);
    }

    #[test]
    fn seasonal_fallback_is_deterministic_inside_time_bucket() {
        let first = seasonal_fallback(1_786_060_801_000);
        let second = seasonal_fallback(1_786_060_899_000);
        assert_eq!(
            first.outdoor_temperature_millicelsius,
            second.outdoor_temperature_millicelsius
        );
        assert_eq!(first.is_day, second.is_day);
        assert_eq!(first.reliability, EnvironmentReliability::SeasonalFallback);
    }
}
