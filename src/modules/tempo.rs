use crate::{
    components::{
        ButtonKind, ButtonSize, MenuSize,
        icons::{StaticIcon, icon_button},
        styled_button,
    },
    config::{
        TempoCalendarSource, TempoCalendarType, TempoModuleConfig, WeatherIndicator,
        WeatherLocation,
    },
    theme::AshellTheme,
};
use chrono::{
    DateTime, Datelike, Days, FixedOffset, Local, Months, NaiveDate, NaiveDateTime, TimeZone, Utc,
    Weekday,
};
use chrono_tz::Tz;
use hex_color::HexColor;
use iced::{
    Background, Border, Color, Degrees, Element,
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
use std::collections::HashSet;
use std::str::FromStr;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

#[derive(Debug, Clone)]
pub enum Message {
    Update,
    ChangeSelectDate(Option<NaiveDate>),
    UpdateWeather(Box<WeatherData>),
    UpdateLocation(Location),
    UpdateCalendarEvents(Vec<CalendarEvent>),
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

#[derive(Debug, Clone)]
pub(crate) struct CalendarEvent {
    title: String,
    start: DateTime<Local>,
    end: DateTime<Local>,
    color: Option<String>,
}

#[derive(Clone)]
struct CalendarCacheEntry {
    events: Vec<CalendarEvent>,
    updated_at: std::time::Instant,
}

static CALENDAR_CACHE: OnceLock<Arc<Mutex<std::collections::HashMap<String, CalendarCacheEntry>>>> =
    OnceLock::new();

pub struct Tempo {
    config: TempoModuleConfig,
    date: DateTime<Local>,
    selected_date: Option<NaiveDate>,
    weather_data: Option<WeatherData>,
    location: Option<Location>,
    calendar_events: Vec<CalendarEvent>,
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
            calendar_events: vec![],
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
            Message::UpdateCalendarEvents(events) => {
                self.calendar_events = events;

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
                self.calendar_events.clear();
                Action::None
            }
        }
    }

    pub fn view(&'_ self, theme: &AshellTheme) -> Element<'_, Message> {
        let display_text = self.time_str(self.current_format(), self.current_timezone_index);

        Row::with_capacity(2)
            .push(self.weather_indicator(theme))
            .push(text(display_text))
            .align_y(Vertical::Center)
            .spacing(theme.space.sm)
            .into()
    }

    fn time_str(&'_ self, format: &str, timezone_index: usize) -> String {
        // %Z prints timezone abbreviations; other specifiers (e.g., %z/%:z) only need numeric offsets https://docs.rs/chrono/latest/chrono/format/strftime/index.html#fn6
        let format_requests_name = format.contains("%Z");
        let utc_now = self.date.with_timezone(&Utc);

        self.config
            .timezones
            .get(timezone_index)
            .and_then(|tz_name| {
                if !format_requests_name && let Ok(offset) = tz_name.parse::<FixedOffset>() {
                    return Some(
                        offset
                            .from_utc_datetime(&utc_now.naive_utc())
                            .format_localized(format, self.config.locale)
                            .to_string(),
                    );
                }

                if let Ok(tz) = tz_name.parse::<Tz>() {
                    return Some(
                        tz.from_utc_datetime(&utc_now.naive_utc())
                            .format_localized(format, self.config.locale)
                            .to_string(),
                    );
                }

                None
            })
            .unwrap_or_else(|| {
                self.date
                    .format_localized(format, self.config.locale)
                    .to_string()
            })
    }

    pub fn weather_indicator(&'_ self, theme: &AshellTheme) -> Option<Element<'_, Message>> {
        if self.config.weather_location.is_none()
            || self.config.weather_indicator == WeatherIndicator::None
        {
            return None;
        }
        self.weather_data
            .as_ref()
            .zip(self.location.as_ref())
            .map(|(data, _)| {
                Row::new()
                    .push(
                        weather_icon(data.current.weather_code, data.current.is_day > 0)
                            .width(Length::Fixed(theme.font_size.sm)),
                    )
                    .push(
                        (self.config.weather_indicator == WeatherIndicator::IconAndTemperature)
                            .then(|| {
                                text(format!("{}°C", data.current.temperature_2m))
                                    .align_y(Vertical::Center)
                                    .size(theme.font_size.sm)
                            }),
                    )
                    .align_y(Vertical::Center)
                    .spacing(theme.space.xxs)
                    .into()
            })
    }

    pub fn menu_view<'a>(&'a self, theme: &'a AshellTheme) -> Element<'a, Message> {
        container(
            Row::with_capacity(2)
                .push(self.calendar_panel(theme))
                .push(self.weather(theme))
                .spacing(theme.space.lg),
        )
        .max_width(MenuSize::XLarge)
        .into()
    }

    fn calendar_panel<'a>(&'a self, theme: &'a AshellTheme) -> Element<'a, Message> {
        let content = if self.config.calendar_type == TempoCalendarType::Calendar {
            column!(
                self.calendar_header(theme, false),
                self.calendar(theme),
                self.timezones(theme)
            )
        } else {
            column!(
                self.calendar_header(theme, true),
                self.events_view(theme),
                self.timezones(theme)
            )
        };

        content.spacing(theme.space.lg).width(225).into()
    }

    fn calendar_header<'a>(
        &'a self,
        theme: &'a AshellTheme,
        events_mode: bool,
    ) -> Element<'a, Message> {
        let date = if events_mode {
            self.selected_date
                .unwrap_or_else(|| self.naive_date(self.current_timezone_index))
        } else {
            self.date.date_naive()
        };

        let content = if events_mode {
            column!(
                text(
                    date.format_localized("%a, %d %b %Y", self.config.locale)
                        .to_string()
                )
                .align_x(Horizontal::Center)
                .wrapping(text::Wrapping::None)
                .size(theme.font_size.sm),
            )
        } else {
            column!(
                text(date.format_localized("%A", self.config.locale).to_string())
                    .size(theme.font_size.sm),
                text(
                    date.format_localized("%d %B %Y", self.config.locale)
                        .to_string()
                )
                .size(theme.font_size.md),
            )
            .spacing(theme.space.xs)
        };

        if events_mode {
            Row::with_capacity(3)
                .push(
                    container(
                        icon_button::<Message>(theme, StaticIcon::LeftChevron)
                            .kind(ButtonKind::Solid)
                            .on_press(Message::ChangeSelectDate(Some(
                                date - chrono::Duration::days(1),
                            ))),
                    )
                    .width(Length::Shrink),
                )
                .push(
                    container(
                        styled_button(theme, Element::from(content))
                            .size(ButtonSize::Large)
                            .kind(ButtonKind::Outline)
                            .width(Length::Fixed(145.0))
                            .on_press_maybe(if self.selected_date.is_some() {
                                Some(Message::ChangeSelectDate(None))
                            } else {
                                None
                            }),
                    )
                    .width(Length::Fill)
                    .center_x(Length::Fill),
                )
                .push(
                    container(
                        icon_button::<Message>(theme, StaticIcon::RightChevron)
                            .kind(ButtonKind::Solid)
                            .on_press(Message::ChangeSelectDate(Some(
                                date + chrono::Duration::days(1),
                            ))),
                    )
                    .width(Length::Shrink),
                )
                .spacing(theme.space.xs)
                .align_y(Vertical::Center)
                .into()
        } else {
            styled_button(theme, Element::from(content))
                .size(ButtonSize::Large)
                .kind(ButtonKind::Outline)
                .on_press_maybe(if self.selected_date.is_some() {
                    Some(Message::ChangeSelectDate(None))
                } else {
                    None
                })
                .width(Length::Fill)
                .into()
        }
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

    fn calendar<'a>(&'a self, theme: &'a AshellTheme) -> Element<'a, Message> {
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
                icon_button::<Message>(theme, StaticIcon::LeftChevron)
                    .kind(ButtonKind::Solid)
                    .on_press(Message::ChangeSelectDate(
                        selected_date.checked_sub_months(Months::new(1)),
                    )),
                text(
                    selected_date
                        .format_localized("%B", self.config.locale)
                        .to_string()
                )
                .size(theme.font_size.md)
                .width(Length::Fill)
                .align_x(Horizontal::Center),
                icon_button::<Message>(theme, StaticIcon::RightChevron)
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
                            .format_localized("%a", self.config.locale)
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
                                    let event_count = self.events_on_day(day);

                                    styled_button(
                                        theme,
                                        Element::from(
                                            column!(
                                                text(
                                                    day.format_localized("%d", self.config.locale)
                                                        .to_string(),
                                                )
                                                .align_x(Horizontal::Center)
                                                .color_maybe({
                                                    if day
                                                        == self
                                                            .naive_date(self.current_timezone_index)
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
                                                (event_count > 0).then(|| {
                                                    text("•")
                                                        .align_x(Horizontal::Center)
                                                        .size(theme.font_size.xs)
                                                        .color(theme.iced_theme.palette().primary)
                                                }),
                                            )
                                            .spacing(theme.space.xxs),
                                        ),
                                    )
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

        let timezones = self.timezones(theme);

        column!(
            styled_button(
                theme,
                Element::from(
                    column!(
                        text(
                            self.date
                                .format_localized("%A", self.config.locale)
                                .to_string()
                        )
                        .size(theme.font_size.sm),
                        text(
                            self.date
                                .format_localized("%d %B %Y", self.config.locale)
                                .to_string()
                        )
                        .size(theme.font_size.md),
                    )
                    .spacing(theme.space.xs),
                ),
            )
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

    fn timezones<'a>(&'a self, theme: &'a AshellTheme) -> Column<'a, Message> {
        Column::with_children(
            self.config
                .timezones
                .iter()
                .enumerate()
                .map(|(index, tz_name)| {
                    if self.current_timezone_index == index {
                        container(
                            text(format!("{}: {}", tz_name, self.time_str("%d %h %R", index)))
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
                        styled_button(
                            theme,
                            format!("{}: {}", tz_name, self.time_str("%d %h %R", index)),
                        )
                        .width(Length::Fill)
                        .on_press(Message::SetTimezone(index))
                        .into()
                    }
                })
                .collect::<Vec<Element<'a, Message>>>(),
        )
    }

    fn events_view<'a>(&'a self, theme: &'a AshellTheme) -> Element<'a, Message> {
        let events = if self.config.calendars.is_empty() {
            self.mock_events()
        } else {
            self.calendar_events.clone()
        };
        let selected_day = self
            .selected_date
            .unwrap_or_else(|| self.naive_date(self.current_timezone_index));
        let today_start = selected_day.and_hms_opt(0, 0, 0).unwrap_or_default();
        let tomorrow_start = today_start + chrono::Duration::days(1);
        let event_opacity = theme.opacity;
        Column::with_children(
            events
                .into_iter()
                .filter(|event| {
                    event.start.naive_local() < tomorrow_start
                        && event.end.naive_local() >= today_start
                })
                .map(|event| {
                    let card_opacity = if event.end.naive_local() < self.date.naive_local() {
                        event_opacity * 0.35
                    } else {
                        event_opacity
                    };
                    let background = event
                        .color
                        .as_deref()
                        .and_then(|color| HexColor::from_str(color).ok())
                        .map(|color| Color::from_rgb8(color.r, color.g, color.b))
                        .unwrap_or_else(|| {
                            theme.iced_theme.extended_palette().background.weak.color
                        });

                    container(
                        column!(
                            text(event.title).size(theme.font_size.sm),
                            text(format!(
                                "{} - {}",
                                event.start.format("%R"),
                                event.end.format("%R")
                            ))
                            .size(theme.font_size.xs),
                        )
                        .spacing(theme.space.xxs),
                    )
                    .padding(theme.space.sm)
                    .width(Length::Fill)
                    .style(move |_theme: &Theme| container::Style {
                        background: Background::Color(background.scale_alpha(card_opacity)).into(),
                        border: Border::default().rounded(theme.radius.sm),
                        ..Default::default()
                    })
                    .into()
                })
                .collect::<Vec<Element<'a, Message>>>(),
        )
        .spacing(theme.space.xs)
        .into()
    }

    fn mock_events(&self) -> Vec<CalendarEvent> {
        let day = self.date.date_naive();
        vec![
            CalendarEvent {
                title: "Team sync".to_string(),
                start: Local
                    .from_local_datetime(&day.and_hms_opt(9, 30, 0).unwrap_or_default())
                    .single()
                    .unwrap_or(self.date),
                end: Local
                    .from_local_datetime(&day.and_hms_opt(10, 0, 0).unwrap_or_default())
                    .single()
                    .unwrap_or(self.date),
                color: None,
            },
            CalendarEvent {
                title: "Release prep".to_string(),
                start: Local
                    .from_local_datetime(&day.and_hms_opt(13, 0, 0).unwrap_or_default())
                    .single()
                    .unwrap_or(self.date),
                end: Local
                    .from_local_datetime(&day.and_hms_opt(14, 0, 0).unwrap_or_default())
                    .single()
                    .unwrap_or(self.date),
                color: None,
            },
            CalendarEvent {
                title: "Design review".to_string(),
                start: Local
                    .from_local_datetime(&day.and_hms_opt(15, 30, 0).unwrap_or_default())
                    .single()
                    .unwrap_or(self.date),
                end: Local
                    .from_local_datetime(&day.and_hms_opt(16, 15, 0).unwrap_or_default())
                    .single()
                    .unwrap_or(self.date),
                color: None,
            },
        ]
    }

    fn events_on_day(&self, day: NaiveDate) -> usize {
        self.calendar_events
            .iter()
            .filter(|event| {
                let start = event.start.naive_local().date();
                let end = event.end.naive_local().date();
                start <= day && end >= day
            })
            .count()
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
                                    "{}{} - {}",
                                    location.city,
                                    if location.region_name.is_empty() {
                                        String::new()
                                    } else {
                                        format!(", {}", location.region_name)
                                    },
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
                                        text(format!("{} km/h", data.current.wind_speed_10m))
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
                                .background
                                .weak
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
                            .spacing(theme.space.sm)
                            .padding(Padding::default().bottom(theme.space.sm))
                        )
                        .horizontal()
                    )
                    .padding(theme.space.sm)
                    .style(move |app_theme: &Theme| container::Style {
                        background: Background::Color(
                            app_theme
                                .extended_palette()
                                .background
                                .weak
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
                                        text(
                                            time.format_localized("%a, %d %b", self.config.locale)
                                                .to_string()
                                        )
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
                                                    text(format!("{} km/h", wind_speed))
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
                                            .background
                                            .weak
                                            .color
                                            .scale_alpha(theme.opacity),
                                    )
                                    .into(),
                                    border: Border::default().rounded(iced::border::Radius {
                                        top_left: if index == 0 {
                                            theme.radius.lg
                                        } else {
                                            theme.radius.sm
                                        },
                                        top_right: if index == 0 {
                                            theme.radius.lg
                                        } else {
                                            theme.radius.sm
                                        },
                                        bottom_right: if index == data.daily.time.len() - 2 {
                                            theme.radius.lg
                                        } else {
                                            theme.radius.sm
                                        },
                                        bottom_left: if index == data.daily.time.len() - 2 {
                                            theme.radius.lg
                                        } else {
                                            theme.radius.sm
                                        },
                                    }),
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
            Subscription::run_with(location, |location| {
                let location = location.clone();
                channel(100, async move |mut output| {
                    let mut failed_attempt: u64 = 0;

                    loop {
                        let loc = match fetch_location(&location).await {
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
                            match fetch_weather_data(lat, lon).await {
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

        let calendars_sub = if self.config.calendars.is_empty() {
            None
        } else {
            let calendars = self.config.calendars.clone();
            Some(Subscription::run_with(calendars, |calendars| {
                let calendars = calendars.clone();
                channel(100, async move |mut output| {
                    loop {
                        let mut events = Vec::new();
                        for calendar in &calendars {
                            match fetch_calendar_events_cached(calendar).await {
                                Ok(mut items) => events.append(&mut items),
                                Err(e) => warn!("Failed to fetch calendar: {:?}", e),
                            }
                        }

                        events.sort_by_key(|event| event.start);
                        let _ = output.send(Message::UpdateCalendarEvents(events)).await;
                        tokio::time::sleep(Duration::from_secs(60 * 10)).await;
                    }
                })
            }))
        };

        let mut subscriptions = vec![time_sub];
        if let Some(weather_sub) = weather_sub {
            subscriptions.push(weather_sub);
        }
        if let Some(calendars_sub) = calendars_sub {
            subscriptions.push(calendars_sub);
        }

        if subscriptions.len() > 1 {
            Subscription::batch(subscriptions)
        } else {
            subscriptions.into_iter().next().unwrap()
        }
    }
}

async fn fetch_calendar_events(source: &TempoCalendarSource) -> anyhow::Result<Vec<CalendarEvent>> {
    let raw = match source {
        TempoCalendarSource::Url { url, .. } => {
            reqwest::Client::new().get(url).send().await?.text().await?
        }
        TempoCalendarSource::Path { path, .. } => {
            let expanded = shellexpand::tilde(path);
            std::fs::read_to_string(expanded.as_ref())?
        }
    };

    Ok(parse_ics_events(&raw, source))
}

async fn fetch_calendar_events_cached(
    source: &TempoCalendarSource,
) -> anyhow::Result<Vec<CalendarEvent>> {
    let key = match source {
        TempoCalendarSource::Url { url, .. } => format!("url:{url}"),
        TempoCalendarSource::Path { path, .. } => format!("path:{path}"),
    };

    let cache = CALENDAR_CACHE
        .get_or_init(|| Arc::new(Mutex::new(std::collections::HashMap::new())))
        .clone();

    if let Some(entry) = cache.lock().ok().and_then(|m| m.get(&key).cloned())
        && entry.updated_at.elapsed() < Duration::from_secs(60 * 10)
    {
        return Ok(entry.events);
    }

    let events = fetch_calendar_events(source).await?;
    if let Ok(mut guard) = cache.lock() {
        guard.insert(
            key,
            CalendarCacheEntry {
                events: events.clone(),
                updated_at: std::time::Instant::now(),
            },
        );
    }

    Ok(events)
}

fn parse_ics_events(raw: &str, source: &TempoCalendarSource) -> Vec<CalendarEvent> {
    let color = match source {
        TempoCalendarSource::Url { color, .. } | TempoCalendarSource::Path { color, .. } => {
            Some(color.clone())
        }
    };

    let unfolded = unfold_ics_lines(raw);

    let events: Vec<CalendarEvent> = unfolded
        .split("BEGIN:VEVENT")
        .filter_map(|chunk| {
            let section = chunk.split("END:VEVENT").next()?;
            let title = get_ics_value(section, "SUMMARY")?;
            let (start_value, start_tzid) = get_ics_value_and_tzid(section, "DTSTART")?;
            let start = match parse_ics_datetime(&start_value, start_tzid.as_deref()) {
                Ok(start) => start,
                Err(e) => {
                    warn!("Skipping ICS event with invalid DTSTART: {e:?}");
                    return None;
                }
            };
            let end = get_ics_value_and_tzid(section, "DTEND")
                .and_then(|(value, tzid)| parse_ics_datetime(&value, tzid.as_deref()).ok())
                .unwrap_or_else(|| start + chrono::Duration::hours(1));

            let event = RecurringEvent {
                title,
                start,
                end,
                color: color.clone(),
                rrule: get_ics_value(section, "RRULE"),
            };

            Some(event.expand())
        })
        .flatten()
        .collect();

    let mut seen = HashSet::new();
    events
        .into_iter()
        .filter(|event| {
            seen.insert((
                event.title.clone(),
                event.start.naive_local(),
                event.end.naive_local(),
                event.color.clone(),
            ))
        })
        .collect()
}

#[derive(Clone)]
struct RecurringEvent {
    title: String,
    start: DateTime<Local>,
    end: DateTime<Local>,
    color: Option<String>,
    rrule: Option<String>,
}

impl RecurringEvent {
    fn expand(self) -> Vec<CalendarEvent> {
        let Some(rrule) = self.rrule else {
            return vec![CalendarEvent {
                title: self.title,
                start: self.start,
                end: self.end,
                color: self.color,
            }];
        };

        let rule = RRule::parse(&rrule);
        let duration = self.end - self.start;
        let mut out = Vec::new();
        let mut current = self.start;
        let mut emitted = 0usize;
        let interval = rule.interval.max(1) as i64;
        let count = rule.count.unwrap_or(usize::MAX);
        let until = rule.until;

        while emitted < count {
            if let Some(until) = until
                && current > until
            {
                break;
            }

            if rule.matches(current) {
                out.push(CalendarEvent {
                    title: self.title.clone(),
                    start: current,
                    end: current + duration,
                    color: self.color.clone(),
                });
                emitted += 1;
            }

            current = match rule.freq {
                Frequency::Daily => current + chrono::Duration::days(interval),
                Frequency::Weekly => current + chrono::Duration::weeks(interval),
                Frequency::Monthly => current + chrono::Duration::days(30 * interval),
                Frequency::Yearly => current + chrono::Duration::days(365 * interval),
            };

            if out.len() > 500 {
                break;
            }
        }

        if out.is_empty() {
            vec![CalendarEvent {
                title: self.title,
                start: self.start,
                end: self.end,
                color: self.color,
            }]
        } else {
            out
        }
    }
}

#[derive(Clone, Copy)]
enum Frequency {
    Daily,
    Weekly,
    Monthly,
    Yearly,
}

struct RRule {
    freq: Frequency,
    interval: u32,
    count: Option<usize>,
    until: Option<DateTime<Local>>,
    byday: Vec<Weekday>,
}

impl RRule {
    fn parse(raw: &str) -> Self {
        let mut freq = Frequency::Weekly;
        let mut interval = 1;
        let mut count = None;
        let mut until = None;
        let mut byday = vec![];

        for part in raw.split(';') {
            if let Some(v) = part.strip_prefix("FREQ=") {
                freq = match v {
                    "DAILY" => Frequency::Daily,
                    "WEEKLY" => Frequency::Weekly,
                    "MONTHLY" => Frequency::Monthly,
                    "YEARLY" => Frequency::Yearly,
                    _ => Frequency::Weekly,
                };
            } else if let Some(v) = part.strip_prefix("INTERVAL=") {
                interval = v.parse().unwrap_or(1);
            } else if let Some(v) = part.strip_prefix("COUNT=") {
                count = v.parse().ok();
            } else if let Some(v) = part.strip_prefix("UNTIL=") {
                until = parse_ics_datetime(v, None).ok();
            } else if let Some(v) = part.strip_prefix("BYDAY=") {
                byday = v.split(',').filter_map(parse_weekday).collect();
            }
        }

        Self {
            freq,
            interval,
            count,
            until,
            byday,
        }
    }

    fn matches(&self, dt: DateTime<Local>) -> bool {
        if self.byday.is_empty() {
            return true;
        }
        self.byday.contains(&dt.weekday())
    }
}

fn parse_weekday(v: &str) -> Option<Weekday> {
    match v {
        "MO" => Some(Weekday::Mon),
        "TU" => Some(Weekday::Tue),
        "WE" => Some(Weekday::Wed),
        "TH" => Some(Weekday::Thu),
        "FR" => Some(Weekday::Fri),
        "SA" => Some(Weekday::Sat),
        "SU" => Some(Weekday::Sun),
        _ => None,
    }
}

fn unfold_ics_lines(raw: &str) -> String {
    let mut out = String::new();
    for line in raw.lines() {
        if line.starts_with(' ') || line.starts_with('\t') {
            out.push_str(line.trim_start());
        } else {
            if !out.is_empty() {
                out.push('\n');
            }
            out.push_str(line);
        }
    }
    out
}

fn get_ics_value(section: &str, key: &str) -> Option<String> {
    section
        .lines()
        .find(|line| line.starts_with(key))
        .and_then(|line| {
            line.split_once(':')
                .map(|(_, value)| value.trim().to_string())
        })
}

fn get_ics_value_and_tzid(section: &str, key: &str) -> Option<(String, Option<String>)> {
    section.lines().find_map(|line| {
        let (prop, value) = line.split_once(':')?;
        if !prop.starts_with(key) {
            return None;
        }

        let tzid = prop
            .split(';')
            .find_map(|part| part.strip_prefix("TZID=").map(|v| v.to_string()));

        Some((value.trim().to_string(), tzid))
    })
}

fn parse_ics_datetime(value: &str, tzid: Option<&str>) -> anyhow::Result<DateTime<Local>> {
    let value = value.trim();

    if let Some(utc_value) = value.strip_suffix('Z')
        && let Ok(dt) = NaiveDateTime::parse_from_str(utc_value, "%Y%m%dT%H%M%S")
    {
        return Ok(Utc.from_utc_datetime(&dt).with_timezone(&Local));
    }

    if let Ok(dt) = NaiveDateTime::parse_from_str(value, "%Y%m%dT%H%M%S") {
        if let Some(tzid) = tzid
            && let Ok(tz) = tzid.parse::<Tz>()
        {
            return Ok(tz
                .from_local_datetime(&dt)
                .single()
                .unwrap_or_else(|| tz.from_utc_datetime(&dt))
                .with_timezone(&Local));
        }

        if let Some(local_dt) = Local.from_local_datetime(&dt).single() {
            return Ok(local_dt);
        }
        return Ok(Local.from_utc_datetime(&dt));
    }

    if let Ok(date) = NaiveDate::parse_from_str(value, "%Y%m%d") {
        return Ok(Local
            .from_local_datetime(&date.and_hms_opt(0, 0, 0).unwrap_or_default())
            .single()
            .unwrap_or_else(|| {
                Local.from_utc_datetime(&date.and_hms_opt(0, 0, 0).unwrap_or_default())
            }));
    }

    anyhow::bail!("unsupported ICS datetime: {value}")
}

async fn fetch_location(location: &WeatherLocation) -> anyhow::Result<Location> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(20))
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
        WeatherLocation::Coordinates(lat, lon) => {
            let (city, region_name) = match try_reverse_geocode(&client, *lat, *lon).await {
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
) -> anyhow::Result<Option<(String, String)>> {
    let url = format!(
        "https://nominatim.openstreetmap.org/reverse?format=json&lat={}&lon={}&accept-language=en",
        lat, lon
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

async fn fetch_weather_data(lat: f32, lon: f32) -> anyhow::Result<WeatherData> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(20))
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
