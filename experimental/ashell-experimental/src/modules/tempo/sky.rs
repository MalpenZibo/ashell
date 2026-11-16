use ashell_i18n::t;
use ashell_services::weather::{self, WeatherData, WindSpeedUnit};
use guido::prelude::Color;
use styletto::Icon;

/// The line icon for a WMO code, by day or by night.
pub fn icon(code: u32, is_day: bool) -> Icon {
    match code {
        0 | 1 if is_day => Icon::Sun,
        0 | 1 => Icon::Moon,
        2 if is_day => Icon::CloudSun,
        2 => Icon::CloudMoon,
        3 => Icon::Cloud,
        45 | 48 => Icon::CloudFog,
        51..=57 => Icon::CloudDrizzle,
        61..=67 | 80..=82 => Icon::CloudRain,
        71..=77 | 85 | 86 => Icon::CloudSnow,
        95..=99 => Icon::CloudLightning,
        _ => Icon::Cloud,
    }
}

/// Whether something falls from the sky: rain, snow or a storm.
pub fn wet(code: u32) -> bool {
    matches!(code, 51..=67 | 71..=86 | 95..=99)
}

/// `accent` when something falls from the sky, `dry` otherwise.
pub fn ink(code: u32, accent: Color, dry: Color) -> Color {
    if wet(code) { accent } else { dry }
}

pub fn degrees(value: f32) -> String {
    format!("{}°", value.round())
}

/// An hour of the forecast as the views show it.
#[derive(Clone, Debug, PartialEq)]
pub struct Hour {
    pub when: String,
    pub code: u32,
    pub is_day: bool,
    pub temperature: f32,
}

impl Hour {
    pub fn new(hour: &weather::Hour, when: String) -> Self {
        Self {
            when,
            code: hour.code,
            is_day: hour.is_day,
            temperature: hour.temperature,
        }
    }
}

/// The forecast hours after the current conditions.
pub fn next_hours(data: &WeatherData) -> impl Iterator<Item = &weather::Hour> {
    data.hours
        .iter()
        .filter(|hour| hour.time > data.current.time)
}

pub fn describe(code: u32) -> String {
    match code {
        0 => t!("weather-clear-sky"),
        1 => t!("weather-mainly-clear"),
        2 => t!("weather-partly-cloudy"),
        3 => t!("weather-overcast"),
        45 => t!("weather-fog"),
        48 => t!("weather-fog-rime"),
        51 => t!("weather-drizzle-light"),
        53 => t!("weather-drizzle-moderate"),
        55 => t!("weather-drizzle-dense"),
        56 => t!("weather-drizzle-freezing-light"),
        57 => t!("weather-drizzle-freezing-dense"),
        61 => t!("weather-rain-slight"),
        63 => t!("weather-rain-moderate"),
        65 => t!("weather-rain-heavy"),
        66 => t!("weather-rain-freezing-light"),
        67 => t!("weather-rain-freezing-heavy"),
        71 => t!("weather-snow-slight"),
        73 => t!("weather-snow-moderate"),
        75 => t!("weather-snow-heavy"),
        77 => t!("weather-snow-grains"),
        80 => t!("weather-rain-showers-slight"),
        81 => t!("weather-rain-showers-moderate"),
        82 => t!("weather-rain-showers-violent"),
        85 => t!("weather-snow-showers-slight"),
        86 => t!("weather-snow-showers-heavy"),
        95 => t!("weather-thunderstorm"),
        96 => t!("weather-thunderstorm-hail-slight"),
        99 => t!("weather-thunderstorm-hail-heavy"),
        _ => t!("weather-unknown"),
    }
}

pub fn wind_symbol(unit: WindSpeedUnit) -> &'static str {
    match unit {
        WindSpeedUnit::Kmh => "km/h",
        WindSpeedUnit::Mph => "mph",
        WindSpeedUnit::Ms => "m/s",
    }
}
