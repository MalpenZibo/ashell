use crate::{
    components::{
        ButtonKind, ButtonSize, MenuSize,
        icons::{StaticIcon, icon_button},
        styled_button,
    },
    config::{TempoModuleConfig, WeatherIndicator, WeatherLocation},
    i18n::{UnitSystem, chrono_locale, language_subtag, unit_system},
    t,
    theme::{AshellTheme, use_theme},
};
use chrono::{
    DateTime, Datelike, Days, FixedOffset, Local, Months, NaiveDate, NaiveDateTime, TimeZone, Utc,
    Weekday,
};
use chrono_tz::Tz;
use iced::{
    Background, Border, Degrees, Element,
    Length::{self, FillPortion},
    Padding, Rotation, Subscription, Theme,
    alignment::{Horizontal, Vertical},
    core::svg::Handle,
    futures::SinkExt,
    stream::channel,
    widget::{Column, Row, Svg, column, container, row, scrollable, svg, text},
};
use itertools::izip;
use log::{debug, warn};
use serde::{Deserialize, Deserializer};
use std::time::Duration;

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
                // Reset indices if they would be out of bounds or if config is empty
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

    fn time_str(
        &'_ self,
        format: &str,
        timezone_index: usize,
        utc_datetime: Option<NaiveDateTime>,
    ) -> String {
        // %Z prints timezone abbreviations; other specifiers (e.g., %z/%:z) only need numeric offsets https://docs.rs/chrono/latest/chrono/format/strftime/index.html#fn6
        let format_requests_name = format.contains("%Z");
        let utc_now = self.date.with_timezone(&Utc);
        let naive_utc = utc_datetime.unwrap_or_else(|| utc_now.naive_utc());
        let locale = chrono_locale();

        self.config
            .timezones
            .get(timezone_index)
            .and_then(|tz_name| {
                if !format_requests_name && let Ok(offset) = tz_name.parse::<FixedOffset>() {
                    return Some(
                        offset
                            .from_utc_datetime(&naive_utc)
                            .format_localized(format, locale)
                            .to_string(),
                    );
                }

                if let Ok(tz) = tz_name.parse::<Tz>() {
                    return Some(
                        tz.from_utc_datetime(&naive_utc)
                            .format_localized(format, locale)
                            .to_string(),
                    );
                }

                None
            })
            .unwrap_or_else(|| {
                Local
                    .from_utc_datetime(&naive_utc)
                    .format_localized(format, locale)
                    .to_string()
            })
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

    fn naive_date(&'_ self, timezone_index: usize) -> NaiveDate {
        let utc_now = self.date.with_timezone(&Utc);

        self.config
            .timezones
            .get(timezone_index)
            .and_then(|tz_name| {
                if let Ok(offset) = tz_name.parse::<FixedOffset>() {
                    return Some(offset.from_utc_datetime(&utc_now.naive_utc()).date_naive());
                }

                if let Ok(tz) = tz_name.parse::<Tz>() {
                    return Some(tz.from_utc_datetime(&utc_now.naive_utc()).date_naive());
                }

                None
            })
            .unwrap_or_else(|| self.date.date_naive())
    }

    fn calendar<'a>(&'a self) -> Element<'a, Message> {
        use_theme(|theme| self.calendar_with_theme(theme))
    }

    fn calendar_with_theme<'a>(&'a self, theme: &AshellTheme) -> Element<'a, Message> {
        let locale = chrono_locale();
        let selected_date = self
            .selected_date
            .unwrap_or(self.naive_date(self.current_timezone_index));

        let current_month = selected_date.month0();
        let first_day_month = selected_date.with_day0(0).unwrap_or_default();
        let day_of_week_first_day = first_day_month.weekday();

        let mut current = first_day_month
            .checked_sub_days(Days::new(
                day_of_week_first_day.num_days_from_monday() as u64
            ))
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

        let calendar = column![
            row![
                icon_button::<Message>(StaticIcon::LeftChevron)
                    .kind(ButtonKind::Solid)
                    .on_press(Message::ChangeSelectDate(
                        selected_date.checked_sub_months(Months::new(1)),
                    )),
                text(selected_date.format_localized("%B", locale).to_string())
                    .size(theme.font_size.md)
                    .width(Length::Fill)
                    .align_x(Horizontal::Center),
                icon_button::<Message>(StaticIcon::RightChevron)
                    .kind(ButtonKind::Solid)
                    .on_press(Message::ChangeSelectDate(
                        selected_date.checked_add_months(Months::new(1))
                    ))
            ]
            .width(Length::Fill)
            .align_y(Vertical::Center),
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
                    text(
                        NaiveDate::from_isoywd_opt(2000, 20, i)
                            .expect("valid NaiveDate")
                            .format_localized("%a", locale)
                            .to_string(),
                    )
                    .align_x(Horizontal::Center)
                    .width(Length::Fill)
                    .into()
                })
                .collect::<Vec<Element<'a, Message>>>(),
            )
            .width(Length::Fill)
            .spacing(theme.space.sm),
            Column::with_children(
                (0..weeks_in_month)
                    .map(|_| {
                        Row::with_children(
                            (0..7)
                                .map(|_| {
                                    let day = current;
                                    current = current.succ_opt().unwrap_or(current);

                                    styled_button(Element::from(
                                        text(day.format_localized("%-d", locale).to_string())
                                            .align_x(Horizontal::Center)
                                            .color_maybe({
                                                if day
                                                    == self.naive_date(self.current_timezone_index)
                                                {
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
                                    ))
                                    .on_press_maybe(
                                        if day != self.naive_date(self.current_timezone_index) {
                                            Some(Message::ChangeSelectDate(Some(day)))
                                        } else {
                                            None
                                        },
                                    )
                                    .size(ButtonSize::Small)
                                    .width(Length::Fill)
                                    .into()
                                })
                                .collect::<Vec<Element<'a, Message>>>(),
                        )
                        .spacing(theme.space.xs)
                        .width(Length::Fill)
                        .into()
                    })
                    .collect::<Vec<Element<'a, Message>>>(),
            ),
        ]
        .spacing(theme.space.md);

        let timezones = Column::with_children(
            self.config
                .timezones
                .iter()
                .enumerate()
                .map(|(index, tz_name)| {
                    if self.current_timezone_index == index {
                        container(
                            text(format!(
                                "{}: {}",
                                tz_name,
                                self.time_str("%d %h %R", index, None)
                            ))
                            .wrapping(text::Wrapping::Word),
                        )
                        .padding([theme.space.xxs, theme.space.sm])
                        .width(Length::Fill)
                        .style(|theme: &Theme| container::Style {
                            text_color: Some(theme.palette().success),
                            ..Default::default()
                        })
                        .into()
                    } else {
                        styled_button(format!(
                            "{}: {}",
                            tz_name,
                            self.time_str("%d %h %R", index, None)
                        ))
                        .width(Length::Fill)
                        .on_press(Message::SetTimezone(index))
                        .into()
                    }
                })
                .collect::<Vec<Element<'a, Message>>>(),
        );

        column!(
            styled_button(Element::from(
                column!(
                    text(self.date.format_localized("%A", locale).to_string())
                        .size(theme.font_size.sm),
                    text(self.date.format_localized("%d %B %Y", locale).to_string())
                        .size(theme.font_size.md),
                )
                .spacing(theme.space.xs),
            ),)
            .size(ButtonSize::Large)
            .kind(ButtonKind::Outline)
            .on_press_maybe(if self.selected_date.is_some() {
                Some(Message::ChangeSelectDate(None))
            } else {
                None
            })
            .width(Length::Fill),
            calendar,
            timezones,
        )
        .spacing(theme.space.lg)
        .width(225)
        .into()
    }

    fn weather<'a>(&'a self) -> Option<Element<'a, Message>> {
        let (space, font_size, opacity, radius) =
            use_theme(|t| (t.space, t.font_size, t.opacity, t.radius));
        let locale = chrono_locale();
        let units = unit_system();
        let temp = units.temperature_symbol();
        let wind = units.wind_speed_symbol();
        self.weather_data
            .as_ref()
            .zip(self.location.as_ref())
            .map(|(data, location)| {
                column!(
                    container(
                        row!(
                            weather_icon(data.current.weather_code, data.current.is_day > 0)
                                .height(font_size.xxl)
                                .width(Length::Shrink),
                            column!(
                                text(format!(
                                    "{}{} - {}",
                                    location.city,
                                    if location.region_name.is_empty() {
                                        String::new()
                                    } else {
                                        format!(", {}", location.region_name)
                                    },
                                    self.time_str(
                                        "%R",
                                        self.current_timezone_index,
                                        Some(data.current.time)
                                    )
                                ))
                                .size(font_size.sm),
                                text(weather_description(data.current.weather_code)),
                                row!(
                                    text(format!("{}{temp}", data.current.temperature_2m)),
                                    text(t!(
                                        "tempo-feels-like",
                                        value = data.current.apparent_temperature,
                                        unit = temp,
                                    ))
                                    .size(font_size.sm)
                                )
                                .align_y(Vertical::Bottom)
                                .spacing(space.sm),
                            )
                            .width(FillPortion(2))
                            .spacing(space.xs),
                            column!(
                                row!(
                                    svg(Handle::from_memory(include_bytes!(
                                        "../../assets/weather_icon/drop.svg"
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
                                        "../../assets/weather_icon/wind.svg"
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
                                .map(|(time, weather_code, temp_value, is_day)| {
                                    column!(
                                        text(format!("{}{temp}", temp_value.round())),
                                        weather_icon(*weather_code, *is_day > 0)
                                            .height(font_size.md)
                                            .width(Length::Shrink),
                                        text(time.format("%H:%M").to_string()).size(font_size.sm)
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
                                        .width(Length::Fill),
                                        weather_icon(*weather_code, true)
                                            .height(font_size.md)
                                            .width(Length::Shrink),
                                        container(
                                            row!(
                                                text(format!(
                                                    "{}{temp}/{}{temp}",
                                                    temp_min.round(),
                                                    temp_max.round()
                                                ))
                                                .width(Length::Shrink),
                                                row!(
                                                    svg(Handle::from_memory(include_bytes!(
                                                        "../../assets/weather_icon/wind.svg"
                                                    )))
                                                    .height(font_size.md)
                                                    .width(Length::Shrink)
                                                    .rotation(Rotation::Floating(
                                                        Degrees(*wind_dir as f32 + 90.).into()
                                                    )),
                                                    text(format!("{} {wind}", wind_speed.round()))
                                                )
                                                .spacing(space.xxs)
                                            )
                                            .spacing(space.sm)
                                        )
                                        .width(Length::FillPortion(2))
                                        .align_x(Horizontal::Right)
                                    )
                                    .spacing(space.sm),
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

    pub fn subscription(&self) -> Subscription<Message> {
        let second_specifiers = [
            "%S",  // Seconds (00-60)
            "%T",  // Hour:Minute:Second
            "%X",  // Locale time representation with seconds
            "%r",  // 12-hour clock time with seconds
            "%:z", // UTC offset with seconds
            "%s",  // Unix timestamp (seconds since epoch)
        ];

        let current_format = self.current_format();

        let interval = if second_specifiers
            .iter()
            .any(|&spec| current_format.contains(spec))
        {
            Duration::from_secs(1)
        } else {
            Duration::from_secs(5)
        };

        let time_sub = Subscription::run_with(interval, |interval| {
            let interval = *interval;
            channel(
                100,
                async move |mut output: iced::futures::channel::mpsc::Sender<Message>| {
                    let mut interval = tokio::time::interval(interval);
                    loop {
                        interval.tick().await;
                        output.send(Message::Update).await.ok();
                    }
                },
            )
        });

        let weather_sub = self.config.weather_location.clone().map(|location| {
            let key = (location, unit_system(), language_subtag());
            Subscription::run_with(key, |(location, units, lang)| {
                let location = location.clone();
                let units = *units;
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
                            match fetch_weather_data(lat, lon, units).await {
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

async fn fetch_location(location: &WeatherLocation, lang: &str) -> anyhow::Result<Location> {
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

    // Nominatim requires a custom User-Agent header per their usage policy
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

            // Return city and country if both available and different
            if let Some(country) = address.get("country").and_then(|v| v.as_str())
                && let Some(city_name) = city
            {
                return Ok(Some((
                    city_name.to_string(),
                    if city_name != country {
                        country.to_string()
                    } else {
                        String::new() // Don't repeat city name as region i.e. Singapore, Singapore
                    },
                )));
            }

            // Return just the city if no country found
            if let Some(city_name) = city {
                return Ok(Some((city_name.to_string(), String::new())));
            }
        }
    }

    Ok(None)
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
        // Prefer country over admin1 if they're the same (avoids "Stockholm, Stockholm")
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
    latitude: f32,
    longitude: f32,
    city: String,
    region_name: String,
}

async fn fetch_weather_data(lat: f32, lon: f32, units: UnitSystem) -> anyhow::Result<WeatherData> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(20))
        .build()?;

    let (temp_param, wind_param) = match units {
        UnitSystem::Metric => ("celsius", "kmh"),
        UnitSystem::Imperial => ("fahrenheit", "mph"),
    };

    let response = client.get(format!(
        "https://api.open-meteo.com/v1/forecast?\
latitude={lat}&longitude={lon}\
&current=weather_code,apparent_temperature,relative_humidity_2m,temperature_2m,is_day,wind_speed_10m,wind_direction_10m\
&hourly=weather_code,temperature_2m,is_day\
&daily=weather_code,temperature_2m_max,temperature_2m_min,wind_speed_10m_max,wind_direction_10m_dominant\
&forecast_days=7\
&temperature_unit={temp_param}\
&wind_speed_unit={wind_param}"
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
