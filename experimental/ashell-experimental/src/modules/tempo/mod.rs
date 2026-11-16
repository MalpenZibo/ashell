//! The time, the date and the weather: the `clock`, `date`, `weather` and
//! `forecast` items, and the `calendar` panel.

mod calendar;
mod config;
mod items;
mod sky;
mod time;

use std::pin::pin;
use std::time::Duration;

use ashell_i18n::language_subtag;
use ashell_services::weather::{Weather, WeatherData};
use chrono::{NaiveDateTime, Utc};
use futures::StreamExt;
use guido::prelude::*;
use toml::Table;

use super::ItemView;
use crate::config::{Item, ItemId, NEXT_TIMEZONE, PREVIOUS_TIMEZONE};

pub use calendar::WIDTH as CALENDAR_WIDTH;
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

    pub fn calendar(self) -> Container {
        calendar::view(self)
    }
}

pub fn check_options(id: ItemId, options: &Table) -> Vec<String> {
    items::Options::read(id, options).1
}
