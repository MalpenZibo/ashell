use std::rc::Rc;

use guido::prelude::*;
use styletto::components::roll;
use styletto::foundations::tokens::{icon as icon_box, size};
use styletto::foundations::typography as ty;
use styletto::primitives::text::typed;
use styletto::{Icon, centered_row, icon, ui};
use toml::{Table, Value};

use super::{CYCLE_FORMAT, Tempo, duration, sky, time, timer};
use crate::config::{Item, ItemId, Show};
use crate::modules::ItemView;

const CLOCK_FORMAT: &str = "%R";
const DATE_FORMAT: &str = "%a %-d";
const FORECAST_HOURS: usize = 6;
const MAX_FORECAST_HOURS: i64 = 12;

/// An item's own options, read from its table.
pub enum Options {
    /// The formats a right click goes through, the first shown first.
    Clock {
        formats: Vec<String>,
    },
    Date {
        format: String,
    },
    Weather {
        temperature: bool,
    },
    Forecast {
        hours: usize,
    },
    Timer,
    Stopwatch,
    Alarm,
}

impl Options {
    pub fn read(id: ItemId, table: &Table) -> (Self, Vec<String>) {
        let mut problems = Vec::new();
        let known: &[&str] = match id {
            ItemId::Clock => &["format", "formats"],
            ItemId::Date => &["format"],
            ItemId::Weather => &["style"],
            ItemId::Timer | ItemId::Stopwatch | ItemId::Alarm => &[],
            _ => &["hours"],
        };

        for key in table.keys() {
            if !known.contains(&key.as_str()) {
                problems.push(format!("{key}: unknown option of {}, ignored", id.name()));
            }
        }

        let options = match id {
            ItemId::Clock => {
                let mut formats: Vec<String> = match table.get("formats") {
                    Some(Value::Array(values)) => values
                        .iter()
                        .enumerate()
                        .filter_map(|(i, v)| {
                            date_format(&mut problems, &format!("formats[{i}]"), v)
                        })
                        .collect(),
                    Some(_) => {
                        problems.push("formats: expected a list".to_string());
                        Vec::new()
                    }
                    None => Vec::new(),
                };

                if formats.is_empty() {
                    formats.push(
                        table
                            .get("format")
                            .and_then(|v| date_format(&mut problems, "format", v))
                            .unwrap_or_else(|| CLOCK_FORMAT.to_string()),
                    );
                }

                Options::Clock { formats }
            }
            ItemId::Date => Options::Date {
                format: table
                    .get("format")
                    .and_then(|v| date_format(&mut problems, "format", v))
                    .unwrap_or_else(|| DATE_FORMAT.to_string()),
            },
            ItemId::Timer => Options::Timer,
            ItemId::Stopwatch => Options::Stopwatch,
            ItemId::Alarm => Options::Alarm,
            ItemId::Weather => Options::Weather {
                temperature: match table.get("style").map(Value::as_str) {
                    None | Some(Some("icon-temperature")) => true,
                    Some(Some("icon")) => false,
                    Some(_) => {
                        problems
                            .push("style: expected \"icon\" or \"icon-temperature\"".to_string());
                        true
                    }
                },
            },
            _ => Options::Forecast {
                hours: match table.get("hours").map(Value::as_integer) {
                    None => FORECAST_HOURS,
                    Some(Some(hours @ 1..=MAX_FORECAST_HOURS)) => hours as usize,
                    Some(_) => {
                        problems.push(format!(
                            "hours: expected a number from 1 to {MAX_FORECAST_HOURS}"
                        ));
                        FORECAST_HOURS
                    }
                },
            },
        };

        (options, problems)
    }
}

fn date_format(problems: &mut Vec<String>, key: &str, value: &Value) -> Option<String> {
    match value.as_str() {
        Some(format) if time::is_valid(format) => Some(format.to_string()),
        Some(format) => {
            problems.push(format!("{key}: \"{format}\" is not a date format"));
            None
        }
        None => {
            problems.push(format!("{key}: expected a string"));
            None
        }
    }
}

pub fn view(tempo: Tempo, item: &Item) -> ItemView {
    let show = item.show;
    let (options, _) = Options::read(item.id, &item.options);

    match options {
        Options::Clock { formats } => clock(tempo, show, formats),
        Options::Date { format } => date(tempo, show, format),
        Options::Weather { temperature } => weather(tempo, show, temperature),
        Options::Forecast { hours } => forecast(tempo, show, hours),
        Options::Timer => timer_item(tempo, show),
        Options::Stopwatch => stopwatch_item(tempo, show),
        Options::Alarm => alarm_item(tempo, show),
    }
}

fn shown(
    show: Show,
    has: impl Fn() -> bool + 'static,
    notable: impl Fn() -> bool + 'static,
) -> Memo<bool> {
    create_memo(move || {
        has()
            && match show {
                Show::Always => true,
                Show::Notable => notable(),
                Show::Never => false,
            }
    })
}

fn clock(tempo: Tempo, show: Show, formats: Vec<String>) -> ItemView {
    let t = ui();
    let count = formats.len();

    let index = create_signal(0usize);
    let text = create_memo(move || {
        let format = &formats[index.get() % count];

        time::format(tempo.now.get(), tempo.zone().as_ref(), format)
    });

    ItemView {
        view: container().child(
            roll::rolling(move || text.get())
                .look(ty::LABEL_SMALL.em())
                .color(t.colors.text),
        ),
        shown: shown(show, || true, || true),
        act: Rc::new(move |action| match action {
            CYCLE_FORMAT => index.update(|i| *i = (*i + 1) % count),
            action => tempo.zone_action(action),
        }),
    }
}

fn timer_item(tempo: Tempo, show: Show) -> ItemView {
    let t = ui();
    let clock = tempo.clock;
    let state = create_memo(move || clock.timer.state());
    let left = create_memo(move || duration::clock(clock.timer.remaining(clock.tick.get())));

    let view = container()
        .layout(centered_row(size::S1.px()))
        .child(icon(Icon::Gauge, icon_box::SMALL.px(), t.colors.sub))
        .child(
            typed(left, ty::LABEL_SMALL.tabular())
                .color(t.colors.text)
                .nowrap(),
        );

    ItemView {
        view,
        shown: shown(
            show,
            move || state.get() != timer::State::Idle,
            move || state.get() != timer::State::Idle,
        ),
        act: Rc::new(move |action| clock.timer.act(action)),
    }
}

fn stopwatch_item(tempo: Tempo, show: Show) -> ItemView {
    let t = ui();
    let clock = tempo.clock;
    let running = create_memo(move || clock.stopwatch.running());
    let elapsed =
        create_memo(move || duration::stopwatch(clock.stopwatch.elapsed(clock.tick.get())));

    let view = container()
        .layout(centered_row(size::S1.px()))
        .child(icon(Icon::Refresh, icon_box::SMALL.px(), t.colors.sub))
        .child(
            typed(elapsed, ty::LABEL_SMALL.tabular())
                .color(t.colors.text)
                .nowrap(),
        );

    ItemView {
        view,
        shown: shown(
            show,
            move || {
                running.get()
                    || clock.stopwatch.elapsed(clock.tick.get()) > std::time::Duration::ZERO
            },
            move || {
                running.get()
                    || clock.stopwatch.elapsed(clock.tick.get()) > std::time::Duration::ZERO
            },
        ),
        act: Rc::new(move |action| clock.stopwatch.act(action)),
    }
}

fn alarm_item(tempo: Tempo, show: Show) -> ItemView {
    let t = ui();
    let clock = tempo.clock;
    let now = tempo.now;
    let ringing = create_memo(move || clock.alarms.ringing());
    // The next alarm depends on the current time, so it follows the module's
    // one-second tick: otherwise it would keep the time of an alarm that has
    // already rung until the alarm list changed.
    let next = create_memo(move || {
        now.get();
        clock.alarms.next(chrono::Local::now().naive_local())
    });

    let text = create_memo(move || {
        if let Some(alarm) = ringing.get() {
            alarm.time()
        } else {
            next.get()
                .map(|(alarm, _)| alarm.time())
                .unwrap_or_default()
        }
    });
    let ink = move || {
        if ringing.get().is_some() {
            t.colors.accent
        } else {
            t.colors.text
        }
    };

    let view = container()
        .layout(centered_row(size::S1.px()))
        .child(icon(Icon::Bell, icon_box::SMALL.px(), ink))
        .child(typed(text, ty::LABEL_SMALL.tabular()).color(ink).nowrap());

    ItemView {
        view,
        shown: shown(
            show,
            move || ringing.get().is_some() || next.get().is_some(),
            move || ringing.get().is_some() || next.get().is_some(),
        ),
        act: Rc::new(move |action| clock.alarms.act(action)),
    }
}

fn date(tempo: Tempo, show: Show, format: String) -> ItemView {
    let t = ui();

    let text = create_memo(move || time::format(tempo.now.get(), tempo.zone().as_ref(), &format));

    ItemView {
        view: container().child(
            typed(move || text.get(), ty::LABEL_SMALL.tabular())
                .color(t.colors.sub)
                .nowrap(),
        ),
        shown: shown(show, || true, || true),
        act: Rc::new(move |action| tempo.zone_action(action)),
    }
}

fn weather(tempo: Tempo, show: Show, temperature: bool) -> ItemView {
    let t = ui();

    let now = create_memo(move || {
        tempo.weather.with(|data| {
            data.as_ref()
                .map(|d| (d.current.code, d.current.is_day, d.current.temperature))
        })
    });

    let view = container()
        .layout(centered_row(size::S1.px()))
        .child(move || {
            now.get().map(|(code, is_day, _)| {
                icon(
                    sky::icon(code, is_day),
                    icon_box::SMALL.px(),
                    sky::ink(code, t.colors.accent, t.colors.sub),
                )
            })
        })
        .child(temperature.then(|| {
            typed(
                move || {
                    now.get()
                        .map(|(_, _, degrees)| sky::degrees(degrees))
                        .unwrap_or_default()
                },
                ty::LABEL_SMALL.tabular(),
            )
            .color(t.colors.sub)
            .nowrap()
        }));

    ItemView {
        view,
        shown: shown(
            show,
            move || now.with(Option::is_some),
            move || now.get().is_some_and(|(code, _, _)| sky::wet(code)),
        ),
        act: Rc::new(|_| {}),
    }
}

fn forecast(tempo: Tempo, show: Show, hours: usize) -> ItemView {
    let t = ui();
    let formats = time::Formats::of_locale();

    let coming = create_memo(move || {
        let zone = tempo.zone();

        tempo.weather.with(|data| {
            data.as_ref()
                .map(|data| {
                    sky::next_hours(data)
                        .take(hours)
                        .map(|hour| {
                            let when = time::format(hour.time, zone.as_ref(), &formats.hour);

                            sky::Hour::new(hour, when)
                        })
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default()
        })
    });

    let view = container()
        .layout(centered_row(size::S2.px()))
        .children(move || {
            coming
                .get()
                .into_iter()
                .map(|hour| {
                    container()
                        .layout(centered_row(size::S0_5.px()))
                        .child(
                            typed(hour.when, ty::LABEL_XSMALL.tabular())
                                .color(t.colors.dim)
                                .nowrap(),
                        )
                        .child(icon(
                            sky::icon(hour.code, hour.is_day),
                            icon_box::XSMALL.px(),
                            sky::ink(hour.code, t.colors.accent, t.colors.sub),
                        ))
                        .child(
                            typed(sky::degrees(hour.temperature), ty::LABEL_XSMALL.tabular())
                                .color(t.colors.sub)
                                .nowrap(),
                        )
                })
                .collect::<Vec<_>>()
        });

    ItemView {
        view,
        shown: shown(
            show,
            move || coming.with(|hours| !hours.is_empty()),
            move || coming.with(|hours| hours.iter().any(|hour| sky::wet(hour.code))),
        ),
        act: Rc::new(|_| {}),
    }
}
