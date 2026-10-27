pub mod calendar;
pub mod weather;

use std::time::Duration;

use chrono::{
    DateTime, Local, Locale, NaiveDate,
    format::{Fixed, Item, Numeric, Pad, StrftimeItems},
};
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
    theme::use_theme,
};
use ashell_i18n::{chrono_locale, language_subtag, unit_system};

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

/// Weather times in the menu, picked to match
/// the clock's hour cycle. Hourly forecast entries always fall on the hour,
/// so the 12-hour variant drops the minutes there.
const WEATHER_TIME_FORMAT_12H: &str = "%-I:%M %p";
const WEATHER_HOUR_FORMAT_12H: &str = "%-I %p";
const WEATHER_TIME_FORMAT_24H: &str = "%H:%M";

/// Whether `format` renders a 12-hour time in `locale`. Parsing with the
/// locale expands `%r`, `%X` and `%c`, which are 12-hour in some locales only.
fn uses_twelve_hour(format: &str, locale: Locale) -> bool {
    StrftimeItems::new_with_locale(format, locale).any(|item| {
        matches!(
            item,
            Item::Numeric(Numeric::Hour12, _)
                | Item::Fixed(Fixed::UpperAmPm)
                | Item::Fixed(Fixed::LowerAmPm)
        )
    })
}

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

/// Formats for the dates in the menu, the forecast days and the calendar header.
#[derive(Debug, PartialEq, Eq)]
struct DateFormats {
    forecast: String,
    header: String,
}

/// Date formats that follow the clock. Locales whose `%x` writes the year with
/// a word after it, like `年` and `년`, keep that order. Otherwise the month goes
/// first when the clock puts it before the day, or `%x` does for a clock with no
/// date, and a date that starts with the year stays day-first. The day and month
/// keep the clock's padding.
fn date_formats(clock: &str, locale: Locale) -> DateFormats {
    let clock: Vec<_> = StrftimeItems::new_with_locale(clock, locale).collect();
    let day_pad = pad_of(&clock, Numeric::Day);
    let month_pad = pad_of(&clock, Numeric::Month);

    if let Some(formats) = unit_date_formats(locale, day_pad, month_pad) {
        return formats;
    }

    let day = format!("%{}d", pad_flag(day_pad.unwrap_or(Pad::Zero)));
    let month_first = if clock.iter().any(is_day) && clock.iter().any(is_month) {
        month_first(&clock)
    } else {
        month_first(&StrftimeItems::new_with_locale("%x", locale).collect::<Vec<_>>())
    };

    if month_first {
        DateFormats {
            forecast: format!("%a, %b {day}"),
            header: format!("%B {day}, %Y"),
        }
    } else {
        DateFormats {
            forecast: format!("%a, {day} %b"),
            header: format!("{day} %B %Y"),
        }
    }
}

/// Formats from the locale's own `%x`, for locales that write the year, month
/// and day with words after them. The weekday goes where the locale's `%x` or
/// `%c` puts it, and in parentheses after the date where neither has one.
fn unit_date_formats(
    locale: Locale,
    day_pad: Option<Pad>,
    month_pad: Option<Pad>,
) -> Option<DateFormats> {
    let date: Vec<_> = StrftimeItems::new_with_locale("%x", locale).collect();
    let year = date.iter().position(is_year)?;
    let month = date.iter().position(is_month)?;
    let end = date_end(&date)?;

    if !(year < month && month < end && date.get(year + 1).is_some_and(is_word)) {
        return None;
    }

    let spec = |item: &Item| match item {
        Item::Numeric(Numeric::Day, pad) => {
            Some(format!("%{}d", pad_flag(day_pad.unwrap_or(*pad))))
        }
        Item::Numeric(Numeric::Month, pad) => {
            Some(format!("%{}m", pad_flag(month_pad.unwrap_or(*pad))))
        }
        item => spec(item),
    };
    let header = date[year..end]
        .iter()
        .map(spec)
        .collect::<Option<String>>()?;
    let month_day = date[month..end]
        .iter()
        .map(spec)
        .collect::<Option<String>>()?;

    let date_time: Vec<_> = StrftimeItems::new_with_locale("%c", locale).collect();
    let weekday = weekday_after(&date[end..])
        .or_else(|| weekday_after(&date_time[date_end(&date_time)?..]))
        .unwrap_or_else(|| "(%a)".to_owned());

    Some(DateFormats {
        forecast: format!("{month_day}{weekday}"),
        header,
    })
}

/// The weekday right after a date, with the spaces and parentheses around it.
fn weekday_after(items: &[Item]) -> Option<String> {
    let weekday = items.iter().position(|item| {
        matches!(
            item,
            Item::Fixed(Fixed::ShortWeekdayName | Fixed::LongWeekdayName)
        )
    })?;
    let before_ok = items[..weekday].iter().all(|item| match item {
        Item::Space(_) | Item::OwnedSpace(_) => true,
        Item::Literal(text) => *text == "(",
        Item::OwnedLiteral(text) => &**text == "(",
        _ => false,
    });
    if !before_ok {
        return None;
    }

    let close = match items.get(weekday + 1) {
        Some(Item::Literal(")")) => 1,
        Some(Item::OwnedLiteral(text)) if &**text == ")" => 1,
        _ => 0,
    };
    items[..=weekday + close].iter().map(spec).collect()
}

/// The index after the day, and after the word that follows it, like `日`.
fn date_end(items: &[Item]) -> Option<usize> {
    let day = items.iter().position(is_day)?;
    Some(day + 1 + usize::from(items.get(day + 1).is_some_and(is_word)))
}

fn month_first(items: &[Item]) -> bool {
    let year = items.iter().position(is_year);
    match (
        items.iter().position(is_month),
        items.iter().position(is_day),
    ) {
        (Some(month), Some(day)) => month < day && year.is_none_or(|year| year > month),
        _ => false,
    }
}

fn pad_of(items: &[Item], numeric: Numeric) -> Option<Pad> {
    items.iter().find_map(|item| match item {
        Item::Numeric(found, pad) if *found == numeric => Some(*pad),
        _ => None,
    })
}

fn pad_flag(pad: Pad) -> &'static str {
    match pad {
        Pad::Zero => "",
        Pad::None => "-",
        Pad::Space => "_",
    }
}

fn is_day(item: &Item) -> bool {
    matches!(item, Item::Numeric(Numeric::Day, _))
}

fn is_month(item: &Item) -> bool {
    matches!(
        item,
        Item::Numeric(Numeric::Month, _)
            | Item::Fixed(Fixed::ShortMonthName | Fixed::LongMonthName)
    )
}

fn is_year(item: &Item) -> bool {
    matches!(item, Item::Numeric(Numeric::Year | Numeric::YearMod100, _))
}

/// A literal made of letters, like `年` or `일`.
fn is_word(item: &Item) -> bool {
    let text = match item {
        Item::Literal(text) => *text,
        Item::OwnedLiteral(text) => text,
        _ => return false,
    };
    text.chars().all(char::is_alphabetic)
}

/// Writes a date item back as strftime text.
fn spec(item: &Item) -> Option<String> {
    Some(match item {
        Item::Literal(text) | Item::Space(text) => text.replace('%', "%%"),
        Item::OwnedLiteral(text) | Item::OwnedSpace(text) => text.replace('%', "%%"),
        Item::Numeric(Numeric::Year, pad) => format!("%{}Y", pad_flag(*pad)),
        Item::Numeric(Numeric::Month, pad) => format!("%{}m", pad_flag(*pad)),
        Item::Numeric(Numeric::Day, pad) => format!("%{}d", pad_flag(*pad)),
        Item::Fixed(Fixed::ShortWeekdayName) => "%a".to_owned(),
        Item::Fixed(Fixed::LongWeekdayName) => "%A".to_owned(),
        Item::Fixed(Fixed::ShortMonthName) => "%b".to_owned(),
        Item::Fixed(Fixed::LongMonthName) => "%B".to_owned(),
        _ => return None,
    })
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

    fn weather_time_format(&self) -> &str {
        self.weather_format(WEATHER_TIME_FORMAT_12H)
    }

    fn weather_hour_format(&self) -> &str {
        self.weather_format(WEATHER_HOUR_FORMAT_12H)
    }

    fn weather_format(&self, twelve_hour: &'static str) -> &str {
        if uses_twelve_hour(self.current_format(), chrono_locale()) {
            twelve_hour
        } else {
            WEATHER_TIME_FORMAT_24H
        }
    }

    fn menu_date_formats(&self) -> DateFormats {
        date_formats(self.current_format(), chrono_locale())
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

#[cfg(test)]
mod tests {
    use super::*;

    fn render(clock: &str, locale: Locale) -> (String, String) {
        let formats = date_formats(clock, locale);
        let date = NaiveDate::from_ymd_opt(2026, 10, 28).expect("valid date");
        (
            date.format_localized(&formats.forecast, locale).to_string(),
            date.format_localized(&formats.header, locale).to_string(),
        )
    }

    fn rendered(forecast: &str, header: &str) -> (String, String) {
        (forecast.to_owned(), header.to_owned())
    }

    #[test]
    fn date_order_follows_the_clock() {
        let day_first = rendered("Wed, 28 Oct", "28 October 2026");
        let month_first = rendered("Wed, Oct 28", "October 28, 2026");

        assert_eq!(render("%a %d %b %R", Locale::en_US), day_first);
        assert_eq!(render("%a %b %d  %I:%M:%S %p", Locale::en_US), month_first);
        assert_eq!(render("%Y-%m-%d %R", Locale::en_US), day_first);
        assert_eq!(render("%a %b %-d", Locale::en_US).1, "October 28, 2026");
    }

    #[test]
    fn date_order_falls_back_to_the_locale() {
        assert_eq!(
            render("%R", Locale::en_US),
            rendered("Wed, Oct 28", "October 28, 2026")
        );
        assert_eq!(
            render("%R", Locale::en_GB),
            rendered("Wed, 28 Oct", "28 October 2026")
        );
        assert_eq!(
            render("%R", Locale::sv_SE),
            rendered("ons, 28 okt", "28 oktober 2026")
        );
    }

    #[test]
    fn unit_locales_keep_their_own_order() {
        assert_eq!(
            render("%R", Locale::ja_JP),
            rendered("10月28日(水)", "2026年10月28日")
        );
        assert_eq!(
            render("%R", Locale::ko_KR),
            rendered("10월 28일 (수)", "2026년 10월 28일")
        );
        assert_eq!(
            render("%R", Locale::zh_CN),
            rendered("10月28日 星期三", "2026年10月28日")
        );
        assert_eq!(
            render("%R", Locale::zh_TW),
            rendered("10月28日 (週三)", "2026年10月28日")
        );
        assert_eq!(
            date_formats("%-m月%-d日 %R", Locale::ja_JP),
            DateFormats {
                forecast: "%-m月%-d日(%a)".to_owned(),
                header: "%Y年%-m月%-d日".to_owned(),
            }
        );
    }
}
