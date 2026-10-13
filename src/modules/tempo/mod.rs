pub mod calendar;
pub mod weather;

use std::time::Duration;

use chrono::{DateTime, Local, NaiveDate};
use iced::{
    Element, Length, Row, Subscription,
    alignment::Vertical,
    futures::SinkExt,
    stream::channel,
    widget::{container, text},
};
use log::{debug, warn};

use self::weather::{Location, WeatherData, fetch_location, fetch_weather_data};
use crate::{
    components::MenuSize,
    config::{TempoModuleConfig, WeatherIndicator},
    i18n::{language_subtag, unit_system},
    theme::use_theme,
};

pub use self::weather::weather_icon;

/// Specifiers whose output changes at least once per second, so a format
/// containing any of them has to be refreshed every second.
const SECOND_SPECIFIERS: [&str; 15] = [
    "%S",  // seconds
    "%T",  // %H:%M:%S
    "%X",  // locale time representation, with seconds
    "%r",  // 12-hour clock time, with seconds
    "%c",  // locale date and time, with seconds
    "%+",  // ISO 8601 / RFC 3339, with seconds
    "%s",  // seconds since epoch
    "%f",  // nanoseconds, also covers ".%f"
    "%.f", // nanoseconds, left aligned
    "%3f", // milliseconds
    "%6f", // microseconds
    "%9f", // nanoseconds
    "%.3f", "%.6f", "%.9f", // the left aligned fractional variants
];

fn shows_seconds(format: &str) -> bool {
    let mut rest = format;

    while let Some(percent) = rest.find('%') {
        let after = &rest[percent + 1..];

        // `%%` is a literal percent sign, not the start of a specifier.
        if let Some(literal) = after.strip_prefix('%') {
            rest = literal;
            continue;
        }

        // skip padding flags
        let specifier = after.trim_start_matches(['-', '_', '0']);

        if SECOND_SPECIFIERS
            .iter()
            .any(|candidate| specifier.starts_with(&candidate[1..]))
        {
            return true;
        }

        rest = specifier;
    }

    false
}

/// Local wall-clock minute, so a system time zone change also triggers a redraw.
fn local_minute(now: DateTime<Local>) -> i64 {
    now.naive_local().and_utc().timestamp().div_euclid(60)
}

#[derive(Debug, Clone)]
pub enum Message {
    Update,
    ChangeSelectDate(Option<NaiveDate>),
    UpdateWeather(Box<WeatherData>),
    UpdateLocation(Location),
    CycleFormat,
    CycleTimezone(TimezoneDirection),
    SetTimezone(usize),
    ConfigReloaded(TempoModuleConfig),
    ToggleLocationVisibility,
}

#[derive(Debug, Clone, Copy)]
pub enum TimezoneDirection {
    Forward,
    Backward,
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
    current_format_index: usize,
    current_timezone_index: usize,
    location_visible: bool,
}

impl Tempo {
    pub fn new(config: TempoModuleConfig) -> Self {
        Self {
            config,
            date: Local::now(),
            selected_date: None,
            weather_data: None,
            location: None,
            current_format_index: 0,
            current_timezone_index: 0,
            location_visible: true,
        }
    }

    fn current_format(&self) -> &str {
        if !self.config.formats.is_empty() {
            self.config
                .formats
                .get(self.current_format_index)
                .or_else(|| self.config.formats.first())
                .unwrap_or(&self.config.clock_format)
        } else {
            &self.config.clock_format
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
            Message::CycleFormat => {
                if !self.config.formats.is_empty() {
                    self.current_format_index =
                        (self.current_format_index + 1) % self.config.formats.len();
                }
                Action::None
            }
            Message::CycleTimezone(direction) => {
                if !self.config.timezones.is_empty() {
                    let len = self.config.timezones.len();
                    self.current_timezone_index = match direction {
                        TimezoneDirection::Forward => (self.current_timezone_index + 1) % len,
                        TimezoneDirection::Backward => self
                            .current_timezone_index
                            .checked_sub(1)
                            .unwrap_or(len - 1),
                    };
                }
                Action::None
            }
            Message::SetTimezone(index) => {
                if !self.config.timezones.is_empty() {
                    let len = self.config.timezones.len();
                    if index < len {
                        self.current_timezone_index = index;
                    }
                }
                Action::None
            }
            Message::ConfigReloaded(new_config) => {
                if new_config.formats.is_empty()
                    || self.current_format_index >= new_config.formats.len()
                {
                    self.current_format_index = 0;
                }

                if new_config.timezones.is_empty()
                    || self.current_timezone_index >= new_config.timezones.len()
                {
                    self.current_timezone_index = 0;
                }

                let location_changed = self.config.weather_location != new_config.weather_location;

                if location_changed {
                    self.weather_data = None;
                    self.location = None;
                }

                self.config = new_config;
                Action::None
            }
            Message::ToggleLocationVisibility => {
                self.location_visible = !self.location_visible;
                Action::None
            }
        }
    }

    pub fn view(&'_ self) -> Element<'_, Message> {
        let space = use_theme(|t| t.space);
        let display_text = self.time_str(self.current_format(), self.current_timezone_index, None);

        Row::with_capacity(2)
            .push(self.weather_indicator())
            .push(text(display_text))
            .align_y(Vertical::Center)
            .spacing(space.sm)
            .into()
    }

    pub fn weather_indicator(&'_ self) -> Option<Element<'_, Message>> {
        let (font_size, space) = use_theme(|t| (t.font_size, t.space));
        if self.config.weather_location.is_none()
            || self.config.weather_indicator == WeatherIndicator::None
        {
            return None;
        }
        let temp = unit_system().temperature_symbol();
        self.weather_data
            .as_ref()
            .zip(self.location.as_ref())
            .map(|(data, _)| {
                Row::new()
                    .push(
                        weather_icon(data.current.weather_code, data.current.is_day > 0)
                            .width(Length::Fixed(font_size.sm)),
                    )
                    .push(
                        (self.config.weather_indicator == WeatherIndicator::IconAndTemperature)
                            .then(|| {
                                text(format!("{}{temp}", data.current.temperature_2m))
                                    .align_y(Vertical::Center)
                                    .size(font_size.sm)
                            }),
                    )
                    .align_y(Vertical::Center)
                    .spacing(space.xxs)
                    .into()
            })
    }

    pub fn menu_view<'a>(&'a self) -> Element<'a, Message> {
        let space = use_theme(|t| t.space);
        container(
            Row::with_capacity(2)
                .push(self.calendar())
                .push(self.weather())
                .spacing(space.lg),
        )
        .max_width(MenuSize::XLarge)
        .into()
    }

    pub fn subscription(&self) -> Subscription<Message> {
        let shows_seconds = shows_seconds(self.current_format());

        let time_sub = Subscription::run_with(shows_seconds, |&shows_seconds| {
            channel(
                100,
                async move |mut output: iced::futures::channel::mpsc::Sender<Message>| {
                    let period = if shows_seconds { 1 } else { 5 };
                    let mut interval = tokio::time::interval(Duration::from_secs(period));
                    let mut last_minute = None;
                    loop {
                        interval.tick().await;

                        if !shows_seconds {
                            let minute = local_minute(Local::now());
                            if last_minute == Some(minute) {
                                continue;
                            }
                            last_minute = Some(minute);
                        }

                        output.send(Message::Update).await.ok();
                    }
                },
            )
        });

        let weather_sub = self.config.weather_location.clone().map(|location| {
            let wind_unit = self.config.resolved_wind_speed_unit();
            let key = (location, unit_system(), wind_unit, language_subtag());
            Subscription::run_with(key, |(location, units, wind_unit, lang)| {
                let location = location.clone();
                let units = *units;
                let wind_unit = *wind_unit;
                let lang = lang.clone();
                channel(100, async move |mut output| {
                    let mut failed_attempt: u64 = 0;

                    loop {
                        let loc = match fetch_location(&location, &lang).await {
                            Ok(loc) => {
                                debug!("Location fetched successfully: {:?}", loc);
                                let (lat, lon) = (loc.latitude, loc.longitude);
                                output.send(Message::UpdateLocation(loc)).await.ok();
                                Some((lat, lon))
                            }
                            Err(e) => {
                                warn!("Failed to fetch location: {:?}", e);
                                None
                            }
                        };

                        if let Some((lat, lon)) = loc {
                            match fetch_weather_data(lat, lon, units, wind_unit).await {
                                Ok(weather_data) => {
                                    failed_attempt = 0;
                                    debug!("Weather data fetched successfully: {:?}", weather_data);
                                    output
                                        .send(Message::UpdateWeather(Box::new(weather_data)))
                                        .await
                                        .ok();

                                    tokio::time::sleep(Duration::from_secs(60 * 30)).await;
                                    continue;
                                }
                                Err(e) => {
                                    warn!("Failed to fetch weather data: {:?}", e);
                                }
                            }
                        }

                        failed_attempt += 1;
                        tokio::time::sleep(Duration::from_secs(60 * failed_attempt)).await;
                    }
                })
            })
        });

        if let Some(weather_sub) = weather_sub {
            Subscription::batch(vec![time_sub, weather_sub])
        } else {
            time_sub
        }
    }
}
