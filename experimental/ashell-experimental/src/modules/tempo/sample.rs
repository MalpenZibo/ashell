//! A week of London weather in November, for pictures of the module.

use ashell_services::weather::{Current, Day, Hour, Place, WeatherData};
use chrono::{Duration, NaiveDateTime, Timelike};

/// The sky, the low and the high of each day, today first.
const DAYS: [(u32, f32, f32); 7] = [
    (2, 7.0, 12.0),
    (61, 6.0, 10.0),
    (3, 4.0, 9.0),
    (1, 5.0, 11.0),
    (80, 8.0, 13.0),
    (0, 6.0, 12.0),
    (63, 5.0, 10.0),
];

/// Today's sky by the hour from ten: clouds coming in, rain after one.
const TODAY: [(u32, u32); 6] = [(10, 2), (11, 3), (12, 3), (13, 61), (14, 61), (15, 3)];

/// How warm it is at `hour` of a day from `low` to `high`: warmest at two in
/// the afternoon, coldest before dawn.
fn warmth(hour: u32, low: f32, high: f32) -> f32 {
    low + (high - low) * (1.0 - ((hour as f32 - 14.0).abs() / 12.0).min(1.0))
}

/// The weather as it would be at `now`, a UTC time on London's clock.
pub fn weather(now: NaiveDateTime) -> WeatherData {
    let midnight = now.date().and_hms_opt(0, 0, 0).unwrap();

    let days = DAYS
        .iter()
        .enumerate()
        .map(|(index, &(code, low, high))| Day {
            date: now.date() + Duration::days(index as i64),
            code,
            temperature_min: low,
            temperature_max: high,
            wind_speed_max: 18.0 + index as f32 * 2.0,
            wind_direction: 240,
            humidity: 76,
        })
        .collect();

    let hours = (0..7 * 24)
        .map(|offset| {
            let time = midnight + Duration::hours(offset);
            let day = (offset / 24) as usize;
            let (code, low, high) = DAYS[day];
            let hour = time.hour();
            let today = TODAY.iter().find(|(at, _)| day == 0 && *at == hour);
            let code = match today {
                Some(&(_, code)) => code,
                None if (10..19).contains(&hour) => code,
                None if code >= 51 => 3,
                None => 1,
            };

            Hour {
                time,
                code,
                temperature: warmth(hour, low, high),
                is_day: (8..17).contains(&hour),
            }
        })
        .collect();

    // As warm as the hour the card calls now, the first after it.
    let (_, low, high) = DAYS[0];
    let temperature = warmth(now.hour() + 1, low, high);

    WeatherData {
        place: Place {
            latitude: 51.5,
            longitude: -0.12,
            city: "London".to_string(),
            region: "United Kingdom".to_string(),
        },
        current: Current {
            time: now.with_minute(now.minute() / 15 * 15).unwrap(),
            code: 2,
            temperature,
            feels_like: temperature - 2.0,
            humidity: 78,
            wind_speed: 14.0,
            wind_direction: 240,
            is_day: true,
        },
        hours,
        days,
    }
}
