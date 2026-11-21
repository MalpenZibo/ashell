//! The calendar panel: the time, its time zones and the weather on the
//! left, the month on the right.

use ashell_i18n::{chrono_locale, t};
use ashell_services::weather::WeatherData;
use chrono::{Datelike, Months, NaiveDate, Timelike, Weekday};
use guido::prelude::*;
use styletto::components::button::{Fill, button};
use styletto::components::roll;
use styletto::foundations::control::Size;
use styletto::foundations::tokens::{icon as icon_box, opacity, radius, size};
use styletto::foundations::typography as ty;
use styletto::primitives::text::{caption, typed};
use styletto::{Icon, centered_row, foundations::motion, icon, ui};

use super::config::TimeZone;
use super::{Tempo, sky, time};

/// The panel's width, following the density.
pub fn width() -> f32 {
    size::scaled(600.0)
}

/// The clock's column, following the density.
fn left() -> f32 {
    size::scaled(260.0)
}
/// How many hours of forecast the weather card shows.
const HOURS: usize = 6;
/// The hours of a day the card shows for it.
const DAY_HOURS: [u32; HOURS] = [6, 9, 12, 15, 18, 21];

pub fn view(tempo: Tempo) -> Container {
    // The day the weather card is about: none for now.
    let chosen = create_signal(None::<NaiveDate>);
    let has_weather = create_memo(move || tempo.weather.with(Option::is_some));

    container()
        .width(width())
        .padding(size::S4.px())
        .layout(Flex::row().spacing(size::S4.px()))
        .child(
            container()
                .width(left())
                .layout(Flex::column().spacing(size::S3.px()))
                .child(clock(tempo))
                .child(zones(tempo))
                .child(move || has_weather.get().then(|| card(tempo, chosen))),
        )
        .child(month(tempo, chosen))
}

/// The time, its hours and minutes large, its seconds and AM/PM marker
/// small, and the day under it.
fn clock(tempo: Tempo) -> Container {
    let t = ui();
    let time::Clock {
        before,
        time,
        after,
    } = time::Formats::of_locale().clock;
    let written = move |format: &str| time::format(tempo.now.get(), tempo.zone().as_ref(), format);
    let small = |text: Memo<String>| {
        typed(move || text.get(), ty::TITLE_MEDIUM.tabular())
            .color(t.colors.dim)
            .nowrap()
    };
    let marker_first = !before.is_empty();

    let before = create_memo(move || written(&before));
    let time = create_memo(move || written(&time));
    let after = create_memo(move || written(&after));
    let day = create_memo(move || written("%A %-d %B"));

    container()
        .layout(Flex::column().spacing(size::S0_5.px()))
        .child(
            container()
                .layout(Flex::row().cross_alignment(CrossAlignment::Baseline))
                .child(marker_first.then(|| {
                    container()
                        .padding([0.0, size::S1.px(), 0.0, 0.0])
                        .child(small(before))
                }))
                .child(
                    roll::rolling(move || time.get())
                        .look(ty::DISPLAY_MEDIUM.em())
                        .color(t.colors.text),
                )
                .child(small(after)),
        )
        .child(
            typed(move || day.get(), ty::LABEL_MEDIUM)
                .color(t.colors.sub)
                .nowrap(),
        )
}

/// `[tempo] timezones`, each with its time: choosing one tells the time in
/// it.
fn zones(tempo: Tempo) -> Container {
    let t = ui();
    let formats = time::Formats::of_locale();

    container()
        .width(fill())
        .layout(Flex::column().spacing(size::S0_5.px()))
        .children(move || {
            let active = tempo.zone.get();

            tempo
                .config
                .with(|c| c.timezones.clone())
                .into_iter()
                .enumerate()
                .map(|(index, zone)| {
                    let on = index == active;
                    let ink = if on { t.colors.text } else { t.colors.sub };
                    let name = zone.name.clone();
                    let format = formats.time.clone();
                    let when =
                        create_memo(move || time::format(tempo.now.get(), Some(&zone), &format));

                    container()
                        .width(fill())
                        .height(Size::Sm.height())
                        .padding([0.0, size::S2.px()])
                        .corners(radius::SMALL.corners())
                        .background(if on {
                            t.colors.surface2
                        } else {
                            Color::TRANSPARENT
                        })
                        .when_hovered(move |s| s.background(t.colors.surface))
                        .on_click(move || tempo.zone.set(index))
                        .layout(
                            Flex::row()
                                .main_alignment(MainAlignment::SpaceBetween)
                                .cross_alignment(CrossAlignment::Center),
                        )
                        .child(typed(name, ty::LABEL_SMALL).color(ink).nowrap())
                        .child(
                            typed(move || when.get(), ty::LABEL_SMALL.tabular())
                                .color(ink)
                                .nowrap(),
                        )
                })
                .collect::<Vec<_>>()
        })
}

/// What the weather card says.
#[derive(Clone, Debug, PartialEq)]
struct Card {
    code: u32,
    is_day: bool,
    figure: String,
    what: String,
    lines: [String; 2],
    hours: Vec<sky::Hour>,
}

/// The card for now, or for `day`.
fn card_for(
    data: &WeatherData,
    day: Option<NaiveDate>,
    zone: Option<&TimeZone>,
    wind: &str,
    formats: &time::Formats,
) -> Card {
    let now = &data.current;
    let today = time::date(now.time, zone);

    let high_low = |max: f32, min: f32| {
        t!(
            "tempo-high-low",
            high = max.round(),
            low = min.round(),
            unit = "°"
        )
    };
    let conditions = |humidity: u32, speed: f32| {
        format!(
            "{} {humidity}% · {} {} {wind}",
            t!("tempo-humidity"),
            t!("tempo-wind"),
            speed.round()
        )
    };

    match day
        .filter(|day| *day != today)
        .and_then(|day| data.days.iter().find(|d| d.date == day))
    {
        Some(day) => Card {
            code: day.code,
            is_day: true,
            figure: sky::degrees(day.temperature_max),
            what: sky::describe(day.code),
            lines: [
                high_low(day.temperature_max, day.temperature_min),
                conditions(day.humidity, day.wind_speed_max),
            ],
            hours: data
                .hours
                .iter()
                .filter(|h| {
                    let local = time::local(h.time, zone);

                    local.date_naive() == day.date && DAY_HOURS.contains(&local.hour())
                })
                .map(|h| sky::Hour::new(h, time::format(h.time, zone, &formats.hour)))
                .collect(),
        },
        None => Card {
            code: now.code,
            is_day: now.is_day,
            figure: sky::degrees(now.temperature),
            what: sky::describe(now.code),
            lines: [
                [
                    data.days
                        .first()
                        .map(|d| high_low(d.temperature_max, d.temperature_min))
                        .unwrap_or_default(),
                    t!(
                        "tempo-feels-like",
                        value = now.feels_like.round(),
                        unit = "°"
                    ),
                ]
                .join(" · "),
                conditions(now.humidity, now.wind_speed),
            ],
            hours: sky::next_hours(data)
                .take(HOURS)
                .enumerate()
                .map(|(i, h)| {
                    let when = if i == 0 {
                        t!("tempo-now")
                    } else {
                        time::format(h.time, zone, &formats.hour)
                    };

                    sky::Hour::new(h, when)
                })
                .collect(),
        },
    }
}

/// The weather now and the next hours, or a day of the forecast and its
/// hours, on the accent's soft gradient.
fn card(tempo: Tempo, chosen: RwSignal<Option<NaiveDate>>) -> Container {
    let t = ui();
    let formats = time::Formats::of_locale();

    let card = create_memo(move || {
        let zone = tempo.zone();
        let wind = sky::wind_symbol(tempo.config.with(|c| c.wind_speed_unit()));

        tempo.weather.with(|data| {
            data.as_ref()
                .map(|data| card_for(data, chosen.get(), zone.as_ref(), wind, &formats))
        })
    });

    let place = create_memo(move || {
        tempo.weather.with(|data| {
            data.as_ref()
                .map(|data| {
                    let place = &data.place;

                    if place.region.is_empty() {
                        place.city.clone()
                    } else {
                        format!("{}, {}", place.city, place.region)
                    }
                })
                .unwrap_or_default()
        })
    });

    let read =
        move |f: fn(&Card) -> String| move || card.with(|c| c.as_ref().map(f).unwrap_or_default());

    container()
        .width(fill())
        .padding(size::S3.px())
        .corners(radius::MEDIUM.corners())
        .gradient(LinearGradient::diagonal(
            t.colors.accent.with_alpha(0.28),
            t.colors.accent.with_alpha(0.04),
        ))
        .layout(Flex::column())
        .child(
            container()
                .layout(centered_row(size::S3.px()))
                .child(move || {
                    card.get().map(|c| {
                        icon(
                            sky::icon(c.code, c.is_day),
                            icon_box::XLARGE.px(),
                            sky::ink(c.code, t.colors.accent, t.colors.text),
                        )
                    })
                })
                .child(
                    container()
                        .layout(Flex::column().spacing(size::S1.px()))
                        .child(
                            typed(read(|c| c.figure.clone()), ty::DISPLAY_SMALL.em().tabular())
                                .color(t.colors.text)
                                .nowrap(),
                        )
                        .child(caption(read(|c| c.what.clone()))),
                ),
        )
        .child(
            container()
                .padding([size::S2.px(), 0.0, 0.0, 0.0])
                .layout(Flex::column().spacing(size::S0_5.px()))
                .child(
                    container()
                        .on_click(move || tempo.place_hidden.update(|hidden| *hidden = !*hidden))
                        .layout(centered_row(size::S1.px()))
                        .child(icon(
                            move || {
                                if tempo.place_hidden.get() {
                                    Icon::EyeOff
                                } else {
                                    Icon::Globe
                                }
                            },
                            icon_box::XSMALL.px(),
                            t.colors.sub,
                        ))
                        .child(caption(move || {
                            if tempo.place_hidden.get() {
                                "•••••".to_string()
                            } else {
                                place.get()
                            }
                        })),
                )
                .child(caption(read(|c| c.lines[0].clone())))
                .child(caption(read(|c| c.lines[1].clone()))),
        )
        .child(
            container()
                .width(fill())
                .padding([size::S3.px(), 0.0, 0.0, 0.0])
                .layout(Flex::row())
                .children(move || {
                    card.get()
                        .map(|c| c.hours)
                        .unwrap_or_default()
                        .into_iter()
                        .map(|hour| {
                            // As wide as each other, so the icons stay put
                            // whatever the hours' names.
                            container()
                                .width(fill())
                                .layout(
                                    Flex::column()
                                        .spacing(size::S1.px())
                                        .cross_alignment(CrossAlignment::Center),
                                )
                                .child(
                                    typed(hour.when, ty::LABEL_XSMALL.tabular())
                                        .color(t.colors.sub)
                                        .nowrap(),
                                )
                                .child(icon(
                                    sky::icon(hour.code, hour.is_day),
                                    icon_box::SMALL.px(),
                                    sky::ink(hour.code, t.colors.accent, t.colors.text),
                                ))
                                .child(
                                    typed(
                                        sky::degrees(hour.temperature),
                                        ty::LABEL_XSMALL.em().tabular(),
                                    )
                                    .color(t.colors.text)
                                    .nowrap(),
                                )
                        })
                        .collect::<Vec<_>>()
                }),
        )
}

const WEEK: [Weekday; 7] = [
    Weekday::Mon,
    Weekday::Tue,
    Weekday::Wed,
    Weekday::Thu,
    Weekday::Fri,
    Weekday::Sat,
    Weekday::Sun,
];

/// The month, from Monday, today marked, with a way to the months around it.
fn month(tempo: Tempo, chosen: RwSignal<Option<NaiveDate>>) -> Container {
    let t = ui();

    // How many months from this one the panel shows.
    let offset = create_signal(0i32);
    let today = create_memo(move || time::date(tempo.now.get(), tempo.zone().as_ref()));

    let first = create_memo(move || {
        let this = today.get().with_day(1).expect("every month has a first");
        let months = Months::new(offset.get().unsigned_abs());

        if offset.get() >= 0 {
            this + months
        } else {
            this - months
        }
    });

    let step = move |by: i32| move || offset.update(|o| *o += by);

    container()
        .width(fill())
        .layout(Flex::column().spacing(size::S2.px()))
        .child(
            container()
                .width(fill())
                .height(Size::Sm.height())
                .layout(
                    Flex::row()
                        .main_alignment(MainAlignment::SpaceBetween)
                        .cross_alignment(CrossAlignment::Center),
                )
                .child(
                    typed(
                        move || {
                            first
                                .get()
                                .format_localized("%B %Y", chrono_locale())
                                .to_string()
                        },
                        ty::TITLE_XSMALL.em(),
                    )
                    .color(t.colors.text)
                    .nowrap(),
                )
                .child(
                    // Today before the arrows: coming and going, it moves
                    // nothing a click is aimed at.
                    container()
                        .layout(centered_row(size::S1.px()))
                        .child(move || {
                            (offset.get() != 0 || chosen.get().is_some()).then(|| {
                                button().label(t!("tempo-today")).on_click(move || {
                                    offset.set(0);
                                    chosen.set(None);
                                })
                            })
                        })
                        .child(
                            button()
                                .icon(Icon::ChevronLeft)
                                .size(Size::Sm)
                                .fill(Fill::Ghost)
                                .on_click(step(-1)),
                        )
                        .child(
                            button()
                                .icon(Icon::ChevronRight)
                                .size(Size::Sm)
                                .fill(Fill::Ghost)
                                .on_click(step(1)),
                        ),
                ),
        )
        .child(
            container()
                .width(fill())
                .layout(Flex::column().spacing(size::S0_5.px()))
                .child(week(WEEK.into_iter().map(weekday).collect()))
                .children(move || {
                    let first = first.get();
                    let today = today.get();
                    let skies: Vec<(NaiveDate, u32)> = tempo.weather.with(|data| {
                        data.as_ref()
                            .map(|data| data.days.iter().map(|d| (d.date, d.code)).collect())
                            .unwrap_or_default()
                    });

                    days(first)
                        .chunks(7)
                        .map(|week_days| {
                            week(
                                week_days
                                    .iter()
                                    .map(|day| {
                                        let sky =
                                            skies.iter().find(|(d, _)| d == day).map(|(_, c)| *c);
                                        cell(*day, first, today, sky, chosen)
                                    })
                                    .collect(),
                            )
                        })
                        .collect::<Vec<_>>()
                }),
        )
}

/// The 42 days a month's grid shows, from the Monday on or before its first.
fn days(first: NaiveDate) -> Vec<NaiveDate> {
    let lead = first.weekday().num_days_from_monday() as i64;
    let start = first - chrono::Duration::days(lead);

    (0..42).map(|i| start + chrono::Duration::days(i)).collect()
}

fn week(cells: Vec<Container>) -> Container {
    container()
        .width(fill())
        .layout(Flex::row().spacing(size::S0_5.px()))
        .children(cells)
}

fn weekday(day: Weekday) -> Container {
    let t = ui();
    let name = NaiveDate::from_isoywd_opt(2000, 20, day)
        .expect("a week of 2000")
        .format_localized("%a", chrono_locale())
        .to_string();

    container()
        .width(fill())
        .height(Size::Xs.height())
        .layout(Flex::column().center())
        .child(
            typed(name, ty::LABEL_XSMALL.em())
                .color(t.colors.dim)
                .nowrap(),
        )
}

/// A day of the grid, with the sky of a forecast one: choosing it shows its
/// weather on the card, and choosing today shows now's again.
fn cell(
    day: NaiveDate,
    first: NaiveDate,
    today: NaiveDate,
    sky: Option<u32>,
    chosen: RwSignal<Option<NaiveDate>>,
) -> Container {
    let t = ui();
    let is_today = day == today;
    let outside = day.month() != first.month();

    let ink = if is_today {
        t.colors.on_accent
    } else if outside {
        t.colors.dim.with_alpha(opacity::DISABLED)
    } else {
        t.colors.text
    };

    let picked = move || !is_today && chosen.get() == Some(day);

    let base = container()
        .width(fill())
        .height(size::S10.px())
        .corners(radius::SMALL.corners())
        .background(
            (move || {
                if is_today {
                    t.colors.accent
                } else if picked() {
                    t.colors.surface2
                } else {
                    Color::TRANSPARENT
                }
            })
            .transition(motion::hover()),
        )
        .layout(
            Flex::column()
                .spacing(size::S0_5.px())
                .main_alignment(MainAlignment::Center)
                .cross_alignment(CrossAlignment::Center),
        )
        .child(
            typed(
                day.day().to_string(),
                if is_today {
                    ty::LABEL_SMALL.em().tabular()
                } else {
                    ty::LABEL_SMALL.tabular()
                },
            )
            .color(ink)
            .nowrap(),
        )
        .child(sky.map(|code| {
            let tint = if is_today {
                t.colors.on_accent
            } else {
                self::sky::ink(code, t.colors.accent, t.colors.sub)
            };

            icon(self::sky::icon(code, true), icon_box::XSMALL.px(), tint)
        }));

    match sky {
        Some(_) => base
            .when_hovered(move |s| {
                s.background(move || {
                    if is_today {
                        t.colors.accent
                    } else {
                        t.colors.surface
                    }
                })
            })
            .on_click(move || chosen.set((!is_today).then_some(day))),
        None => base,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_grid_starts_on_the_monday_before_the_first() {
        let first = NaiveDate::from_ymd_opt(2026, 11, 1).unwrap();
        let grid = days(first);

        assert_eq!(grid.len(), 42);
        assert_eq!(grid[0], NaiveDate::from_ymd_opt(2026, 10, 26).unwrap());
        assert_eq!(grid[0].weekday(), Weekday::Mon);
    }

    #[test]
    fn a_month_starting_on_monday_starts_the_grid() {
        let first = NaiveDate::from_ymd_opt(2026, 6, 1).unwrap();

        assert_eq!(days(first)[0], first);
    }
}
