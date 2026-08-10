pub mod calendar;
pub mod weather;

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use chrono::{DateTime, Local, NaiveDate};
use guido::prelude::*;

use self::weather::{Location, WeatherData, fetch_location, fetch_weather_data, weather_icon};
use crate::config::{Config, TempoModuleConfig, WeatherIndicator};
use crate::i18n::{language_subtag, unit_system};
use crate::theme::ThemeColors;

/// Reactive tempo state; all fields are Copy signals.
#[derive(Clone, Copy)]
pub struct TempoHandle {
    pub date: RwSignal<DateTime<Local>>,
    pub selected_date: RwSignal<Option<NaiveDate>>,
    pub weather: RwSignal<Option<WeatherData>>,
    pub location: RwSignal<Option<Location>>,
    pub format_index: RwSignal<usize>,
    pub tz_index: RwSignal<usize>,
    pub location_visible: RwSignal<bool>,
}

fn has_seconds(format: &str) -> bool {
    ["%S", "%T", "%X", "%r", "%:z", "%s"]
        .iter()
        .any(|spec| format.contains(spec))
}

pub fn current_format(config: &TempoModuleConfig, index: usize) -> &str {
    if !config.formats.is_empty() {
        config
            .formats
            .get(index)
            .or_else(|| config.formats.first())
            .unwrap_or(&config.clock_format)
    } else {
        &config.clock_format
    }
}

pub fn create() -> TempoHandle {
    let config = with_context::<Config, _>(|c| c.tempo.clone()).unwrap_or_default();

    let handle = TempoHandle {
        date: create_signal(Local::now()),
        selected_date: create_signal(None),
        weather: create_signal(None),
        location: create_signal(None),
        format_index: create_signal(0usize),
        tz_index: create_signal(0usize),
        location_visible: create_signal(true),
    };

    // Time tick: 1s only while the active format renders seconds, otherwise
    // wake at the minute boundary (the port's battery-friendly clock cadence,
    // vs upstream's fixed 5s poll).
    let format_idx_mirror = Arc::new(AtomicUsize::new(0));
    {
        let mirror = format_idx_mirror.clone();
        let format_index = handle.format_index;
        create_effect(move || {
            mirror.store(format_index.get(), Ordering::Relaxed);
        })
        .detach();
    }
    let date_w = handle.date.writer();
    let tick_config = config.clone();
    let _tick = create_service::<(), _, _>(move |_rx, _ctx| async move {
        loop {
            date_w.set(Local::now());
            let idx = format_idx_mirror.load(Ordering::Relaxed);
            let sleep_ms = if has_seconds(current_format(&tick_config, idx)) {
                1000
            } else {
                use chrono::Timelike;
                let now = Local::now();
                (60 - u64::from(now.second())).max(1) * 1000 + 50
            };
            tokio::time::sleep(Duration::from_millis(sleep_ms)).await;
        }
    });

    // Weather: fetch every 30 minutes, linear backoff on failure. Units and
    // language are resolved here on the main thread (the i18n localizer is
    // thread-local).
    if let Some(location_cfg) = config.weather_location.clone() {
        let units = unit_system();
        let wind_unit = config.resolved_wind_speed_unit();
        let lang = language_subtag();
        let weather_w = handle.weather.writer();
        let location_w = handle.location.writer();

        let _weather = create_service::<(), _, _>(move |_rx, _ctx| async move {
            let mut failed_attempt: u64 = 0;
            loop {
                let loc = match fetch_location(&location_cfg, &lang).await {
                    Ok(loc) => {
                        log::debug!("Location fetched successfully: {loc:?}");
                        let (lat, lon) = (loc.latitude, loc.longitude);
                        location_w.set(Some(loc));
                        Some((lat, lon))
                    }
                    Err(e) => {
                        log::warn!("Failed to fetch location: {e:?}");
                        None
                    }
                };

                if let Some((lat, lon)) = loc {
                    match fetch_weather_data(lat, lon, units, wind_unit).await {
                        Ok(data) => {
                            failed_attempt = 0;
                            log::debug!("Weather data fetched successfully");
                            weather_w.set(Some(data));
                            tokio::time::sleep(Duration::from_secs(60 * 30)).await;
                            continue;
                        }
                        Err(e) => {
                            log::warn!("Failed to fetch weather data: {e:?}");
                        }
                    }
                }

                failed_attempt += 1;
                tokio::time::sleep(Duration::from_secs(60 * failed_attempt)).await;
            }
        });
    }

    handle
}

/// Bar view: optional weather indicator + formatted clock. Scrolling cycles
/// the configured timezones.
pub fn view(handle: TempoHandle) -> impl Widget {
    let theme = expect_context::<ThemeColors>();
    let config = with_context::<Config, _>(|c| c.tempo.clone()).unwrap_or_default();
    let temp_symbol = unit_system().temperature_symbol();

    let clock_config = config.clone();
    let clock = text(move || {
        let fmt = current_format(&clock_config, handle.format_index.get()).to_string();
        calendar::time_str(
            &clock_config,
            &fmt,
            handle.tz_index.get(),
            handle.date.get(),
            None,
        )
    })
    .color(theme.text)
    .font_size(13);

    let show_weather =
        config.weather_location.is_some() && config.weather_indicator != WeatherIndicator::None;
    let indicator = config.weather_indicator;

    let tz_count = config.timezones.len();
    let scroll_accum = std::cell::Cell::new(0.0f32);

    let mut row = container()
        .height(fill())
        .layout(
            Flex::row()
                .spacing(8)
                .cross_alignment(CrossAlignment::Center),
        )
        .on_scroll(move |_dx, dy, _source| {
            if tz_count == 0 {
                return;
            }
            let acc = scroll_accum.get() + dy;
            if acc.abs() < 3.0 {
                scroll_accum.set(acc);
                return;
            }
            scroll_accum.set(0.0);
            handle.tz_index.update(|i| {
                *i = if acc < 0.0 {
                    (*i + 1) % tz_count
                } else {
                    i.checked_sub(1).unwrap_or(tz_count - 1)
                };
            });
        });

    if show_weather {
        row = row.child(container().child(move || {
            let data = handle.weather.get()?;
            handle.location.with(|l| l.is_some()).then_some(())?;
            let mut inner = container()
                .layout(
                    Flex::row()
                        .spacing(4)
                        .cross_alignment(CrossAlignment::Center),
                )
                .child(
                    weather_icon(data.current.weather_code, data.current.is_day > 0)
                        .width(12)
                        .height(12),
                );
            if indicator == WeatherIndicator::IconAndTemperature {
                inner = inner.child(
                    text(format!("{}{temp_symbol}", data.current.temperature_2m))
                        .color(theme.text)
                        .font_size(12),
                );
            }
            Some(inner)
        }));
    }

    row.child(clock)
}

/// Menu: calendar + timezones on the left, weather cards on the right.
pub fn menu_view(handle: TempoHandle) -> impl Widget {
    let config = with_context::<Config, _>(|c| c.tempo.clone()).unwrap_or_default();
    let has_weather = config.weather_location.is_some();

    container()
        .width(fill())
        .layout(Flex::row().spacing(16))
        .child(calendar::view(handle, config.clone()))
        .maybe_child(has_weather.then(|| weather::view(handle, config)))
}
