use crate::config::TempoCalendarSource;
use chrono::{DateTime, Datelike, Local, NaiveDate, NaiveDateTime, TimeZone, Utc, Weekday};
use chrono_tz::Tz;
use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex, OnceLock},
    time::Duration,
};

#[derive(Debug, Clone)]
pub struct CalendarEvent {
    pub title: String,
    pub start: DateTime<Local>,
    pub end: DateTime<Local>,
    pub color: Option<String>,
}

#[derive(Clone)]
struct CalendarCacheEntry {
    events: Vec<CalendarEvent>,
    updated_at: std::time::Instant,
}

static CALENDAR_CACHE: OnceLock<Arc<Mutex<HashMap<String, CalendarCacheEntry>>>> = OnceLock::new();

pub async fn fetch_calendar_events_cached(
    source: &TempoCalendarSource,
) -> anyhow::Result<Vec<CalendarEvent>> {
    let key = match source {
        TempoCalendarSource::Url { url, .. } => format!("url:{url}"),
        TempoCalendarSource::Path { path, .. } => format!("path:{path}"),
    };

    let cache = CALENDAR_CACHE
        .get_or_init(|| Arc::new(Mutex::new(HashMap::new())))
        .clone();

    if let Some(entry) = cache.lock().ok().and_then(|m| m.get(&key).cloned())
        && entry.updated_at.elapsed() < Duration::from_secs(600)
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

pub async fn fetch_calendar_events(
    source: &TempoCalendarSource,
) -> anyhow::Result<Vec<CalendarEvent>> {
    let raw = match source {
        TempoCalendarSource::Url { url, .. } => {
            reqwest::Client::new().get(url).send().await?.text().await?
        }
        TempoCalendarSource::Path { path, .. } => {
            std::fs::read_to_string(shellexpand::tilde(path).as_ref())?
        }
    };

    Ok(parse_ics_events(&raw, source))
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
            let start = parse_ics_datetime(&start_value, start_tzid.as_deref()).ok()?;

            let end = match get_ics_value_and_tzid(section, "DTEND") {
                Some((value, _tzid)) if is_ics_date_only(&value) => {
                    parse_ics_date_end(&value).unwrap_or_else(|_| start + chrono::Duration::days(1))
                }
                Some((value, tzid)) => parse_ics_datetime(&value, tzid.as_deref())
                    .ok()
                    .unwrap_or_else(|| start + chrono::Duration::hours(1)),
                None => start + chrono::Duration::hours(1),
            };

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
        while emitted < count {
            if let Some(until) = rule.until
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
        self.byday.is_empty() || self.byday.contains(&dt.weekday())
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

fn parse_ics_date_end(value: &str) -> anyhow::Result<DateTime<Local>> {
    let date = NaiveDate::parse_from_str(value.trim(), "%Y%m%d")?;
    let next_day = date
        .succ_opt()
        .ok_or_else(|| anyhow::anyhow!("invalid ICS end date: {value}"))?;
    Ok(Local
        .from_local_datetime(&next_day.and_hms_opt(0, 0, 0).unwrap_or_default())
        .single()
        .unwrap_or_else(|| {
            Local.from_utc_datetime(&next_day.and_hms_opt(0, 0, 0).unwrap_or_default())
        }))
}

fn is_ics_date_only(value: &str) -> bool {
    value.trim().len() == 8 && value.chars().all(|c| c.is_ascii_digit())
}
