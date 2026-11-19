use std::fmt::Write;

use ashell_i18n::chrono_locale;
use chrono::format::{Fixed, Item, Numeric, StrftimeItems};
use chrono::{DateTime, FixedOffset, Local, Locale, NaiveDate, NaiveDateTime, TimeZone as _, Utc};

use super::config::{TimeZone, Zone};

/// `utc` written with `format` in `zone`, or in the system's when there is
/// none, with the locale's names.
pub fn format(utc: NaiveDateTime, zone: Option<&TimeZone>, format: &str) -> String {
    match zone.map(|z| z.zone) {
        Some(Zone::Named(tz)) => written(tz.from_utc_datetime(&utc), format),
        Some(Zone::Offset(offset)) => written(offset.from_utc_datetime(&utc), format),
        None => written(Local.from_utc_datetime(&utc), format),
    }
}

/// Kept on the zone's own type, so `%Z` writes its abbreviation.
fn written<Tz: chrono::TimeZone>(time: DateTime<Tz>, format: &str) -> String
where
    Tz::Offset: std::fmt::Display,
{
    let mut out = String::new();

    if write!(out, "{}", time.format_localized(format, chrono_locale())).is_err() {
        out.clear();
    }

    out
}

/// `utc` in `zone`, or in the system's when there is none.
pub fn local(utc: NaiveDateTime, zone: Option<&TimeZone>) -> DateTime<FixedOffset> {
    match zone.map(|z| z.zone) {
        Some(Zone::Named(tz)) => tz.from_utc_datetime(&utc).fixed_offset(),
        Some(Zone::Offset(offset)) => offset.from_utc_datetime(&utc).fixed_offset(),
        None => Local.from_utc_datetime(&utc).fixed_offset(),
    }
}

pub fn date(utc: NaiveDateTime, zone: Option<&TimeZone>) -> NaiveDate {
    local(utc, zone).date_naive()
}

pub fn now() -> NaiveDateTime {
    Utc::now().naive_utc()
}

/// Whether `format` is one chrono can write: a wrong specifier would leave
/// the clock empty.
pub fn is_valid(format: &str) -> bool {
    !StrftimeItems::new(format).any(|item| matches!(item, Item::Error))
}

/// Where a 12-hour clock puts the AM/PM marker.
#[derive(Debug, PartialEq, Eq)]
enum AmPm {
    After,
    /// Before the hour, with the text the clock puts between the two.
    Before(String),
}

/// Where `format` puts the AM/PM marker in `locale`, or `None` when it
/// renders a 24-hour time. Parsing with the locale expands `%r`, `%X` and
/// `%c`, which are 12-hour in some locales only and put the marker first in
/// `ja_JP`, `ko_KR` and `zh_CN`.
fn am_pm_placement(format: &str, locale: Locale) -> Option<AmPm> {
    let mut twelve_hour = false;
    let mut hour_seen = false;
    let mut before = None::<String>;

    for item in StrftimeItems::new_with_locale(format, locale) {
        match item {
            Item::Fixed(Fixed::UpperAmPm | Fixed::LowerAmPm) => {
                if hour_seen {
                    return Some(AmPm::After);
                }
                before = Some(String::new());
            }
            Item::Numeric(hour @ (Numeric::Hour | Numeric::Hour12), _) => {
                if let Some(separator) = before {
                    return Some(AmPm::Before(separator));
                }
                hour_seen = true;
                twelve_hour |= hour == Numeric::Hour12;
            }
            Item::Literal(text) | Item::Space(text) => {
                if let Some(separator) = &mut before {
                    separator.push_str(text);
                }
            }
            Item::OwnedLiteral(text) | Item::OwnedSpace(text) => {
                if let Some(separator) = &mut before {
                    separator.push_str(&text);
                }
            }
            _ => {}
        }
    }

    (twelve_hour || before.is_some()).then_some(AmPm::After)
}

/// The word `locale` writes after a bare 12-hour hour, like `시` in `ko_KR`,
/// `時` in `ja_JP` and `时` in `zh_CN`. Taken from the text after `%I` in the
/// locale's `%r`, and only where `%r` puts the marker first, since `br_FR`
/// separates the hour and minutes with a letter (`%Ie%M`).
fn hour_unit(locale: Locale) -> Option<String> {
    let Some(AmPm::Before(_)) = am_pm_placement("%r", locale) else {
        return None;
    };

    let mut items = StrftimeItems::new_with_locale("%r", locale)
        .skip_while(|item| !matches!(item, Item::Numeric(Numeric::Hour12, _)))
        .skip(1);

    let unit = match items.next()? {
        Item::Literal(text) => text.to_owned(),
        Item::OwnedLiteral(text) => text.into(),
        _ => return None,
    };

    unit.chars().all(char::is_alphabetic).then_some(unit)
}

/// The formats of the times the panel writes, on the locale's clock: 12 or
/// 24 hours, as its own time (`%X`) is.
#[derive(Clone)]
pub struct Formats {
    /// Hours and minutes.
    pub time: String,
    /// A bare hour, as a forecast's.
    pub hour: String,
}

impl Formats {
    pub fn of_locale() -> Self {
        let locale = chrono_locale();
        let twelve = |hours: &str| match am_pm_placement("%X", locale) {
            None => None,
            Some(AmPm::After) => Some(format!("{hours} %p")),
            Some(AmPm::Before(separator)) => {
                Some(format!("%p{}{hours}", separator.replace('%', "%%")))
            }
        };

        let hour = match hour_unit(locale) {
            Some(unit) => format!("%-I{unit}"),
            None => "%-I".to_string(),
        };

        Self {
            time: twelve("%-I:%M").unwrap_or_else(|| "%H:%M".to_string()),
            hour: twelve(&hour).unwrap_or_else(|| "%H".to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn am_pm_placement_follows_the_clock() {
        let before = |separator: &str| Some(AmPm::Before(separator.to_owned()));

        assert_eq!(am_pm_placement("%H:%M", Locale::en_US), None);
        assert_eq!(
            am_pm_placement("%I:%M %p", Locale::en_US),
            Some(AmPm::After)
        );
        assert_eq!(
            am_pm_placement("%H:%M %p", Locale::en_US),
            Some(AmPm::After)
        );
        assert_eq!(am_pm_placement("%p%I:%M", Locale::en_US), before(""));
        assert_eq!(am_pm_placement("%a %X", Locale::en_US), Some(AmPm::After));
        assert_eq!(am_pm_placement("%r", Locale::de_DE), None);
        assert_eq!(am_pm_placement("%r", Locale::ja_JP), before(""));
        assert_eq!(am_pm_placement("%r", Locale::ko_KR), before(" "));
        assert_eq!(am_pm_placement("%r", Locale::zh_CN), before(" "));
    }

    #[test]
    fn hour_unit_comes_from_the_locale() {
        assert_eq!(hour_unit(Locale::ko_KR).as_deref(), Some("시"));
        assert_eq!(hour_unit(Locale::ja_JP).as_deref(), Some("時"));
        assert_eq!(hour_unit(Locale::zh_CN).as_deref(), Some("时"));
        assert_eq!(hour_unit(Locale::en_US), None);
        assert_eq!(hour_unit(Locale::de_DE), None);
        assert_eq!(hour_unit(Locale::br_FR), None);
        assert_eq!(hour_unit(Locale::zh_TW).as_deref(), Some("時"));
        assert_eq!(hour_unit(Locale::bo_CN), None);
    }

    #[test]
    fn a_time_is_written_in_its_zone() {
        let utc = NaiveDateTime::parse_from_str("2026-11-16T22:30", "%FT%R").unwrap();
        let rome = TimeZone::parse("Europe/Rome");
        let tokyo = TimeZone::parse("+09:00");

        assert_eq!(format(utc, rome.as_ref(), "%H:%M"), "23:30");
        assert_eq!(format(utc, tokyo.as_ref(), "%H:%M"), "07:30");
        assert_eq!(date(utc, tokyo.as_ref()).to_string(), "2026-11-17");
    }

    #[test]
    fn a_wrong_specifier_is_not_a_format() {
        assert!(is_valid("%a %d %b %R"));
        assert!(!is_valid("%Q"));
    }
}
