use std::time::Duration;

use serde::Deserialize;
use tokio::task::JoinHandle;

use crate::{ActorError, WeatherObservation, WeatherSite, WorldActorHandle};

const DEFAULT_ENDPOINT: &str = "https://api.open-meteo.com/v1/forecast";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug, thiserror::Error)]
pub enum WeatherPollerError {
    #[error("weather transport failed: {0}")]
    Transport(#[from] reqwest::Error),
    #[error("weather response is invalid JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("weather response is invalid: {0}")]
    InvalidResponse(String),
}

#[derive(Clone)]
pub struct OpenMeteoClient {
    client: reqwest::Client,
    endpoint: String,
}

impl OpenMeteoClient {
    pub fn from_environment() -> Result<Self, WeatherPollerError> {
        let endpoint =
            std::env::var("MAKISE_WEATHER_ENDPOINT").unwrap_or_else(|_| DEFAULT_ENDPOINT.into());
        Self::with_endpoint(endpoint)
    }

    pub fn with_endpoint(endpoint: impl Into<String>) -> Result<Self, WeatherPollerError> {
        let endpoint = endpoint.into();
        if endpoint.trim().is_empty() {
            return Err(WeatherPollerError::InvalidResponse(
                "weather endpoint cannot be empty".into(),
            ));
        }
        let client = reqwest::Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .build()?;
        Ok(Self { client, endpoint })
    }

    pub async fn fetch(
        &self,
        site: &WeatherSite,
    ) -> Result<WeatherObservation, WeatherPollerError> {
        validate_site(site)?;
        let query = [
            ("latitude", format_coordinate(site.latitude_e6)),
            ("longitude", format_coordinate(site.longitude_e6)),
            (
                "current",
                "temperature_2m,relative_humidity_2m,is_day,precipitation,snowfall,weather_code,cloud_cover,wind_speed_10m,wind_direction_10m".into(),
            ),
            ("timezone", site.timezone.clone()),
            ("timeformat", "unixtime".into()),
            ("wind_speed_unit", "ms".into()),
            ("forecast_days", "1".into()),
        ];
        let response = self
            .client
            .get(&self.endpoint)
            .query(&query)
            .send()
            .await?
            .error_for_status()?;
        let body = response.text().await?;
        parse_open_meteo(&body, &site.timezone)
    }
}

pub fn spawn_weather_poller(
    actor: WorldActorHandle,
    site: WeatherSite,
) -> Result<JoinHandle<()>, WeatherPollerError> {
    validate_site(&site)?;
    let interval_ms = u64::try_from(site.poll_interval_ms)
        .map_err(|_| WeatherPollerError::InvalidResponse("poll interval is invalid".into()))?;
    let client = OpenMeteoClient::from_environment()?;
    Ok(tokio::spawn(async move {
        let interval = Duration::from_millis(interval_ms);
        loop {
            match client.fetch(&site).await {
                Ok(observation) => match actor.observe_weather(observation).await {
                    Ok(_) => {}
                    Err(ActorError::Stopped) => break,
                    Err(error) => eprintln!("weather ingestion failed: {error}"),
                },
                Err(error) => eprintln!("weather polling failed; cached data retained: {error}"),
            }
            tokio::time::sleep(interval).await;
        }
    }))
}

fn validate_site(site: &WeatherSite) -> Result<(), WeatherPollerError> {
    if site.provider != "open_meteo"
        || !(-90_000_000..=90_000_000).contains(&site.latitude_e6)
        || !(-180_000_000..=180_000_000).contains(&site.longitude_e6)
        || site.timezone.trim().is_empty()
        || !(60_000..=3_600_000).contains(&site.poll_interval_ms)
        || !(site.poll_interval_ms..=21_600_000).contains(&site.stale_after_ms)
        || !(site.stale_after_ms..=172_800_000).contains(&site.fallback_after_ms)
    {
        return invalid("weather site configuration is invalid");
    }
    Ok(())
}

fn format_coordinate(value_e6: i32) -> String {
    format!("{:.6}", f64::from(value_e6) / 1_000_000.0)
}

#[derive(Debug, Deserialize)]
struct OpenMeteoResponse {
    timezone: String,
    current_units: CurrentUnits,
    current: CurrentWeather,
}

#[derive(Debug, Deserialize)]
struct CurrentUnits {
    time: String,
    temperature_2m: String,
    relative_humidity_2m: String,
    precipitation: String,
    snowfall: String,
    cloud_cover: String,
    wind_speed_10m: String,
    wind_direction_10m: String,
}

#[derive(Debug, Deserialize)]
struct CurrentWeather {
    time: i64,
    temperature_2m: f64,
    relative_humidity_2m: f64,
    is_day: u8,
    precipitation: f64,
    snowfall: f64,
    weather_code: u16,
    cloud_cover: f64,
    wind_speed_10m: f64,
    wind_direction_10m: f64,
}

fn parse_open_meteo(
    body: &str,
    expected_timezone: &str,
) -> Result<WeatherObservation, WeatherPollerError> {
    let response: OpenMeteoResponse = serde_json::from_str(body)?;
    if response.timezone != expected_timezone {
        return invalid(format!(
            "timezone mismatch: expected {expected_timezone}, got {}",
            response.timezone
        ));
    }
    let units = &response.current_units;
    let expected_units = [
        ("time", units.time.as_str(), "unixtime"),
        ("temperature_2m", units.temperature_2m.as_str(), "°C"),
        (
            "relative_humidity_2m",
            units.relative_humidity_2m.as_str(),
            "%",
        ),
        ("precipitation", units.precipitation.as_str(), "mm"),
        ("snowfall", units.snowfall.as_str(), "cm"),
        ("cloud_cover", units.cloud_cover.as_str(), "%"),
        ("wind_speed_10m", units.wind_speed_10m.as_str(), "m/s"),
        ("wind_direction_10m", units.wind_direction_10m.as_str(), "°"),
    ];
    for (field, actual, expected) in expected_units {
        if actual != expected {
            return invalid(format!(
                "unit mismatch for {field}: expected {expected}, got {actual}"
            ));
        }
    }

    let current = response.current;
    if current.is_day > 1 {
        return invalid("is_day must be zero or one");
    }
    let observed_at_ms = current
        .time
        .checked_mul(1_000)
        .ok_or_else(|| WeatherPollerError::InvalidResponse("time overflow".into()))?;
    let observation = WeatherObservation {
        source: "open_meteo".into(),
        observed_at_ms,
        temperature_millicelsius: scaled(current.temperature_2m, 1_000.0, "temperature")?
            .try_into()
            .map_err(|_| WeatherPollerError::InvalidResponse("temperature overflow".into()))?,
        relative_humidity_permille: scaled(
            current.relative_humidity_2m,
            10.0,
            "relative humidity",
        )?
        .try_into()
        .map_err(|_| WeatherPollerError::InvalidResponse("humidity overflow".into()))?,
        precipitation_micrometers: scaled(current.precipitation, 1_000.0, "precipitation")?
            .try_into()
            .map_err(|_| WeatherPollerError::InvalidResponse("precipitation overflow".into()))?,
        snowfall_micrometers: scaled(current.snowfall, 10_000.0, "snowfall")?
            .try_into()
            .map_err(|_| WeatherPollerError::InvalidResponse("snowfall overflow".into()))?,
        weather_code: current.weather_code,
        cloud_cover_permille: scaled(current.cloud_cover, 10.0, "cloud cover")?
            .try_into()
            .map_err(|_| WeatherPollerError::InvalidResponse("cloud cover overflow".into()))?,
        wind_speed_mm_per_s: scaled(current.wind_speed_10m, 1_000.0, "wind speed")?
            .try_into()
            .map_err(|_| WeatherPollerError::InvalidResponse("wind speed overflow".into()))?,
        wind_direction_degrees: scaled(current.wind_direction_10m, 1.0, "wind direction")?
            .try_into()
            .map_err(|_| WeatherPollerError::InvalidResponse("wind direction overflow".into()))?,
        is_day: current.is_day == 1,
    };
    observation
        .validate(observation.observed_at_ms)
        .map_err(|error| WeatherPollerError::InvalidResponse(error.to_string()))?;
    Ok(observation)
}

fn scaled(value: f64, factor: f64, field: &str) -> Result<i64, WeatherPollerError> {
    let value = value * factor;
    if !value.is_finite() || value < i64::MIN as f64 || value > i64::MAX as f64 {
        return invalid(format!("{field} is not finite or overflows"));
    }
    Ok(value.round() as i64)
}

fn invalid<T>(reason: impl Into<String>) -> Result<T, WeatherPollerError> {
    Err(WeatherPollerError::InvalidResponse(reason.into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"{
        "timezone":"Asia/Novosibirsk",
        "current_units":{
            "time":"unixtime",
            "temperature_2m":"°C",
            "relative_humidity_2m":"%",
            "is_day":"",
            "precipitation":"mm",
            "snowfall":"cm",
            "weather_code":"wmo code",
            "cloud_cover":"%",
            "wind_speed_10m":"m/s",
            "wind_direction_10m":"°"
        },
        "current":{
            "time":1786060800,
            "temperature_2m":21.56,
            "relative_humidity_2m":62,
            "is_day":1,
            "precipitation":0.2,
            "snowfall":0.01,
            "weather_code":61,
            "cloud_cover":75,
            "wind_speed_10m":3.4,
            "wind_direction_10m":240
        }
    }"#;

    #[test]
    fn parses_open_meteo_current_weather_into_integer_units() {
        let observation = parse_open_meteo(SAMPLE, "Asia/Novosibirsk").unwrap();
        assert_eq!(observation.observed_at_ms, 1_786_060_800_000);
        assert_eq!(observation.temperature_millicelsius, 21_560);
        assert_eq!(observation.relative_humidity_permille, 620);
        assert_eq!(observation.precipitation_micrometers, 200);
        assert_eq!(observation.snowfall_micrometers, 100);
        assert_eq!(observation.cloud_cover_permille, 750);
        assert_eq!(observation.wind_speed_mm_per_s, 3_400);
        assert!(observation.is_day);
    }

    #[test]
    fn rejects_response_with_unexpected_units() {
        let sample = SAMPLE.replace("\"m/s\"", "\"km/h\"");
        assert!(matches!(
            parse_open_meteo(&sample, "Asia/Novosibirsk"),
            Err(WeatherPollerError::InvalidResponse(_))
        ));
    }

    #[test]
    fn formats_coordinates_without_locale_dependence() {
        assert_eq!(format_coordinate(55_041_500), "55.041500");
        assert_eq!(format_coordinate(-82_934_600), "-82.934600");
    }
}
