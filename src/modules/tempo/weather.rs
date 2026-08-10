use std::sync::Arc;
use std::time::Duration;

use chrono::{NaiveDate, NaiveDateTime};
use guido::prelude::*;
use itertools::izip;
use serde::{Deserialize, Deserializer};

use super::{TempoHandle, calendar};
use crate::config::{TempoModuleConfig, WeatherLocation, WindSpeedUnit};
use crate::i18n::{UnitSystem, chrono_locale, unit_system};
use crate::t;
use crate::theme::ThemeColors;

// ── Data model + fetching (upstream, verbatim except PartialEq derives) ─────

#[derive(Clone, Debug, Deserialize)]
struct GeoLocations {
    results: Vec<GeoLocation>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeoLocation {
    latitude: f32,
    longitude: f32,
    name: String,
    #[serde(default)]
    admin1: Option<String>,
    #[serde(default)]
    country: Option<String>,
}

impl From<GeoLocation> for Location {
    fn from(value: GeoLocation) -> Self {
        let region_name = if let Some(admin1) = &value.admin1 {
            if let Some(country) = &value.country {
                if admin1 == country || admin1 == &value.name {
                    country.clone()
                } else {
                    admin1.clone()
                }
            } else {
                admin1.clone()
            }
        } else {
            value.country.unwrap_or_default()
        };

        Location {
            latitude: value.latitude,
            longitude: value.longitude,
            city: value.name,
            region_name,
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IpLocation {
    lat: f32,
    lon: f32,
    city: String,
    region_name: String,
}

impl From<IpLocation> for Location {
    fn from(value: IpLocation) -> Self {
        Location {
            latitude: value.lat,
            longitude: value.lon,
            city: value.city,
            region_name: value.region_name,
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct Location {
    pub latitude: f32,
    pub longitude: f32,
    pub city: String,
    pub region_name: String,
}

pub async fn fetch_location(location: &WeatherLocation, lang: &str) -> anyhow::Result<Location> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(20))
        .build()?;

    match location {
        WeatherLocation::City(city) => {
            let url = format!(
                "https://geocoding-api.open-meteo.com/v1/search?name={city}&count=1&language={lang}&format=json"
            );
            let response = client.get(&url).send().await?;
            let raw_data = response.text().await?;

            let data: GeoLocations = serde_json::from_str(&raw_data)?;

            data.results
                .first()
                .ok_or_else(|| anyhow::anyhow!("No location found"))
                .cloned()
                .map(|l| l.into())
        }
        WeatherLocation::Current => {
            let find_location = "http://ip-api.com/json/";

            let response = client.get(find_location).send().await?;
            let raw_data = response.text().await?;

            let data: IpLocation = serde_json::from_str(&raw_data)?;

            Ok(data.into())
        }
        WeatherLocation::Coordinates(lat, lon) => {
            let (city, region_name) = match try_reverse_geocode(&client, *lat, *lon, lang).await {
                Ok(Some((city, region))) => (city, region),
                _ => (format!("Lat: {lat}, Lon: {lon}"), String::new()),
            };

            Ok(Location {
                latitude: *lat,
                longitude: *lon,
                city,
                region_name,
            })
        }
    }
}

async fn try_reverse_geocode(
    client: &reqwest::Client,
    lat: f32,
    lon: f32,
    lang: &str,
) -> anyhow::Result<Option<(String, String)>> {
    let url = format!(
        "https://nominatim.openstreetmap.org/reverse?format=json&lat={lat}&lon={lon}&accept-language={lang}"
    );

    let response = client
        .get(&url)
        .header("User-Agent", "ashell")
        .send()
        .await?;

    if response.status().is_success() {
        let raw_data = response.text().await?;

        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&raw_data)
            && let Some(address) = json.get("address")
        {
            let mut city = None;

            if let Some(c) = address.get("city").and_then(|v| v.as_str()) {
                city = Some(c);
            } else if let Some(t) = address.get("town").and_then(|v| v.as_str()) {
                city = Some(t);
            } else if let Some(v) = address.get("village").and_then(|v| v.as_str()) {
                city = Some(v);
            } else if let Some(h) = address.get("hamlet").and_then(|v| v.as_str()) {
                city = Some(h);
            }

            if let Some(country) = address.get("country").and_then(|v| v.as_str())
                && let Some(city_name) = city
            {
                return Ok(Some((
                    city_name.to_string(),
                    if city_name != country {
                        country.to_string()
                    } else {
                        String::new()
                    },
                )));
            }

            if let Some(city_name) = city {
                return Ok(Some((city_name.to_string(), String::new())));
            }
        }
    }

    Ok(None)
}

pub async fn fetch_weather_data(
    lat: f32,
    lon: f32,
    units: UnitSystem,
    wind_unit: WindSpeedUnit,
) -> anyhow::Result<WeatherData> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(20))
        .build()?;

    let temp_param = match units {
        UnitSystem::Metric => "celsius",
        UnitSystem::Imperial => "fahrenheit",
    };
    let wind_param = wind_unit.api_param();

    let response = client.get(format!(
        "https://api.open-meteo.com/v1/forecast?\
latitude={lat}&longitude={lon}\
&current=weather_code,apparent_temperature,relative_humidity_2m,temperature_2m,is_day,wind_speed_10m,wind_direction_10m\
&hourly=weather_code,temperature_2m,is_day\
&daily=weather_code,temperature_2m_max,temperature_2m_min,wind_speed_10m_max,wind_direction_10m_dominant\
&forecast_days=7\
&temperature_unit={temp_param}\
&wind_speed_unit={wind_param}\
&timezone=UTC"
    )).send().await?;
    let raw_data = response.text().await?;

    let data: WeatherData = serde_json::from_str(&raw_data)?;

    Ok(data)
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct WeatherData {
    pub current: WeatherCondition,
    pub hourly: HourlyWeatherData,
    pub daily: DailyWeatherData,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct WeatherCondition {
    #[serde(with = "offsetdatetime_no_seconds")]
    pub time: NaiveDateTime,
    pub weather_code: u32,
    pub temperature_2m: f32,
    pub apparent_temperature: f32,
    pub relative_humidity_2m: u32,
    pub wind_speed_10m: f32,
    pub wind_direction_10m: u32,
    pub is_day: u8,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct HourlyWeatherData {
    #[serde(deserialize_with = "deserialize_datetime_vec")]
    pub time: Vec<NaiveDateTime>,
    pub weather_code: Vec<u32>,
    pub temperature_2m: Vec<f32>,
    pub is_day: Vec<u8>,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct DailyWeatherData {
    #[serde(deserialize_with = "deserialize_date_vec")]
    pub time: Vec<NaiveDate>,
    pub weather_code: Vec<u32>,
    pub temperature_2m_max: Vec<f32>,
    pub temperature_2m_min: Vec<f32>,
    pub wind_speed_10m_max: Vec<f32>,
    pub wind_direction_10m_dominant: Vec<u32>,
}

fn deserialize_datetime_vec<'de, D>(d: D) -> Result<Vec<NaiveDateTime>, D::Error>
where
    D: Deserializer<'de>,
{
    let strs = Vec::<String>::deserialize(d)?;
    strs.into_iter()
        .map(|s| offsetdatetime_no_seconds::parse_str::<D>(&s))
        .collect()
}

fn deserialize_date_vec<'de, D>(d: D) -> Result<Vec<NaiveDate>, D::Error>
where
    D: Deserializer<'de>,
{
    let strs = Vec::<String>::deserialize(d)?;
    strs.into_iter()
        .map(|s| NaiveDate::parse_from_str(&s, "%Y-%m-%d").map_err(serde::de::Error::custom))
        .collect()
}

mod offsetdatetime_no_seconds {
    use chrono::NaiveDateTime;
    use serde::{Deserialize, Deserializer};

    pub fn parse_str<'de, D: Deserializer<'de>>(s: &str) -> Result<NaiveDateTime, D::Error> {
        let naive = NaiveDateTime::parse_from_str(s, "%FT%R").map_err(serde::de::Error::custom)?;

        Ok(naive)
    }

    pub fn deserialize<'de, D>(d: D) -> Result<NaiveDateTime, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(d)?;
        parse_str::<D>(&s)
    }
}

// ── Icons ────────────────────────────────────────────────────────────────────

macro_rules! wicon {
    ($name:literal) => {
        include_bytes!(concat!("../../../assets/weather_icon/", $name, ".svg")) as &'static [u8]
    };
}

const DROP_ICON: &[u8] = wicon!("drop");
const WIND_ICON: &[u8] = wicon!("wind");

fn weather_icon_bytes(code: u32, is_day: bool) -> &'static [u8] {
    match (code, is_day) {
        (0, true) => wicon!("clear-day"),
        (0, false) => wicon!("clear-night"),
        (1, true) => wicon!("cloudy-1-day"),
        (1, false) => wicon!("cloudy-1-night"),
        (2, true) => wicon!("cloudy-3-day"),
        (2, false) => wicon!("cloudy-3-night"),
        (3, _) => wicon!("cloudy"),
        (45, _) | (48, _) => wicon!("fog"),
        (51, true) => wicon!("rainy-1-day"),
        (51, false) => wicon!("rainy-1-night"),
        (53, true) | (56, true) => wicon!("rainy-2-day"),
        (53, false) | (56, false) => wicon!("rainy-2-night"),
        (55, true) | (57, true) => wicon!("rainy-3-day"),
        (55, false) | (57, false) => wicon!("rainy-3-night"),
        (61, _) => wicon!("rainy-1"),
        (63, _) | (66, _) => wicon!("rainy-2"),
        (65, _) | (67, _) => wicon!("rainy-3"),
        (71, _) | (77, _) => wicon!("snowy-1"),
        (73, _) => wicon!("snowy-2"),
        (75, _) => wicon!("snowy-3"),
        (80, true) => wicon!("showers-rainy-1-day"),
        (80, false) => wicon!("showers-rainy-1-night"),
        (81, true) => wicon!("showers-rainy-2-day"),
        (81, false) => wicon!("showers-rainy-2-night"),
        (82, true) => wicon!("showers-rainy-3-day"),
        (82, false) => wicon!("showers-rainy-3-night"),
        (85, true) => wicon!("snowy-2-day"),
        (85, false) => wicon!("snowy-2-night"),
        (86, true) => wicon!("snowy-3-day"),
        (86, false) => wicon!("snowy-3-night"),
        (95, _) => wicon!("isolated-thunderstorms"),
        (96, _) => wicon!("scattered-thunderstorms"),
        (99, _) => wicon!("severe-thunderstorm"),
        _ => wicon!("unknown"),
    }
}

pub fn weather_icon(code: u32, is_day: bool) -> guido::prelude::Image {
    image(ImageSource::SvgBytes(Arc::from(weather_icon_bytes(
        code, is_day,
    ))))
}

/// Sized weather icon.
pub fn weather_icon_sized(code: u32, is_day: bool, w: f32, h: f32) -> AnyWidget {
    weather_icon(code, is_day).width(w).height(h).into_any()
}

fn svg_icon(bytes: &'static [u8], w: Option<f32>, h: f32) -> AnyWidget {
    let img = image(ImageSource::SvgBytes(Arc::from(bytes)));
    let img = if let Some(w) = w { img.width(w) } else { img };
    img.height(h).into_any()
}

pub fn weather_description(code: u32) -> String {
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

// ── Menu view ────────────────────────────────────────────────────────────────

fn card(theme: ThemeColors) -> Container {
    container()
        .width(fill())
        .padding(12)
        .corner_radius(16)
        .background(theme.background.lighter(0.05))
}

/// Weather column: current conditions, hourly strip, daily list.
pub fn view(handle: TempoHandle, config: TempoModuleConfig) -> impl Widget {
    let theme = expect_context::<ThemeColors>();
    let temp = unit_system().temperature_symbol();
    let wind = config.resolved_wind_speed_unit().symbol();

    container().width(fill()).child(move || {
        let data = handle.weather.get()?;
        let location = handle.location.get()?;
        let locale = chrono_locale();
        let location_visible = handle.location_visible.get();

        // ── Current conditions card ──
        let location_line: AnyWidget = if location_visible {
            let display_time = calendar::time_str(
                &config,
                "%R",
                handle.tz_index.get(),
                handle.date.get(),
                Some(data.current.time),
            );
            text(format!(
                "{}{} - {}",
                location.city,
                if location.region_name.is_empty() {
                    String::new()
                } else {
                    format!(", {}", location.region_name)
                },
                display_time
            ))
            .color(theme.text)
            .font_size(12)
            .into_any()
        } else {
            container()
                .padding([2, 6])
                .corner_radius(6)
                .background(theme.background.lighter(0.15))
                .child(text("•••••").color(theme.text).font_size(12))
                .into_any()
        };

        let location_btn = container()
            .on_click(move || handle.location_visible.update(|v| *v = !*v))
            .child(location_line);

        let current_left = container()
            .width(fill())
            .layout(Flex::column().spacing(4))
            .child(location_btn)
            .child(
                text(weather_description(data.current.weather_code))
                    .color(theme.text)
                    .font_size(14),
            )
            .child(
                container()
                    .layout(Flex::row().spacing(8).cross_alignment(CrossAlignment::End))
                    .child(
                        text(format!("{}{temp}", data.current.temperature_2m))
                            .color(theme.text)
                            .font_size(14),
                    )
                    .child(
                        text(t!(
                            "tempo-feels-like",
                            value = data.current.apparent_temperature.round(),
                            unit = temp,
                        ))
                        .color(theme.text)
                        .font_size(12),
                    ),
            );

        let right_stat = |icon_widget: AnyWidget, label: String, value: String| {
            container()
                .width(fill())
                .layout(
                    Flex::row()
                        .spacing(8)
                        .cross_alignment(CrossAlignment::Center)
                        .main_alignment(MainAlignment::End),
                )
                .child(icon_widget)
                .child(
                    container()
                        .layout(
                            Flex::column()
                                .spacing(2)
                                .cross_alignment(CrossAlignment::End),
                        )
                        .child(text(label).color(theme.text).font_size(11))
                        .child(text(value).color(theme.text).font_size(11)),
                )
        };

        let current_right = container()
            .width(fill())
            .layout(Flex::column().spacing(4))
            .child(right_stat(
                svg_icon(DROP_ICON, None, 16.0),
                t!("tempo-humidity"),
                format!("{}%", data.current.relative_humidity_2m),
            ))
            .child(right_stat(
                container()
                    .transform(Transform::rotate_degrees(
                        data.current.wind_direction_10m as f32 + 90.0,
                    ))
                    .child(svg_icon(WIND_ICON, None, 16.0))
                    .into_any(),
                t!("tempo-wind"),
                format!("{} {wind}", data.current.wind_speed_10m.round()),
            ));

        let current_card = card(theme).child(
            container()
                .width(fill())
                .layout(
                    Flex::row()
                        .spacing(16)
                        .cross_alignment(CrossAlignment::Center),
                )
                .child(weather_icon_sized(
                    data.current.weather_code,
                    data.current.is_day > 0,
                    48.0,
                    48.0,
                ))
                .child(current_left)
                .child(current_right),
        );

        // ── Hourly strip ──
        let mut hourly_row = container().layout(Flex::row().spacing(8));
        {
            let mut time = data
                .hourly
                .time
                .iter()
                .enumerate()
                .filter(|(_, t)| **t > data.current.time)
                .take(23)
                .peekable();
            let start_index = time.peek().map(|(index, _)| *index).unwrap_or(0);

            for (hour_time, weather_code, temp_value, is_day) in izip!(
                time.map(|(_, v)| v),
                data.hourly.weather_code.iter().skip(start_index),
                data.hourly.temperature_2m.iter().skip(start_index),
                data.hourly.is_day.iter().skip(start_index),
            ) {
                let display_time = calendar::time_str(
                    &config,
                    "%H:%M",
                    handle.tz_index.get(),
                    handle.date.get(),
                    Some(*hour_time),
                );
                hourly_row = hourly_row.child(
                    container()
                        .layout(
                            Flex::column()
                                .spacing(4)
                                .cross_alignment(CrossAlignment::Center),
                        )
                        .child(
                            text(format!("{}{temp}", temp_value.round()))
                                .color(theme.text)
                                .font_size(12),
                        )
                        .child(
                            weather_icon(*weather_code, *is_day > 0)
                                .width(16)
                                .height(16),
                        )
                        .child(text(display_time).color(theme.text).font_size(11)),
                );
            }
        }
        let hourly_card = card(theme).child(
            container()
                .width(fill())
                .scrollable(ScrollAxis::Horizontal)
                .child(hourly_row),
        );

        // ── Daily list (accordion radii) ──
        let mut daily_col = container().width(fill()).layout(Flex::column().spacing(2));
        let daily_count = data.daily.time.len().saturating_sub(1);
        for (index, (time, weather_code, temp_min, temp_max, wind_dir, wind_speed)) in izip!(
            &data.daily.time,
            &data.daily.weather_code,
            &data.daily.temperature_2m_min,
            &data.daily.temperature_2m_max,
            &data.daily.wind_direction_10m_dominant,
            &data.daily.wind_speed_10m_max,
        )
        .skip(1)
        .enumerate()
        {
            let is_first = index == 0;
            let is_last = index + 1 == daily_count;
            let radius = if is_first || is_last { 16.0 } else { 6.0 };

            daily_col = daily_col.child(
                container()
                    .width(fill())
                    .padding(8)
                    .corner_radius(radius)
                    .background(theme.background.lighter(0.05))
                    .layout(
                        Flex::row()
                            .spacing(8)
                            .cross_alignment(CrossAlignment::Center),
                    )
                    .child(
                        container().width(fill()).child(
                            text(time.format_localized("%a, %d %b", locale).to_string())
                                .color(theme.text)
                                .font_size(12),
                        ),
                    )
                    .child(
                        container()
                            .layout(
                                Flex::row()
                                    .spacing(2)
                                    .cross_alignment(CrossAlignment::Center),
                            )
                            .child(weather_icon_sized(*weather_code, true, 14.0, 14.0))
                            .child(
                                text(format!(
                                    "{}{temp}/{}{temp}",
                                    temp_max.round(),
                                    temp_min.round()
                                ))
                                .color(theme.text)
                                .font_size(12),
                            ),
                    )
                    .child(
                        container()
                            .layout(
                                Flex::row()
                                    .spacing(2)
                                    .cross_alignment(CrossAlignment::Center),
                            )
                            .child(
                                container()
                                    .transform(Transform::rotate_degrees(*wind_dir as f32 + 90.0))
                                    .child(svg_icon(WIND_ICON, Some(14.0), 14.0)),
                            )
                            .child(
                                text(format!("{} {wind}", wind_speed.round()))
                                    .color(theme.text)
                                    .font_size(12),
                            ),
                    ),
            );
        }

        Some(
            container()
                .width(fill())
                .layout(Flex::column().spacing(8))
                .child(current_card)
                .child(hourly_card)
                .child(daily_col),
        )
    })
}
