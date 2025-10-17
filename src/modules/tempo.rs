use crate::{
    components::icons::{StaticIcon, icon},
    config::{TempoModuleConfig, WeatherLocation},
    theme::AshellTheme,
};
use chrono::{DateTime, Datelike, Days, Local, Months, NaiveDate, Weekday};
use iced::{
    Element, Length, Subscription,
    alignment::{Horizontal, Vertical},
    futures::SinkExt,
    stream::channel,
    time::every,
    widget::{Column, Row, button, column, row, text},
};
use log::{debug, warn};
use serde::{Deserialize, Deserializer};
use std::{any::TypeId, time::Duration};
use time::{Date, OffsetDateTime};

#[derive(Debug, Clone)]
pub enum Message {
    Update,
    ChangeSelectDate(Option<NaiveDate>),
    UpdateWeather(Box<WeatherData>),
}

pub enum Action {
    None,
}

pub struct Tempo {
    config: TempoModuleConfig,
    date: DateTime<Local>,
    selected_date: Option<NaiveDate>,
    weather_data: Option<WeatherData>,
}

impl Tempo {
    pub fn new(config: TempoModuleConfig) -> Self {
        Self {
            config,
            date: Local::now(),
            selected_date: None,
            weather_data: None,
        }
    }

    pub fn update(&mut self, message: Message) -> Action {
        match message {
            Message::Update => {
                self.date = Local::now();

                Action::None
            }
            Message::ChangeSelectDate(selected_date) => {
                self.selected_date = selected_date;

                Action::None
            }
            Message::UpdateWeather(data) => {
                self.weather_data = Some(*data);

                Action::None
            }
        }
    }

    pub fn view(&'_ self, theme: &AshellTheme) -> Element<'_, Message> {
        Row::new()
            .push_maybe(self.weather_indicator(theme))
            .push(text(
                self.date.format(&self.config.clock_format).to_string(),
            ))
            .spacing(theme.space.sm)
            .into()
    }

    pub fn weather_indicator(&'_ self, theme: &AshellTheme) -> Option<Element<'_, Message>> {
        self.weather_data.as_ref().map(|data| {
            row!(
                text(weather_icon(data.current.weather_code).to_string()),
                text(format!("{}°C", data.current.temperature_2m))
            )
            .spacing(theme.space.xxs)
            .into()
        })
    }

    pub fn menu_view<'a>(&'a self, theme: &'a AshellTheme) -> Element<'a, Message> {
        Row::new()
            .push(self.calendar(theme))
            .push_maybe(self.weather(theme))
            .spacing(theme.space.lg)
            .into()
    }

    fn calendar<'a>(&'a self, theme: &'a AshellTheme) -> Element<'a, Message> {
        let selected_date = self.selected_date.unwrap_or(self.date.date_naive());

        let current_month = selected_date.month0();
        let first_day_month = selected_date.with_day0(0).unwrap_or_default();
        let day_of_week_first_day = first_day_month.weekday();

        let mut current = first_day_month
            .checked_sub_days(Days::new(day_of_week_first_day.number_from_monday() as u64))
            .unwrap_or_default();

        let weeks_in_month = if current
            .checked_add_days(Days::new(5 * 7))
            .map(|d| d.month0())
            .unwrap_or_default()
            != current_month
        {
            5
        } else {
            6
        };

        let calendar = Column::new()
            .push(
                row!(
                    button(icon(StaticIcon::LeftChevron))
                        .on_press(Message::ChangeSelectDate(
                            selected_date.checked_sub_months(Months::new(1)),
                        ))
                        .padding([theme.space.xs, theme.space.md])
                        .style(theme.settings_button_style()),
                    text(selected_date.format("%B").to_string())
                        .size(theme.font_size.md)
                        .width(Length::Fill)
                        .align_x(Horizontal::Center),
                    button(icon(StaticIcon::RightChevron))
                        .on_press(Message::ChangeSelectDate(
                            selected_date.checked_add_months(Months::new(1))
                        ))
                        .padding([theme.space.xs, theme.space.md])
                        .style(theme.settings_button_style())
                )
                .width(Length::Fill)
                .align_y(Vertical::Center),
            )
            .push(
                Row::with_children(
                    [
                        Weekday::Mon,
                        Weekday::Tue,
                        Weekday::Wed,
                        Weekday::Thu,
                        Weekday::Fri,
                        Weekday::Sat,
                        Weekday::Sun,
                    ]
                    .into_iter()
                    .map(|i| {
                        text(i.to_string())
                            .align_x(Horizontal::Center)
                            .width(Length::Fill)
                            .into()
                    })
                    .collect::<Vec<Element<'a, Message>>>(),
                )
                .width(Length::Fill)
                .spacing(theme.space.sm),
            )
            .push(Column::with_children(
                (0..weeks_in_month)
                    .map(|_| {
                        Row::with_children(
                            (0..7)
                                .map(|_| {
                                    let day = current;
                                    current = current.succ_opt().unwrap_or(current);

                                    button(
                                        text(day.format("%d").to_string())
                                            .align_x(Horizontal::Center)
                                            .color_maybe({
                                                if day == self.date.date_naive() {
                                                    Some(theme.iced_theme.palette().success)
                                                } else if day == selected_date {
                                                    Some(theme.iced_theme.palette().primary)
                                                } else if day.month0() != current_month {
                                                    Some(
                                                        theme
                                                            .iced_theme
                                                            .palette()
                                                            .text
                                                            .scale_alpha(0.2),
                                                    )
                                                } else {
                                                    None
                                                }
                                            }),
                                    )
                                    .on_press_maybe(if day != self.date.date_naive() {
                                        Some(Message::ChangeSelectDate(Some(day)))
                                    } else {
                                        None
                                    })
                                    .width(Length::Fill)
                                    .style(theme.ghost_button_style())
                                    .into()
                                })
                                .collect::<Vec<Element<'a, Message>>>(),
                        )
                        .spacing(theme.space.sm)
                        .width(Length::Fill)
                        .into()
                    })
                    .collect::<Vec<Element<'a, Message>>>(),
            ))
            .spacing(theme.space.md)
            .width(Length::Fixed(225.));

        column!(
            button(
                column!(
                    text(self.date.format("%A").to_string()).size(theme.font_size.md),
                    text(self.date.format("%d %B %Y").to_string()).size(theme.font_size.lg),
                )
                .spacing(theme.space.xs)
            )
            .on_press_maybe(if self.selected_date.is_some() {
                Some(Message::ChangeSelectDate(None))
            } else {
                None
            })
            .style(theme.ghost_button_style()),
            calendar
        )
        .spacing(theme.space.lg)
        .into()
    }

    fn weather<'a>(&'a self, _: &'a AshellTheme) -> Option<Element<'a, Message>> {
        self.weather_data.as_ref().map(|data| {
            column!(text(weather_icon(data.current.weather_code)))
                .width(Length::Fill)
                .into()
        })
    }

    pub fn subscription(&self) -> Subscription<Message> {
        let second_specifiers = [
            "%S",  // Seconds (00-60)
            "%T",  // Hour:Minute:Second
            "%X",  // Locale time representation with seconds
            "%r",  // 12-hour clock time with seconds
            "%:z", // UTC offset with seconds
            "%s",  // Unix timestamp (seconds since epoch)
        ];
        let interval = if second_specifiers
            .iter()
            .any(|&spec| self.config.clock_format.contains(spec))
        {
            Duration::from_secs(1)
        } else {
            Duration::from_secs(5)
        };

        let location = self.config.weather_location.clone();

        Subscription::batch(vec![
            every(interval).map(|_| Message::Update),
            Subscription::run_with_id(
                (
                    TypeId::of::<Self>(),
                    format!("{:?}", self.config.weather_location),
                    "weather",
                ),
                channel(100, async move |mut output| {
                    let location = fetch_location(location).await;

                    match location {
                        Ok(loc) => {
                            debug!("Location fetched successfully: {:?}", loc);

                            loop {
                                let data = fetch_weather_data(&loc).await;

                                match data {
                                    Ok(weather_data) => {
                                        debug!(
                                            "Weather data fetched successfully: {:?}",
                                            weather_data
                                        );
                                        output
                                            .send(Message::UpdateWeather(Box::new(weather_data)))
                                            .await
                                            .ok();
                                    }
                                    Err(e) => {
                                        warn!("Failed to fetch weather data: {:?}", e);
                                    }
                                }

                                tokio::time::sleep(Duration::from_secs(60 * 30)).await;
                            }
                        }
                        Err(e) => {
                            warn!("Failed to fetch location: {:?}", e);
                        }
                    }
                }),
            ),
        ])
    }
}

async fn fetch_location(location: WeatherLocation) -> anyhow::Result<Location> {
    match location {
        WeatherLocation::City(city) => {
            let url = format!(
                "https://geocoding-api.open-meteo.com/v1/search?name={}&count=1&language=en&format=json",
                city
            );
            let response = reqwest::get(&url).await?;
            let raw_data = response.text().await?;

            let data: Locations = serde_json::from_str(&raw_data)?;

            data.results
                .first()
                .ok_or_else(|| anyhow::anyhow!("No location found"))
                .cloned()
        }
        WeatherLocation::Current => {
            let find_location = "http://ip-api.com/json/";

            let response = reqwest::get(find_location).await?;
            let raw_data = response.text().await?;

            let data: IpApiResponse = serde_json::from_str(&raw_data)?;

            Ok(Location {
                latitude: data.lat,
                longitude: data.lon,
            })
        }
        WeatherLocation::Coordinates {
            latitude,
            longitude,
        } => Ok(Location {
            latitude,
            longitude,
        }),
    }
}

#[derive(Clone, Debug, Deserialize)]
struct Locations {
    results: Vec<Location>,
}

#[derive(Clone, Copy, Debug, Deserialize)]
struct Location {
    latitude: f32,
    longitude: f32,
}

#[derive(Clone, Copy, Debug, Deserialize)]
struct IpApiResponse {
    lat: f32,
    lon: f32,
}

async fn fetch_weather_data(location: &Location) -> anyhow::Result<WeatherData> {
    let response = reqwest::get(format!(
        "https://api.open-meteo.com/v1/forecast?\
        latitude={}&longitude={}\
        &current=weather_code,apparent_temperature,relative_humidity_2m,temperature_2m,is_day,wind_speed_10m,wind_direction_10m\
        &hourly=weather_code,temperature_2m,is_day\
        &daily=weather_code,temperature_2m_max,temperature_2m_min,apparent_temperature_max,apparent_temperature_min,wind_speed_10m_max,wind_direction_10m_dominant\
        &forecast_days=7", 
        location.latitude, location.longitude
    )).await?;
    let raw_data = response.text().await?;

    let data: WeatherData = serde_json::from_str(&raw_data)?;

    Ok(data)
}

#[derive(Clone, Debug, Deserialize)]
pub struct WeatherData {
    current: WeatherCondition,
    hourly: HourlyWeatherData,
    daily: DailyWeatherData,
}

#[derive(Clone, Debug, Deserialize)]
pub struct WeatherCondition {
    #[serde(with = "offsetdatetime_no_seconds")]
    time: OffsetDateTime,
    weather_code: u32,
    temperature_2m: f32,
    apparent_temperature: f32,
    relative_humidity_2m: u32,
    wind_speed_10m: f32,
    wind_direction_10m: u32,
    is_day: u8,
}

#[derive(Clone, Debug, Deserialize)]
pub struct HourlyWeatherData {
    #[serde(deserialize_with = "deserialize_datetime_vec")]
    time: Vec<OffsetDateTime>,
    weather_code: Vec<u32>,
    temperature_2m: Vec<f32>,
    is_day: Vec<u8>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct DailyWeatherData {
    #[serde(deserialize_with = "deserialize_date_vec")]
    time: Vec<Date>,
    weather_code: Vec<u32>,
    temperature_2m_max: Vec<f32>,
    temperature_2m_min: Vec<f32>,
    apparent_temperature_max: Vec<f32>,
    apparent_temperature_min: Vec<f32>,
    wind_speed_10m_max: Vec<f32>,
    wind_direction_10m_dominant: Vec<u32>,
}

fn deserialize_datetime_vec<'de, D>(d: D) -> Result<Vec<OffsetDateTime>, D::Error>
where
    D: Deserializer<'de>,
{
    let strs = Vec::<String>::deserialize(d)?;
    strs.into_iter()
        .map(|s| offsetdatetime_no_seconds::parse_str::<D>(&s))
        .collect()
}

fn deserialize_date_vec<'de, D>(d: D) -> Result<Vec<Date>, D::Error>
where
    D: Deserializer<'de>,
{
    let fmt = time::format_description::parse("[year]-[month]-[day]")
        .map_err(serde::de::Error::custom)?;
    let strs = Vec::<String>::deserialize(d)?;
    strs.into_iter()
        .map(|s| time::Date::parse(&s, &fmt).map_err(serde::de::Error::custom))
        .collect()
}

mod offsetdatetime_no_seconds {
    use serde::{Deserialize, Deserializer};
    use time::{OffsetDateTime, UtcOffset};

    const FORMAT: &str = "[year]-[month]-[day]T[hour]:[minute]";

    pub fn parse_str<'de, D: Deserializer<'de>>(s: &str) -> Result<OffsetDateTime, D::Error> {
        let fmt = time::format_description::parse(FORMAT).map_err(serde::de::Error::custom)?;
        let naive = time::PrimitiveDateTime::parse(s, &fmt).map_err(serde::de::Error::custom)?;
        Ok(naive.assume_offset(UtcOffset::UTC)) // assume UTC (adjust as needed)
    }

    pub fn deserialize<'de, D>(d: D) -> Result<OffsetDateTime, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(d)?;
        parse_str::<D>(&s)
    }
}

pub const fn weather_icon(code: u32) -> &'static str {
    match code {
        0 => "☀️",
        1 => "🌤️",
        2 => "⛅",
        3 => "☁️",
        45 => "🌫️",
        48 => "🌫️",
        51 => "🌦️",
        53 => "🌦️",
        55 => "🌦️",
        56 => "🌦️",
        57 => "🌦️",
        61 => "🌧️",
        63 => "🌧️",
        65 => "🌧️",
        66 => "🌧️",
        67 => "🌧️",
        71 => "🌨️",
        73 => "🌨️",
        75 => "🌨️",
        77 => "🌨️",
        80 => "🌧️",
        81 => "🌧️",
        82 => "🌧️",
        85 => "🌨️",
        86 => "🌨️",
        95 => "🌩️",
        96 => "🌩️",
        99 => "🌩️",
        _ => "❓",
    }
}

pub const fn weather_description(code: i32) -> &'static str {
    match code {
        0 => "Clear sky",
        1 => "Mainly clear",
        2 => "Partly cloudy",
        3 => "Overcast",
        45 => "Fog",
        48 => "Depositing rime fog",
        51 => "Light drizzle",
        53 => "Moderate drizzle",
        55 => "Dense intensity drizzle",
        56 => "Light freezing drizzle",
        57 => "Dense intensity freezing drizzle",
        61 => "Slight rain",
        63 => "Moderate rain",
        65 => "Heavy intensity rain",
        66 => "Light freezing rain",
        67 => "Heavy intensity freezing rain",
        71 => "Slight snow fall",
        73 => "Moderate snow fall",
        75 => "Heavy intensity snow fall",
        77 => "Snow grains",
        80 => "Slight rain showers",
        81 => "Moderate rain showers",
        82 => "Violent rain showers",
        85 => "Slight snow showers",
        86 => "Heavy snow showers",
        95 => "Slight or moderate thunderstorm",
        96 => "Thunderstorm with slight hail",
        99 => "Thunderstorm with heavy hail",
        _ => "Unknown weather condition",
    }
}
