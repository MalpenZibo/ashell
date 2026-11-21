//! The time, the date and the weather: the `clock`, `date`, `weather` and
//! `forecast` items, and the calendar panel: the calendar, alarms, the timer
//! and the stopwatch.

mod alarms;
mod calendar;
mod clock;
mod config;
mod duration;
mod items;
#[cfg(test)]
pub mod sample;
mod sky;
mod stopwatch;
mod time;
mod timer;

use std::pin::pin;
use std::time::Duration;

use ashell_i18n::language_subtag;
use ashell_services::weather::{Weather, WeatherData};
use chrono::{NaiveDateTime, Utc};
use futures::StreamExt;
use guido::prelude::*;
use toml::Table;

use super::{ItemView, Spec};
use crate::config::{Action, Gestures, Item, ItemId, Panel};

pub use config::TempoConfig;
use config::TimeZone;

/// The module's state, shared by every item and panel it draws.
#[derive(Clone, Copy)]
pub struct Tempo {
    config: Memo<TempoConfig>,
    /// UTC, on the second.
    now: Signal<NaiveDateTime>,
    /// Which of `[tempo] timezones` the time is told in.
    zone: RwSignal<usize>,
    weather: Signal<Option<WeatherData>>,
    /// The place of the weather, hidden for a screenshot.
    place_hidden: RwSignal<bool>,
    /// The calendar panel: its open tab, the alarms, the timer and the
    /// stopwatch.
    clock: clock::Clock,
}

impl Tempo {
    pub fn new(config: Memo<TempoConfig>) -> Self {
        let now = create_signal(time::now());
        let tick = now.writer();

        create_task(move |_| async move {
            loop {
                let into = Utc::now().timestamp_subsec_millis() as u64;

                tokio::time::sleep(Duration::from_millis(1000 - into.min(999))).await;
                tick.set(time::now());
            }
        });

        let zone = create_signal(0usize);

        create_effect(move || {
            let count = config.with(|c| c.timezones.len());

            if zone.get_untracked() >= count.max(1) {
                zone.set(0);
            }
        });

        let weather = create_signal(None);
        let query = create_memo(move || config.with(|c| c.weather_query(language_subtag())));

        create_effect(move || {
            let query = query.get();

            weather.set(None);
            let Some(query) = query else {
                return;
            };

            let forecast = weather.writer();

            create_task(move |_| async move {
                let weather = match Weather::connect(query).await {
                    Ok(weather) => weather,
                    Err(err) => {
                        log::error!("tempo: the weather service did not start: {err}");
                        return;
                    }
                };

                let mut updates = pin!(weather.updates());

                while let Some(update) = updates.next().await {
                    if update.is_some() {
                        forecast.set(update);
                    }
                }
            });
        });

        Self {
            config,
            now: now.read_only(),
            zone,
            weather: weather.read_only(),
            place_hidden: create_signal(false),
            clock: clock::Clock::new(),
        }
    }

    /// At `now` with `weather`, with no tick and no weather service: for
    /// pictures of the module.
    #[cfg(test)]
    pub fn fixed(
        config: Memo<TempoConfig>,
        now: NaiveDateTime,
        weather: Option<WeatherData>,
    ) -> Self {
        Self {
            config,
            now: create_signal(now).read_only(),
            zone: create_signal(0),
            weather: create_signal(weather).read_only(),
            place_hidden: create_signal(false),
            clock: clock::Clock::fixed(),
        }
    }

    /// The time zone the time is told in, the system's when none is set.
    fn zone(self) -> Option<TimeZone> {
        let index = self.zone.get();

        self.config.with(|c| c.timezones.get(index).cloned())
    }

    fn step_zone(self, by: isize) {
        let count = self.config.with(|c| c.timezones.len()) as isize;

        if count > 0 {
            self.zone
                .update(|index| *index = (*index as isize + by).rem_euclid(count) as usize);
        }
    }

    /// The action of a time zone's, shared by every item that tells the time.
    fn zone_action(self, action: &str) {
        match action {
            NEXT_TIMEZONE => self.step_zone(1),
            PREVIOUS_TIMEZONE => self.step_zone(-1),
            _ => {}
        }
    }

    pub fn item(self, item: &Item) -> ItemView {
        items::view(self, item)
    }

    /// Its one panel, the calendar.
    pub fn panel(self, _panel: Panel) -> Container {
        clock::view(self)
    }
}

const CYCLE_FORMAT: &str = "cycle_format";
const NEXT_TIMEZONE: &str = "next_timezone";
const PREVIOUS_TIMEZONE: &str = "previous_timezone";

pub const SPEC: Spec = Spec {
    name: "tempo",
    actions,
    default_gestures,
    check_options,
    panel_width,
};

fn actions(id: ItemId) -> &'static [&'static str] {
    match id {
        ItemId::Clock => &[CYCLE_FORMAT, NEXT_TIMEZONE, PREVIOUS_TIMEZONE],
        ItemId::Date => &[NEXT_TIMEZONE, PREVIOUS_TIMEZONE],
        ItemId::Timer => timer::ACTIONS,
        ItemId::Stopwatch => stopwatch::ACTIONS,
        ItemId::Alarm => alarms::ACTIONS,
        _ => &[],
    }
}

/// ashell's: a right click cycles the clock's formats, the wheel the time
/// zones.
fn default_gestures(id: ItemId) -> Gestures {
    let named = |name| Some(Action::Named(name));
    let zones = Gestures {
        scroll_up: named(NEXT_TIMEZONE),
        scroll_down: named(PREVIOUS_TIMEZONE),
        ..Gestures::default()
    };

    match id {
        ItemId::Clock => Gestures {
            right_click: named(CYCLE_FORMAT),
            ..zones
        },
        ItemId::Date => zones,
        ItemId::Timer => Gestures {
            right_click: named(timer::TOGGLE),
            ..Gestures::default()
        },
        ItemId::Stopwatch => Gestures {
            right_click: named(stopwatch::TOGGLE),
            ..Gestures::default()
        },
        ItemId::Alarm => Gestures {
            right_click: named(alarms::SNOOZE),
            ..Gestures::default()
        },
        _ => Gestures::default(),
    }
}

fn check_options(id: ItemId, options: &Table) -> Vec<String> {
    items::Options::read(id, options).1
}

fn panel_width(_panel: Panel) -> f32 {
    clock::WIDTH
}
