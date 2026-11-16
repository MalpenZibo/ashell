use crate::stream::channel;
use anyhow::{Context, anyhow};
use chrono::{NaiveDate, NaiveDateTime};
use futures::{SinkExt, Stream};
use log::{debug, warn};
use serde::{Deserialize, Deserializer};
use std::{
    hash::{Hash, Hasher},
    time::Duration,
};

const REFRESH: Duration = Duration::from_secs(30 * 60);
const RETRY_STEP: Duration = Duration::from_secs(60);
const TIMEOUT: Duration = Duration::from_secs(20);

#[derive(Debug, Clone, Default, PartialEq)]
pub enum WeatherLocation {
    /// Wherever the IP address says.
    #[default]
    Current,
    /// A place by name, as Open-Meteo's geocoding finds it.
    City(String),
    Coordinates(f32, f32),
}

impl Hash for WeatherLocation {
    fn hash<H: Hasher>(&self, state: &mut H) {
        std::mem::discriminant(self).hash(state);
        match self {
            WeatherLocation::Current => {}
            WeatherLocation::City(city) => city.hash(state),
            WeatherLocation::Coordinates(lat, lon) => {
                lat.to_bits().hash(state);
                lon.to_bits().hash(state);
            }
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum TemperatureUnit {
    #[default]
    Celsius,
    Fahrenheit,
}

impl TemperatureUnit {
    fn api_param(self) -> &'static str {
        match self {
            Self::Celsius => "celsius",
            Self::Fahrenheit => "fahrenheit",
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum WindSpeedUnit {
    #[default]
    Kmh,
    Mph,
    Ms,
}

impl WindSpeedUnit {
    fn api_param(self) -> &'static str {
        match self {
            Self::Kmh => "kmh",
            Self::Mph => "mph",
            Self::Ms => "ms",
        }
    }
}

/// What to forecast, and how to say it.
#[derive(Debug, Clone, Default, PartialEq, Hash)]
pub struct WeatherQuery {
    pub location: WeatherLocation,
    pub temperature_unit: TemperatureUnit,
    pub wind_speed_unit: WindSpeedUnit,
    /// The language subtag the place's name is asked in.
    pub language: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Place {
    pub latitude: f32,
    pub longitude: f32,
    pub city: String,
    /// The region or the country, empty when it adds nothing to the city.
    pub region: String,
}

/// Times are UTC; temperatures and wind speeds in the query's units.
#[derive(Debug, Clone, PartialEq)]
pub struct WeatherData {
    pub place: Place,
    pub current: Current,
    /// Every hour of the forecast, the past ones of today included.
    pub hours: Vec<Hour>,
    /// Seven days, today first.
    pub days: Vec<Day>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Current {
    pub time: NaiveDateTime,
    /// A WMO weather interpretation code.
    pub code: u32,
    pub temperature: f32,
    pub feels_like: f32,
    /// Percent.
    pub humidity: u32,
    pub wind_speed: f32,
    /// Degrees, where the wind comes from.
    pub wind_direction: u32,
    pub is_day: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Hour {
    pub time: NaiveDateTime,
    pub code: u32,
    pub temperature: f32,
    pub is_day: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Day {
    pub date: NaiveDate,
    pub code: u32,
    pub temperature_min: f32,
    pub temperature_max: f32,
    pub wind_speed_max: f32,
    /// Degrees, where the wind mostly comes from.
    pub wind_direction: u32,
    /// Percent, the day's mean.
    pub humidity: u32,
}

/// The forecast from Open-Meteo, for a place found by its name, its
/// coordinates or the IP address.
#[derive(Debug, Clone)]
pub struct Weather {
    client: reqwest::Client,
    query: WeatherQuery,
}

impl Weather {
    pub async fn connect(query: WeatherQuery) -> anyhow::Result<Self> {
        let client = reqwest::Client::builder().timeout(TIMEOUT).build()?;

        Ok(Self { client, query })
    }

    /// Yields `None` once subscribed, then the forecast every half hour. The
    /// place is looked up again each time, so `Current` follows the network.
    /// A failed fetch yields nothing and is retried sooner, a minute more
    /// after each failure in a row.
    pub fn updates(&self) -> impl Stream<Item = Option<WeatherData>> + Send + 'static {
        let client = self.client.clone();
        let query = self.query.clone();

        channel(1, |mut output| async move {
            if output.send(None).await.is_err() {
                return;
            }

            let mut failures: u32 = 0;
            loop {
                match fetch(&client, &query).await {
                    Ok(data) => {
                        debug!("Weather fetched for {}", data.place.city);
                        failures = 0;
                        if output.send(Some(data)).await.is_err() {
                            return;
                        }
                        tokio::time::sleep(REFRESH).await;
                    }
                    Err(err) => {
                        failures += 1;
                        let wait = (RETRY_STEP * failures).min(REFRESH);
                        warn!(
                            "Failed to fetch the weather: {err:#}, retrying in {}s",
                            wait.as_secs()
                        );
                        tokio::time::sleep(wait).await;
                    }
                }
            }
        })
    }
}

async fn fetch(client: &reqwest::Client, query: &WeatherQuery) -> anyhow::Result<WeatherData> {
    let place = find_place(client, &query.location, &query.language).await?;

    let forecast: Forecast = client
        .get(format!(
            "https://api.open-meteo.com/v1/forecast?\
latitude={}&longitude={}\
&current=weather_code,apparent_temperature,relative_humidity_2m,temperature_2m,is_day,wind_speed_10m,wind_direction_10m\
&hourly=weather_code,temperature_2m,is_day\
&daily=weather_code,temperature_2m_max,temperature_2m_min,wind_speed_10m_max,wind_direction_10m_dominant,relative_humidity_2m_mean\
&forecast_days=7\
&temperature_unit={}\
&wind_speed_unit={}\
&timezone=UTC",
            place.latitude,
            place.longitude,
            query.temperature_unit.api_param(),
            query.wind_speed_unit.api_param(),
        ))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await
        .context("no forecast")?;

    Ok(forecast.into_data(place))
}

async fn find_place(
    client: &reqwest::Client,
    location: &WeatherLocation,
    language: &str,
) -> anyhow::Result<Place> {
    match location {
        WeatherLocation::City(city) => {
            let found: GeoLocations = client
                .get(format!(
                    "https://geocoding-api.open-meteo.com/v1/search?name={city}&count=1&language={language}&format=json"
                ))
                .send()
                .await?
                .error_for_status()?
                .json()
                .await?;

            found
                .results
                .into_iter()
                .next()
                .map(Place::from)
                .ok_or_else(|| anyhow!("no place named {city}"))
        }
        WeatherLocation::Current => {
            let found: IpLocation = client
                .get("http://ip-api.com/json/")
                .send()
                .await?
                .error_for_status()?
                .json()
                .await?;

            Ok(found.into())
        }
        WeatherLocation::Coordinates(lat, lon) => {
            let (city, region) = match reverse_geocode(client, *lat, *lon, language).await {
                Ok(Some(names)) => names,
                Ok(None) => (format!("Lat: {lat}, Lon: {lon}"), String::new()),
                Err(err) => {
                    debug!("Failed to name the coordinates: {err:#}");
                    (format!("Lat: {lat}, Lon: {lon}"), String::new())
                }
            };

            Ok(Place {
                latitude: *lat,
                longitude: *lon,
                city,
                region,
            })
        }
    }
}

async fn reverse_geocode(
    client: &reqwest::Client,
    lat: f32,
    lon: f32,
    language: &str,
) -> anyhow::Result<Option<(String, String)>> {
    let response = client
        .get(format!(
            "https://nominatim.openstreetmap.org/reverse?format=json&lat={lat}&lon={lon}&accept-language={language}"
        ))
        .header("User-Agent", "ashell")
        .send()
        .await?;

    if !response.status().is_success() {
        return Ok(None);
    }

    let json: serde_json::Value = serde_json::from_str(&response.text().await?)?;

    Ok(names_from_address(json.get("address")))
}

fn names_from_address(address: Option<&serde_json::Value>) -> Option<(String, String)> {
    let address = address?;
    let city = ["city", "town", "village", "hamlet"]
        .iter()
        .find_map(|key| address.get(key).and_then(|v| v.as_str()))?;

    let region = match address.get("country").and_then(|v| v.as_str()) {
        Some(country) if country != city => country.to_string(),
        _ => String::new(),
    };

    Some((city.to_string(), region))
}

#[derive(Deserialize)]
struct GeoLocations {
    #[serde(default)]
    results: Vec<GeoLocation>,
}

#[derive(Deserialize)]
struct GeoLocation {
    latitude: f32,
    longitude: f32,
    name: String,
    #[serde(default)]
    admin1: Option<String>,
    #[serde(default)]
    country: Option<String>,
}

impl From<GeoLocation> for Place {
    fn from(value: GeoLocation) -> Self {
        let region = match (value.admin1, value.country) {
            (Some(admin1), Some(country)) if admin1 == country || admin1 == value.name => country,
            (Some(admin1), _) => admin1,
            (None, country) => country.unwrap_or_default(),
        };

        Place {
            latitude: value.latitude,
            longitude: value.longitude,
            city: value.name,
            region,
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct IpLocation {
    lat: f32,
    lon: f32,
    city: String,
    region_name: String,
}

impl From<IpLocation> for Place {
    fn from(value: IpLocation) -> Self {
        Place {
            latitude: value.lat,
            longitude: value.lon,
            city: value.city,
            region: value.region_name,
        }
    }
}

#[derive(Deserialize)]
struct Forecast {
    current: ForecastCurrent,
    hourly: ForecastHourly,
    daily: ForecastDaily,
}

#[derive(Deserialize)]
struct ForecastCurrent {
    #[serde(deserialize_with = "datetime")]
    time: NaiveDateTime,
    weather_code: u32,
    temperature_2m: f32,
    apparent_temperature: f32,
    relative_humidity_2m: u32,
    wind_speed_10m: f32,
    wind_direction_10m: u32,
    is_day: u8,
}

#[derive(Deserialize)]
struct ForecastHourly {
    #[serde(deserialize_with = "datetimes")]
    time: Vec<NaiveDateTime>,
    weather_code: Vec<u32>,
    temperature_2m: Vec<f32>,
    is_day: Vec<u8>,
}

#[derive(Deserialize)]
struct ForecastDaily {
    #[serde(deserialize_with = "dates")]
    time: Vec<NaiveDate>,
    weather_code: Vec<u32>,
    temperature_2m_max: Vec<f32>,
    temperature_2m_min: Vec<f32>,
    wind_speed_10m_max: Vec<f32>,
    wind_direction_10m_dominant: Vec<u32>,
    relative_humidity_2m_mean: Vec<u32>,
}

impl Forecast {
    fn into_data(self, place: Place) -> WeatherData {
        let Forecast {
            current,
            hourly,
            daily,
        } = self;

        let hours = hourly
            .time
            .into_iter()
            .zip(hourly.weather_code)
            .zip(hourly.temperature_2m)
            .zip(hourly.is_day)
            .map(|(((time, code), temperature), is_day)| Hour {
                time,
                code,
                temperature,
                is_day: is_day > 0,
            })
            .collect();

        let days = daily
            .time
            .into_iter()
            .zip(daily.weather_code)
            .zip(daily.temperature_2m_min)
            .zip(daily.temperature_2m_max)
            .zip(daily.wind_speed_10m_max)
            .zip(daily.wind_direction_10m_dominant)
            .zip(daily.relative_humidity_2m_mean)
            .map(
                |((((((date, code), min), max), wind_speed), wind_direction), humidity)| Day {
                    date,
                    code,
                    temperature_min: min,
                    temperature_max: max,
                    wind_speed_max: wind_speed,
                    wind_direction,
                    humidity,
                },
            )
            .collect();

        WeatherData {
            place,
            current: Current {
                time: current.time,
                code: current.weather_code,
                temperature: current.temperature_2m,
                feels_like: current.apparent_temperature,
                humidity: current.relative_humidity_2m,
                wind_speed: current.wind_speed_10m,
                wind_direction: current.wind_direction_10m,
                is_day: current.is_day > 0,
            },
            hours,
            days,
        }
    }
}

fn parse_datetime<E: serde::de::Error>(s: &str) -> Result<NaiveDateTime, E> {
    NaiveDateTime::parse_from_str(s, "%FT%R").map_err(E::custom)
}

fn datetime<'de, D: Deserializer<'de>>(d: D) -> Result<NaiveDateTime, D::Error> {
    parse_datetime(&String::deserialize(d)?)
}

fn datetimes<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<NaiveDateTime>, D::Error> {
    Vec::<String>::deserialize(d)?
        .iter()
        .map(|s| parse_datetime(s))
        .collect()
}

fn dates<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<NaiveDate>, D::Error> {
    Vec::<String>::deserialize(d)?
        .iter()
        .map(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").map_err(serde::de::Error::custom))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn place() -> Place {
        Place {
            latitude: 45.46,
            longitude: 9.19,
            city: "Milano".into(),
            region: "Lombardia".into(),
        }
    }

    #[test]
    fn a_forecast_reads_into_hours_and_days() {
        let json = r#"{
            "current": {
                "time": "2026-11-16T18:15", "weather_code": 61, "temperature_2m": 9.4,
                "apparent_temperature": 7.1, "relative_humidity_2m": 88,
                "wind_speed_10m": 12.3, "wind_direction_10m": 220, "is_day": 0
            },
            "hourly": {
                "time": ["2026-11-16T18:00", "2026-11-16T19:00"],
                "weather_code": [61, 3], "temperature_2m": [9.4, 8.9], "is_day": [0, 0]
            },
            "daily": {
                "time": ["2026-11-16", "2026-11-17"],
                "weather_code": [61, 2], "temperature_2m_max": [11.0, 13.5],
                "temperature_2m_min": [6.2, 5.8], "wind_speed_10m_max": [18.0, 9.0],
                "wind_direction_10m_dominant": [210, 90],
                "relative_humidity_2m_mean": [85, 70]
            }
        }"#;

        let data = serde_json::from_str::<Forecast>(json)
            .unwrap()
            .into_data(place());

        assert_eq!(data.current.code, 61);
        assert!(!data.current.is_day);
        assert_eq!(
            data.current.time,
            NaiveDateTime::parse_from_str("2026-11-16T18:15", "%FT%R").unwrap()
        );
        assert_eq!(data.hours.len(), 2);
        assert_eq!(data.hours[1].code, 3);
        assert_eq!(data.days.len(), 2);
        assert_eq!(data.days[1].temperature_max, 13.5);
        assert_eq!(data.days[1].wind_direction, 90);
        assert_eq!(data.days[0].humidity, 85);
    }

    #[test]
    fn a_geocoded_place_names_its_region_unless_it_repeats() {
        let geo = |name: &str, admin1: Option<&str>, country: Option<&str>| GeoLocation {
            latitude: 0.0,
            longitude: 0.0,
            name: name.into(),
            admin1: admin1.map(Into::into),
            country: country.map(Into::into),
        };

        let region = |g| Place::from(g).region;
        assert_eq!(
            region(geo("Milan", Some("Lombardy"), Some("Italy"))),
            "Lombardy"
        );
        assert_eq!(
            region(geo("Monaco", Some("Monaco"), Some("Monaco"))),
            "Monaco"
        );
        assert_eq!(
            region(geo("Berlin", Some("Berlin"), Some("Germany"))),
            "Germany"
        );
        assert_eq!(region(geo("Somewhere", None, Some("Chile"))), "Chile");
        assert_eq!(region(geo("Nowhere", None, None)), "");
    }

    #[test]
    fn reverse_geocoding_takes_the_nearest_settlement() {
        let address = serde_json::json!({ "village": "Bellagio", "country": "Italia" });
        assert_eq!(
            names_from_address(Some(&address)),
            Some(("Bellagio".into(), "Italia".into()))
        );

        let address = serde_json::json!({ "city": "Singapore", "country": "Singapore" });
        assert_eq!(
            names_from_address(Some(&address)),
            Some(("Singapore".into(), String::new()))
        );

        assert_eq!(names_from_address(Some(&serde_json::json!({}))), None);
        assert_eq!(names_from_address(None), None);
    }
}
