use crate::{
    components::icons::{StaticIcon, icon},
    config::{TempoModuleConfig, WeatherLocation},
    menu::MenuSize,
    theme::AshellTheme,
};
use chrono::{DateTime, Datelike, Days, Local, Months, NaiveDate, NaiveDateTime, Weekday};
use iced::{
    Background, Border, Degrees, Element,
    Length::{self, FillPortion},
    Rotation, Subscription, Theme,
    alignment::{Horizontal, Vertical},
    core::svg::Handle,
    futures::SinkExt,
    stream::channel,
    time::every,
    widget::{
        Column, Row, Svg, button, column, container, row, scrollable, scrollable::Scrollbar, svg,
        text,
    },
};
use itertools::izip;
use log::{debug, warn};
use serde::{Deserialize, Deserializer};
use std::{any::TypeId, time::Duration};

#[derive(Debug, Clone)]
pub enum Message {
    Update,
    ChangeSelectDate(Option<NaiveDate>),
    UpdateWeather(Box<WeatherData>),
    UpdateLocation(Location),
}

pub enum Action {
    None,
}

pub struct Tempo {
    config: TempoModuleConfig,
    date: DateTime<Local>,
    selected_date: Option<NaiveDate>,
    weather_data: Option<WeatherData>,
    location: Option<Location>,
}

impl Tempo {
    pub fn new(config: TempoModuleConfig) -> Self {
        Self {
            config,
            date: Local::now(),
            selected_date: None,
            weather_data: None,
            location: None,
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
            Message::UpdateLocation(location) => {
                self.location = Some(location);

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
            .align_y(Vertical::Center)
            .spacing(theme.space.sm)
            .into()
    }

    pub fn weather_indicator(&'_ self, theme: &AshellTheme) -> Option<Element<'_, Message>> {
        self.weather_data.as_ref().map(|data| {
            row!(
                weather_icon(data.current.weather_code, data.current.is_day > 0)
                    .width(Length::Fixed(theme.font_size.sm as f32)),
                text(format!("{}°C", data.current.temperature_2m))
                    .align_y(Vertical::Center)
                    .size(theme.font_size.sm)
            )
            .align_y(Vertical::Center)
            .spacing(theme.space.xxs)
            .into()
        })
    }

    pub fn menu_view<'a>(&'a self, theme: &'a AshellTheme) -> Element<'a, Message> {
        container(
            Row::new()
                .push(self.calendar(theme))
                .push_maybe(self.weather(theme))
                .spacing(theme.space.lg),
        )
        .max_width(MenuSize::XLarge)
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
                    text(self.date.format("%A").to_string()).size(theme.font_size.sm),
                    text(self.date.format("%d %B %Y").to_string()).size(theme.font_size.md),
                )
                .spacing(theme.space.xs)
            )
            .padding([theme.space.sm, theme.space.lg])
            .on_press_maybe(if self.selected_date.is_some() {
                Some(Message::ChangeSelectDate(None))
            } else {
                None
            })
            .style(theme.outline_button_style()),
            calendar
        )
        .spacing(theme.space.lg)
        .into()
    }

    fn weather<'a>(&'a self, theme: &'a AshellTheme) -> Option<Element<'a, Message>> {
        self.weather_data
            .as_ref()
            .zip(self.location.as_ref())
            .map(|(data, location)| {
                column!(
                    container(
                        row!(
                            weather_icon(data.current.weather_code, data.current.is_day > 0)
                                .height(theme.font_size.xxl)
                                .width(Length::Shrink),
                            column!(
                                text(format!(
                                    "{}, {} - {}",
                                    location.city,
                                    location.region_name,
                                    data.current.time.format("%R")
                                ))
                                .size(theme.font_size.sm),
                                text(weather_description(data.current.weather_code)),
                                row!(
                                    text(format!("{} °C", data.current.temperature_2m)),
                                    text(format!(
                                        "Feels like {}°C",
                                        data.current.apparent_temperature
                                    ))
                                    .size(theme.font_size.sm)
                                )
                                .align_y(Vertical::Bottom)
                                .spacing(theme.space.sm),
                            )
                            .width(FillPortion(2))
                            .spacing(theme.space.xs),
                            column!(
                                row!(
                                    svg(Handle::from_memory(include_bytes!(
                                        "../../assets/weather_icon/drop.svg"
                                    )))
                                    .width(Length::Shrink)
                                    .height(theme.font_size.lg),
                                    column!(
                                        text("Humidity")
                                            .size(theme.font_size.xs)
                                            .align_x(Horizontal::Right)
                                            .width(Length::Fill),
                                        text(format!("{}%", data.current.relative_humidity_2m))
                                            .align_x(Horizontal::Right)
                                            .size(theme.font_size.xs)
                                            .width(Length::Fill),
                                    )
                                    .spacing(theme.space.xxs)
                                )
                                .align_y(Vertical::Center)
                                .spacing(theme.space.sm),
                                row!(
                                    svg(Handle::from_memory(include_bytes!(
                                        "../../assets/weather_icon/wind.svg"
                                    )))
                                    .height(theme.font_size.lg)
                                    .width(Length::Shrink)
                                    .rotation(
                                        Rotation::Floating(
                                            Degrees(data.current.wind_direction_10m as f32 + 90.)
                                                .into()
                                        )
                                    ),
                                    column!(
                                        text("Wind")
                                            .size(theme.font_size.xs)
                                            .align_x(Horizontal::Right)
                                            .width(Length::Fill),
                                        text(format!("{} Km/h", data.current.wind_speed_10m))
                                            .align_x(Horizontal::Right)
                                            .size(theme.font_size.xs)
                                            .width(Length::Fill),
                                    )
                                    .spacing(theme.space.xxs)
                                )
                                .align_y(Vertical::Center)
                                .spacing(theme.space.sm),
                            )
                            .width(Length::Fill)
                            .spacing(theme.space.xs),
                        )
                        .spacing(theme.space.lg)
                        .align_y(Vertical::Center)
                        .width(Length::Fill),
                    )
                    .padding(theme.space.md)
                    .style(move |app_theme: &Theme| container::Style {
                        background: Background::Color(
                            app_theme
                                .extended_palette()
                                .secondary
                                .strong
                                .color
                                .scale_alpha(theme.opacity),
                        )
                        .into(),
                        border: Border::default().rounded(theme.radius.lg),
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
                                    .filter(|(_, t)| **t > self.date.naive_local())
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
                                .map(|(time, weather_code, temp, is_day)| {
                                    column!(
                                        text(format!("{}°", temp.round())),
                                        weather_icon(*weather_code, *is_day > 0)
                                            .height(theme.font_size.md)
                                            .width(Length::Shrink),
                                        text(time.format("%H:%M").to_string())
                                            .size(theme.font_size.sm)
                                    )
                                    .spacing(theme.space.xs)
                                    .align_x(Horizontal::Center)
                                    .into()
                                })
                                .collect::<Vec<_>>()
                            })
                            .padding([0, 0, theme.space.md, 0,])
                            .spacing(theme.space.sm)
                        )
                        .direction(scrollable::Direction::Horizontal(Scrollbar::new()))
                    )
                    .padding(theme.space.sm)
                    .style(move |app_theme: &Theme| container::Style {
                        background: Background::Color(
                            app_theme
                                .extended_palette()
                                .secondary
                                .strong
                                .color
                                .scale_alpha(theme.opacity),
                        )
                        .into(),
                        border: Border::default().rounded(theme.radius.lg),
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
                                container(
                                    row!(
                                        text(time.format("%a, %d %b").to_string())
                                            .width(Length::Fill),
                                        weather_icon(*weather_code, true)
                                            .height(theme.font_size.md)
                                            .width(Length::Shrink),
                                        container(
                                            row!(
                                                text(format!(
                                                    "{}°/{}°",
                                                    temp_min.round(),
                                                    temp_max.round()
                                                ))
                                                .width(Length::Shrink),
                                                row!(
                                                    svg(Handle::from_memory(include_bytes!(
                                                        "../../assets/weather_icon/wind.svg"
                                                    )))
                                                    .height(theme.font_size.md)
                                                    .width(Length::Shrink)
                                                    .rotation(Rotation::Floating(
                                                        Degrees(*wind_dir as f32 + 90.).into()
                                                    )),
                                                    text(format!("{} Km/h", wind_speed))
                                                )
                                                .spacing(theme.space.xxs)
                                            )
                                            .spacing(theme.space.sm)
                                        )
                                        .width(Length::FillPortion(2))
                                        .align_x(Horizontal::Right)
                                    )
                                    .spacing(theme.space.sm),
                                )
                                .padding(theme.space.sm)
                                .style(move |app_theme: &Theme| container::Style {
                                    background: Background::Color(
                                        app_theme
                                            .extended_palette()
                                            .secondary
                                            .strong
                                            .color
                                            .scale_alpha(theme.opacity),
                                    )
                                    .into(),
                                    border: Border::default().rounded([
                                        if index == 0 {
                                            theme.radius.lg
                                        } else {
                                            theme.radius.sm
                                        },
                                        if index == 0 {
                                            theme.radius.lg
                                        } else {
                                            theme.radius.sm
                                        },
                                        if index == data.daily.time.len() - 2 {
                                            theme.radius.lg
                                        } else {
                                            theme.radius.sm
                                        },
                                        if index == data.daily.time.len() - 2 {
                                            theme.radius.lg
                                        } else {
                                            theme.radius.sm
                                        },
                                    ]),
                                    ..container::Style::default()
                                })
                                .into()
                            }
                        )
                    )
                    .spacing(theme.space.xxs)
                )
                .spacing(theme.space.sm)
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

        let weather_sub = self.config.weather_location.clone().map(|location| {
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

                            let (lat, lon) = (loc.latitude, loc.longitude);
                            output.send(Message::UpdateLocation(loc)).await.ok();

                            loop {
                                let data = fetch_weather_data(lat, lon).await;

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
            )
        });

        if let Some(weather_sub) = weather_sub {
            Subscription::batch(vec![every(interval).map(|_| Message::Update), weather_sub])
        } else {
            every(interval).map(|_| Message::Update)
        }
    }
}

async fn fetch_location(location: WeatherLocation) -> anyhow::Result<Location> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()?;

    match location {
        WeatherLocation::City(city) => {
            let url = format!(
                "https://geocoding-api.open-meteo.com/v1/search?name={}&count=1&language=en&format=json",
                city
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
        Location {
            latitude: value.latitude,
            longitude: value.longitude,
            city: value.name,
            region_name: value.admin1.or(value.country).unwrap_or_default(),
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
    latitude: f32,
    longitude: f32,
    city: String,
    region_name: String,
}

async fn fetch_weather_data(lat: f32, lon: f32) -> anyhow::Result<WeatherData> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()?;

    let response = client.get(format!(
        "https://api.open-meteo.com/v1/forecast?\
        latitude={}&longitude={}\
        &current=weather_code,apparent_temperature,relative_humidity_2m,temperature_2m,is_day,wind_speed_10m,wind_direction_10m\
        &hourly=weather_code,temperature_2m,is_day\
        &daily=weather_code,temperature_2m_max,temperature_2m_min,wind_speed_10m_max,wind_direction_10m_dominant\
        &forecast_days=7", 
        lat, lon
    )).send().await?;
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
    time: NaiveDateTime,
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
    time: Vec<NaiveDateTime>,
    weather_code: Vec<u32>,
    temperature_2m: Vec<f32>,
    is_day: Vec<u8>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct DailyWeatherData {
    #[serde(deserialize_with = "deserialize_date_vec")]
    time: Vec<NaiveDate>,
    weather_code: Vec<u32>,
    temperature_2m_max: Vec<f32>,
    temperature_2m_min: Vec<f32>,
    wind_speed_10m_max: Vec<f32>,
    wind_direction_10m_dominant: Vec<u32>,
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
            "../../assets/weather_icon/clear-day.svg"
        ))),
        (0, false) => svg(Handle::from_memory(include_bytes!(
            "../../assets/weather_icon/clear-night.svg"
        ))),
        (1, true) => svg(Handle::from_memory(include_bytes!(
            "../../assets/weather_icon/cloudy-1-day.svg"
        ))),
        (1, false) => svg(Handle::from_memory(include_bytes!(
            "../../assets/weather_icon/cloudy-1-night.svg"
        ))),
        (2, true) => svg(Handle::from_memory(include_bytes!(
            "../../assets/weather_icon/cloudy-3-day.svg"
        ))),
        (2, false) => svg(Handle::from_memory(include_bytes!(
            "../../assets/weather_icon/cloudy-3-night.svg"
        ))),
        (3, _) => svg(Handle::from_memory(include_bytes!(
            "../../assets/weather_icon/cloudy.svg"
        ))),
        (45, _) | (48, _) => svg(Handle::from_memory(include_bytes!(
            "../../assets/weather_icon/fog.svg"
        ))),
        (51, true) => svg(Handle::from_memory(include_bytes!(
            "../../assets/weather_icon/rainy-1-day.svg"
        ))),
        (51, false) => svg(Handle::from_memory(include_bytes!(
            "../../assets/weather_icon/rainy-1-night.svg"
        ))),
        (53, true) | (56, true) => svg(Handle::from_memory(include_bytes!(
            "../../assets/weather_icon/rainy-2-day.svg"
        ))),
        (53, false) | (56, false) => svg(Handle::from_memory(include_bytes!(
            "../../assets/weather_icon/rainy-2-night.svg"
        ))),
        (55, true) | (57, true) => svg(Handle::from_memory(include_bytes!(
            "../../assets/weather_icon/rainy-3-day.svg"
        ))),
        (55, false) | (57, false) => svg(Handle::from_memory(include_bytes!(
            "../../assets/weather_icon/rainy-3-night.svg"
        ))),
        (61, _) => svg(Handle::from_memory(include_bytes!(
            "../../assets/weather_icon/rainy-1.svg"
        ))),
        (63, _) | (66, _) => svg(Handle::from_memory(include_bytes!(
            "../../assets/weather_icon/rainy-2.svg"
        ))),
        (65, _) | (67, _) => svg(Handle::from_memory(include_bytes!(
            "../../assets/weather_icon/rainy-3.svg"
        ))),
        (71, _) | (77, _) => svg(Handle::from_memory(include_bytes!(
            "../../assets/weather_icon/snowy-1.svg"
        ))),
        (73, _) => svg(Handle::from_memory(include_bytes!(
            "../../assets/weather_icon/snowy-2.svg"
        ))),
        (75, _) => svg(Handle::from_memory(include_bytes!(
            "../../assets/weather_icon/snowy-3.svg"
        ))),
        (80, true) => svg(Handle::from_memory(include_bytes!(
            "../../assets/weather_icon/showers-rainy-1-day.svg"
        ))),
        (80, false) => svg(Handle::from_memory(include_bytes!(
            "../../assets/weather_icon/showers-rainy-1-night.svg"
        ))),
        (81, true) => svg(Handle::from_memory(include_bytes!(
            "../../assets/weather_icon/showers-rainy-2-day.svg"
        ))),
        (81, false) => svg(Handle::from_memory(include_bytes!(
            "../../assets/weather_icon/showers-rainy-2-night.svg"
        ))),
        (82, true) => svg(Handle::from_memory(include_bytes!(
            "../../assets/weather_icon/showers-rainy-3-day.svg"
        ))),
        (82, false) => svg(Handle::from_memory(include_bytes!(
            "../../assets/weather_icon/showers-rainy-3-night.svg"
        ))),
        (85, true) => svg(Handle::from_memory(include_bytes!(
            "../../assets/weather_icon/snowy-2-day.svg"
        ))),
        (85, false) => svg(Handle::from_memory(include_bytes!(
            "../../assets/weather_icon/snowy-2-night.svg"
        ))),
        (86, true) => svg(Handle::from_memory(include_bytes!(
            "../../assets/weather_icon/snowy-3-day.svg"
        ))),
        (86, false) => svg(Handle::from_memory(include_bytes!(
            "../../assets/weather_icon/snowy-3-night.svg"
        ))),
        (95, _) => svg(Handle::from_memory(include_bytes!(
            "../../assets/weather_icon/isolated-thunderstorms.svg"
        ))),
        (96, _) => svg(Handle::from_memory(include_bytes!(
            "../../assets/weather_icon/scattered-thunderstorms.svg"
        ))),
        (99, _) => svg(Handle::from_memory(include_bytes!(
            "../../assets/weather_icon/severe-thunderstorm.svg"
        ))),
        _ => svg(Handle::from_memory(include_bytes!(
            "../../assets/weather_icon/unknown.svg"
        ))),
    }
}

pub const fn weather_description(code: u32) -> &'static str {
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
