use ashell_i18n::{UnitSystem, unit_system};
use ashell_services::weather::{TemperatureUnit, WeatherLocation, WeatherQuery, WindSpeedUnit};
use chrono::FixedOffset;
use chrono_tz::Tz;
use toml::{Table, Value};

/// `[tempo]`: the time zones to go through, and where the weather is for.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TempoConfig {
    pub timezones: Vec<TimeZone>,
    /// No weather without one.
    pub weather_location: Option<WeatherLocation>,
    /// The locale's when unset.
    pub wind_speed_unit: Option<WindSpeedUnit>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TimeZone {
    /// As written.
    pub name: String,
    pub zone: Zone,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Zone {
    Named(Tz),
    Offset(FixedOffset),
}

impl TempoConfig {
    pub fn read(table: &Table) -> (Self, Vec<String>) {
        let mut problems = Vec::new();
        let mut warn =
            |key: &str, problem: String| problems.push(format!("tempo.{key}: {problem}"));
        let mut config = Self::default();

        for key in table.keys() {
            if !["timezones", "weather_location", "wind_speed_unit"].contains(&key.as_str()) {
                warn(key, "unknown key, ignored".into());
            }
        }

        if let Some(value) = table.get("timezones") {
            match value.as_array() {
                Some(names) => {
                    for (index, name) in names.iter().enumerate() {
                        let at = format!("timezones[{index}]");
                        match name.as_str().map(|name| (name, TimeZone::parse(name))) {
                            Some((_, Some(zone))) => config.timezones.push(zone),
                            Some((name, None)) => warn(
                                &at,
                                format!("\"{name}\" is neither a time zone nor an offset"),
                            ),
                            None => warn(&at, "expected a string".into()),
                        }
                    }
                }
                None => warn("timezones", "expected a list".into()),
            }
        }

        if let Some(value) = table.get("weather_location") {
            match location(value) {
                Ok(location) => config.weather_location = Some(location),
                Err(problem) => warn("weather_location", problem),
            }
        }

        if let Some(value) = table.get("wind_speed_unit") {
            match value.as_str() {
                Some("kmh") => config.wind_speed_unit = Some(WindSpeedUnit::Kmh),
                Some("mph") => config.wind_speed_unit = Some(WindSpeedUnit::Mph),
                Some("ms") => config.wind_speed_unit = Some(WindSpeedUnit::Ms),
                _ => warn(
                    "wind_speed_unit",
                    "expected \"kmh\", \"mph\" or \"ms\"".into(),
                ),
            }
        }

        (config, problems)
    }

    pub fn wind_speed_unit(&self) -> WindSpeedUnit {
        self.wind_speed_unit.unwrap_or(match unit_system() {
            UnitSystem::Metric => WindSpeedUnit::Kmh,
            UnitSystem::Imperial => WindSpeedUnit::Mph,
        })
    }

    pub fn weather_query(&self, language: String) -> Option<WeatherQuery> {
        Some(WeatherQuery {
            location: self.weather_location.clone()?,
            temperature_unit: match unit_system() {
                UnitSystem::Metric => TemperatureUnit::Celsius,
                UnitSystem::Imperial => TemperatureUnit::Fahrenheit,
            },
            wind_speed_unit: self.wind_speed_unit(),
            language,
        })
    }
}

impl TimeZone {
    /// An IANA name (`"Europe/Rome"`, `"UTC"`) or a fixed offset (`"+09:00"`).
    pub fn parse(name: &str) -> Option<Self> {
        let zone = name
            .parse::<FixedOffset>()
            .map(Zone::Offset)
            .or_else(|_| name.parse::<Tz>().map(Zone::Named))
            .ok()?;

        Some(Self {
            name: name.to_string(),
            zone,
        })
    }
}

fn location(value: &Value) -> Result<WeatherLocation, String> {
    const EXPECTED: &str = "expected \"current\", { city = \"<name>\" } or { coordinates = [<latitude>, <longitude>] }";

    match value {
        Value::String(name) if name == "current" => Ok(WeatherLocation::Current),
        Value::Table(table) if table.len() == 1 => {
            if let Some(city) = table.get("city") {
                return city
                    .as_str()
                    .map(|city| WeatherLocation::City(city.to_string()))
                    .ok_or_else(|| "city: expected a string".to_string());
            }
            if let Some(coordinates) = table.get("coordinates") {
                let numbers: Option<Vec<f64>> = coordinates.as_array().and_then(|values| {
                    values
                        .iter()
                        .map(|v| v.as_float().or_else(|| v.as_integer().map(|i| i as f64)))
                        .collect()
                });
                return match numbers.as_deref() {
                    Some(&[lat, lon]) if (-90.0..=90.0).contains(&lat) && (-180.0..=180.0).contains(&lon) => {
                        Ok(WeatherLocation::Coordinates(lat as f32, lon as f32))
                    }
                    Some(&[_, _]) => Err(
                        "coordinates: the latitude goes from -90 to 90, the longitude from -180 to 180"
                            .to_string(),
                    ),
                    _ => Err("coordinates: expected [<latitude>, <longitude>]".to_string()),
                };
            }
            Err(EXPECTED.to_string())
        }
        _ => Err(EXPECTED.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read(text: &str) -> (TempoConfig, Vec<String>) {
        TempoConfig::read(&toml::from_str(text).unwrap())
    }

    #[test]
    fn time_zones_are_names_or_offsets() {
        let (config, problems) =
            read(r#"timezones = ["Europe/Rome", "+09:00", "UTC", "Mars/Olympus"]"#);

        assert_eq!(
            config
                .timezones
                .iter()
                .map(|z| z.name.as_str())
                .collect::<Vec<_>>(),
            ["Europe/Rome", "+09:00", "UTC"]
        );
        assert!(matches!(config.timezones[1].zone, Zone::Offset(_)));
        assert_eq!(problems.len(), 1);
        assert!(problems[0].starts_with("tempo.timezones[3]"));
    }

    #[test]
    fn the_weather_is_for_the_ip_a_city_or_coordinates() {
        assert_eq!(
            read(r#"weather_location = "current""#).0.weather_location,
            Some(WeatherLocation::Current)
        );
        assert_eq!(
            read(r#"weather_location = { city = "Milano" }"#)
                .0
                .weather_location,
            Some(WeatherLocation::City("Milano".into()))
        );
        assert_eq!(
            read(r#"weather_location = { coordinates = [45, 9.2] }"#)
                .0
                .weather_location,
            Some(WeatherLocation::Coordinates(45.0, 9.2))
        );

        let (config, problems) = read(r#"weather_location = { coordinates = [95, 9] }"#);
        assert_eq!(config.weather_location, None);
        assert_eq!(problems.len(), 1);
    }

    #[test]
    fn without_a_location_there_is_no_weather() {
        assert_eq!(read("").0.weather_query("en".into()), None);
    }
}
