use std::time::Duration;

use chrono::{NaiveDate, NaiveDateTime};
use iced::{
    Background, Border, Degrees, Element, Length, Padding, Rotation, Theme,
    alignment::{Horizontal, Vertical},
    core::svg::Handle,
    widget::{Column, MouseArea, Row, Svg, column, container, row, scrollable, svg, text},
};
use itertools::izip;
use serde::{Deserialize, Deserializer};

use crate::{
    config::WeatherLocation,
    i18n::{UnitSystem, chrono_locale, unit_system},
    t,
    theme::use_theme,
};

use super::{Message, Tempo};

impl Tempo {
    pub(super) fn weather<'a>(&'a self) -> Option<Element<'a, Message>> {
        let (space, font_size, opacity, radius) =
            use_theme(|t| (t.space, t.font_size, t.opacity, t.radius));
        let locale = chrono_locale();
        let units = unit_system();
        let temp = units.temperature_symbol();
        let wind = self.config.resolved_wind_speed_unit().symbol();
        let location_visible = self.location_visible;
        self.weather_data
            .as_ref()
            .zip(self.location.as_ref())
            .map(|(data, location)| {
                let inner_element: Element<'a, Message> = if location_visible {
                    let display_time =
                        self.time_str("%R", self.current_timezone_index, Some(data.current.time));
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
                    .size(font_size.sm)
                    .into()
                } else {
                    container(text("•••••").size(font_size.sm))
                        .style(move |theme: &Theme| container::Style {
                            background: Some(Background::Color(
                                theme.extended_palette().background.strong.color,
                            )),
                            border: Border::default().rounded(radius.sm),
                            ..Default::default()
                        })
                        .into()
                };

                let location_element: Element<'a, Message> = MouseArea::new(inner_element)
                    .on_press(Message::ToggleLocationVisibility)
                    .into();

                column!(
                    container(
                        row!(
                            weather_icon(data.current.weather_code, data.current.is_day > 0)
                                .height(font_size.xxl)
                                .width(Length::Shrink),
                            column!(
                                location_element,
                                text(weather_description(data.current.weather_code)),
                                row!(
                                    text(format!("{}{temp}", data.current.temperature_2m)),
                                    text(t!(
                                        "tempo-feels-like",
                                        value = data.current.apparent_temperature.round(),
                                        unit = temp,
                                    ))
                                    .size(font_size.sm)
                                )
                                .align_y(Vertical::Bottom)
                                .spacing(space.sm),
                            )
                            .width(Length::FillPortion(2))
                            .spacing(space.xs),
                            column!(
                                row!(
                                    svg(Handle::from_memory(include_bytes!(
                                        "../../../assets/weather_icon/drop.svg"
                                    )))
                                    .width(Length::Shrink)
                                    .height(font_size.lg),
                                    column!(
                                        text(t!("tempo-humidity"))
                                            .size(font_size.xs)
                                            .align_x(Horizontal::Right)
                                            .width(Length::Fill),
                                        text(format!("{}%", data.current.relative_humidity_2m))
                                            .align_x(Horizontal::Right)
                                            .size(font_size.xs)
                                            .width(Length::Fill),
                                    )
                                    .spacing(space.xxs)
                                )
                                .align_y(Vertical::Center)
                                .spacing(space.sm),
                                row!(
                                    svg(Handle::from_memory(include_bytes!(
                                        "../../../assets/weather_icon/wind.svg"
                                    )))
                                    .height(font_size.lg)
                                    .width(Length::Shrink)
                                    .rotation(
                                        Rotation::Floating(
                                            Degrees(data.current.wind_direction_10m as f32 + 90.)
                                                .into()
                                        )
                                    ),
                                    column!(
                                        text(t!("tempo-wind"))
                                            .size(font_size.xs)
                                            .align_x(Horizontal::Right)
                                            .width(Length::Fill),
                                        text(format!(
                                            "{} {wind}",
                                            data.current.wind_speed_10m.round()
                                        ))
                                        .align_x(Horizontal::Right)
                                        .size(font_size.xs)
                                        .width(Length::Fill),
                                    )
                                    .spacing(space.xxs)
                                )
                                .align_y(Vertical::Center)
                                .spacing(space.sm),
                            )
                            .width(Length::Fill)
                            .spacing(space.xs),
                        )
                        .spacing(space.lg)
                        .align_y(Vertical::Center)
                        .width(Length::Fill),
                    )
                    .padding(space.md)
                    .style(move |app_theme: &Theme| container::Style {
                        background: Background::Color(
                            app_theme
                                .extended_palette()
                                .background
                                .weak
                                .color
                                .scale_alpha(opacity),
                        )
                        .into(),
                        border: Border::default().rounded(radius.lg),
                        ..container::Style::default()
                    }),
                    container(
                        scrollable(
                            Row::with_children({
                                let mut time = data
                                    .hourly
                                    .time
                                    .iter()
                                    .enumerate()
                                    .filter(|(_, t)| **t > data.current.time)
                                    .take(23)
                                    .peekable();
                                let start_index = time.peek().map(|(index, _)| *index).unwrap_or(0);

                                izip!(
                                    time.map(|(_, v)| v),
                                    data.hourly.weather_code.iter().enumerate().filter_map(
                                        |(i, v)| if i >= start_index { Some(v) } else { None }
                                    ),
                                    data.hourly.temperature_2m.iter().enumerate().filter_map(
                                        |(i, v)| if i >= start_index { Some(v) } else { None }
                                    ),
                                    data.hourly.is_day.iter().enumerate().filter_map(|(i, v)| {
                                        if i >= start_index { Some(v) } else { None }
                                    }),
                                )
                                .map(|(hour_time, weather_code, temp_value, is_day)| {
                                    let display_time = self.time_str(
                                        "%H:%M",
                                        self.current_timezone_index,
                                        Some(*hour_time),
                                    );
                                    column!(
                                        text(format!("{}{temp}", temp_value.round())),
                                        weather_icon(*weather_code, *is_day > 0)
                                            .height(font_size.md)
                                            .width(Length::Shrink),
                                        text(display_time).size(font_size.sm)
                                    )
                                    .spacing(space.xs)
                                    .align_x(Horizontal::Center)
                                    .into()
                                })
                                .collect::<Vec<_>>()
                            })
                            .spacing(space.sm)
                            .padding(Padding::default().bottom(space.sm))
                        )
                        .horizontal()
                    )
                    .padding(space.sm)
                    .style(move |app_theme: &Theme| container::Style {
                        background: Background::Color(
                            app_theme
                                .extended_palette()
                                .background
                                .weak
                                .color
                                .scale_alpha(opacity),
                        )
                        .into(),
                        border: Border::default().rounded(radius.lg),
                        ..container::Style::default()
                    }),
                    Column::with_children(
                        izip!(
                            &data.daily.time,
                            &data.daily.weather_code,
                            &data.daily.temperature_2m_min,
                            &data.daily.temperature_2m_max,
                            &data.daily.wind_direction_10m_dominant,
                            &data.daily.wind_speed_10m_max,
                        )
                        .skip(1)
                        .enumerate()
                        .map(
                            |(
                                index,
                                (time, weather_code, temp_min, temp_max, wind_dir, wind_speed),
                            )| {
                                let last_index = data.daily.time.len() - 2;
                                container(
                                    row!(
                                        text(
                                            time.format_localized("%a, %d %b", locale).to_string()
                                        )
                                        .width(Length::FillPortion(5)),
                                        row!(
                                            weather_icon(*weather_code, true)
                                                .height(font_size.md)
                                                .width(Length::Fixed(font_size.md)),
                                            text(format!(
                                                "{}{temp}/{}{temp}",
                                                temp_max.round(),
                                                temp_min.round()
                                            ))
                                        )
                                        .spacing(space.xxs)
                                        .align_y(Vertical::Center)
                                        .width(Length::FillPortion(4)),
                                        row!(
                                            svg(Handle::from_memory(include_bytes!(
                                                "../../../assets/weather_icon/wind.svg"
                                            )))
                                            .height(font_size.md)
                                            .width(Length::Fixed(font_size.md))
                                            .rotation(Rotation::Floating(
                                                Degrees(*wind_dir as f32 + 90.).into()
                                            )),
                                            text(format!("{} {wind}", wind_speed.round()))
                                        )
                                        .spacing(space.xxs)
                                        .align_y(Vertical::Center)
                                        .width(Length::FillPortion(3))
                                    )
                                    .spacing(space.sm)
                                    .align_y(Vertical::Center),
                                )
                                .padding(space.sm)
                                .style(move |app_theme: &Theme| container::Style {
                                    background: Background::Color(
                                        app_theme
                                            .extended_palette()
                                            .background
                                            .weak
                                            .color
                                            .scale_alpha(opacity),
                                    )
                                    .into(),
                                    border: Border::default().rounded(iced::border::Radius {
                                        top_left: if index == 0 { radius.lg } else { radius.sm },
                                        top_right: if index == 0 { radius.lg } else { radius.sm },
                                        bottom_right: if index == last_index {
                                            radius.lg
                                        } else {
                                            radius.sm
                                        },
                                        bottom_left: if index == last_index {
                                            radius.lg
                                        } else {
                                            radius.sm
                                        },
                                    }),
                                    ..container::Style::default()
                                })
                                .into()
                            }
                        )
                    )
                    .spacing(space.xxs)
                )
                .spacing(space.sm)
                .into()
            })
    }
}

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

#[derive(Clone, Debug, Deserialize)]
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
                "https://geocoding-api.open-meteo.com/v1/search?name={}&count=1&language={}&format=json",
                city, lang
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
                _ => (format!("Lat: {}, Lon: {}", lat, lon), String::new()),
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
        "https://nominatim.openstreetmap.org/reverse?format=json&lat={}&lon={}&accept-language={}",
        lat, lon, lang
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
    wind_unit: crate::config::WindSpeedUnit,
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

#[derive(Clone, Debug, Deserialize)]
pub struct WeatherData {
    pub current: WeatherCondition,
    pub hourly: HourlyWeatherData,
    pub daily: DailyWeatherData,
}

#[derive(Clone, Debug, Deserialize)]
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

#[derive(Clone, Debug, Deserialize)]
pub struct HourlyWeatherData {
    #[serde(deserialize_with = "deserialize_datetime_vec")]
    pub time: Vec<NaiveDateTime>,
    pub weather_code: Vec<u32>,
    pub temperature_2m: Vec<f32>,
    pub is_day: Vec<u8>,
}

#[derive(Clone, Debug, Deserialize)]
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

pub fn weather_icon<'a>(code: u32, is_day: bool) -> Svg<'a> {
    match (code, is_day) {
        (0, true) => svg(Handle::from_memory(include_bytes!(
            "../../../assets/weather_icon/clear-day.svg"
        ))),
        (0, false) => svg(Handle::from_memory(include_bytes!(
            "../../../assets/weather_icon/clear-night.svg"
        ))),
        (1, true) => svg(Handle::from_memory(include_bytes!(
            "../../../assets/weather_icon/cloudy-1-day.svg"
        ))),
        (1, false) => svg(Handle::from_memory(include_bytes!(
            "../../../assets/weather_icon/cloudy-1-night.svg"
        ))),
        (2, true) => svg(Handle::from_memory(include_bytes!(
            "../../../assets/weather_icon/cloudy-3-day.svg"
        ))),
        (2, false) => svg(Handle::from_memory(include_bytes!(
            "../../../assets/weather_icon/cloudy-3-night.svg"
        ))),
        (3, _) => svg(Handle::from_memory(include_bytes!(
            "../../../assets/weather_icon/cloudy.svg"
        ))),
        (45, _) | (48, _) => svg(Handle::from_memory(include_bytes!(
            "../../../assets/weather_icon/fog.svg"
        ))),
        (51, true) => svg(Handle::from_memory(include_bytes!(
            "../../../assets/weather_icon/rainy-1-day.svg"
        ))),
        (51, false) => svg(Handle::from_memory(include_bytes!(
            "../../../assets/weather_icon/rainy-1-night.svg"
        ))),
        (53, true) | (56, true) => svg(Handle::from_memory(include_bytes!(
            "../../../assets/weather_icon/rainy-2-day.svg"
        ))),
        (53, false) | (56, false) => svg(Handle::from_memory(include_bytes!(
            "../../../assets/weather_icon/rainy-2-night.svg"
        ))),
        (55, true) | (57, true) => svg(Handle::from_memory(include_bytes!(
            "../../../assets/weather_icon/rainy-3-day.svg"
        ))),
        (55, false) | (57, false) => svg(Handle::from_memory(include_bytes!(
            "../../../assets/weather_icon/rainy-3-night.svg"
        ))),
        (61, _) => svg(Handle::from_memory(include_bytes!(
            "../../../assets/weather_icon/rainy-1.svg"
        ))),
        (63, _) | (66, _) => svg(Handle::from_memory(include_bytes!(
            "../../../assets/weather_icon/rainy-2.svg"
        ))),
        (65, _) | (67, _) => svg(Handle::from_memory(include_bytes!(
            "../../../assets/weather_icon/rainy-3.svg"
        ))),
        (71, _) | (77, _) => svg(Handle::from_memory(include_bytes!(
            "../../../assets/weather_icon/snowy-1.svg"
        ))),
        (73, _) => svg(Handle::from_memory(include_bytes!(
            "../../../assets/weather_icon/snowy-2.svg"
        ))),
        (75, _) => svg(Handle::from_memory(include_bytes!(
            "../../../assets/weather_icon/snowy-3.svg"
        ))),
        (80, true) => svg(Handle::from_memory(include_bytes!(
            "../../../assets/weather_icon/showers-rainy-1-day.svg"
        ))),
        (80, false) => svg(Handle::from_memory(include_bytes!(
            "../../../assets/weather_icon/showers-rainy-1-night.svg"
        ))),
        (81, true) => svg(Handle::from_memory(include_bytes!(
            "../../../assets/weather_icon/showers-rainy-2-day.svg"
        ))),
        (81, false) => svg(Handle::from_memory(include_bytes!(
            "../../../assets/weather_icon/showers-rainy-2-night.svg"
        ))),
        (82, true) => svg(Handle::from_memory(include_bytes!(
            "../../../assets/weather_icon/showers-rainy-3-day.svg"
        ))),
        (82, false) => svg(Handle::from_memory(include_bytes!(
            "../../../assets/weather_icon/showers-rainy-3-night.svg"
        ))),
        (85, true) => svg(Handle::from_memory(include_bytes!(
            "../../../assets/weather_icon/snowy-2-day.svg"
        ))),
        (85, false) => svg(Handle::from_memory(include_bytes!(
            "../../../assets/weather_icon/snowy-2-night.svg"
        ))),
        (86, true) => svg(Handle::from_memory(include_bytes!(
            "../../../assets/weather_icon/snowy-3-day.svg"
        ))),
        (86, false) => svg(Handle::from_memory(include_bytes!(
            "../../../assets/weather_icon/snowy-3-night.svg"
        ))),
        (95, _) => svg(Handle::from_memory(include_bytes!(
            "../../../assets/weather_icon/isolated-thunderstorms.svg"
        ))),
        (96, _) => svg(Handle::from_memory(include_bytes!(
            "../../../assets/weather_icon/scattered-thunderstorms.svg"
        ))),
        (99, _) => svg(Handle::from_memory(include_bytes!(
            "../../../assets/weather_icon/severe-thunderstorm.svg"
        ))),
        _ => svg(Handle::from_memory(include_bytes!(
            "../../../assets/weather_icon/unknown.svg"
        ))),
    }
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
