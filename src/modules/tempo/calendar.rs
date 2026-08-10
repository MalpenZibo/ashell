use chrono::{
    DateTime, Datelike, Days, FixedOffset, Local, Months, NaiveDate, NaiveDateTime, TimeZone, Utc,
    Weekday,
};
use chrono_tz::Tz;
use guido::prelude::*;

use super::TempoHandle;
use crate::components::{ButtonKind, ButtonSize, StaticIcon, button, buttons::icon_button};
use crate::config::TempoModuleConfig;
use crate::i18n::chrono_locale;
use crate::theme::ThemeColors;

/// Format a time in the timezone at `tz_index` (FixedOffset like "+02:00",
/// chrono-tz names like "Europe/Rome", falling back to Local). Upstream
/// logic, verbatim.
pub fn time_str(
    config: &TempoModuleConfig,
    format: &str,
    tz_index: usize,
    date: DateTime<Local>,
    utc_datetime: Option<NaiveDateTime>,
) -> String {
    let format_requests_name = format.contains("%Z");
    let utc_now = date.with_timezone(&Utc);
    let naive_utc = utc_datetime.unwrap_or_else(|| utc_now.naive_utc());
    let locale = chrono_locale();

    config
        .timezones
        .get(tz_index)
        .and_then(|tz_name| {
            if !format_requests_name && let Ok(offset) = tz_name.parse::<FixedOffset>() {
                return Some(
                    offset
                        .from_utc_datetime(&naive_utc)
                        .format_localized(format, locale)
                        .to_string(),
                );
            }

            if let Ok(tz) = tz_name.parse::<Tz>() {
                return Some(
                    tz.from_utc_datetime(&naive_utc)
                        .format_localized(format, locale)
                        .to_string(),
                );
            }

            None
        })
        .unwrap_or_else(|| {
            Local
                .from_utc_datetime(&naive_utc)
                .format_localized(format, locale)
                .to_string()
        })
}

fn naive_date(config: &TempoModuleConfig, tz_index: usize, date: DateTime<Local>) -> NaiveDate {
    let utc_now = date.with_timezone(&Utc);

    config
        .timezones
        .get(tz_index)
        .and_then(|tz_name| {
            if let Ok(offset) = tz_name.parse::<FixedOffset>() {
                return Some(offset.from_utc_datetime(&utc_now.naive_utc()).date_naive());
            }

            if let Ok(tz) = tz_name.parse::<Tz>() {
                return Some(tz.from_utc_datetime(&utc_now.naive_utc()).date_naive());
            }

            None
        })
        .unwrap_or_else(|| date.date_naive())
}

/// Calendar column: today header (reset), month grid, timezone list.
pub fn view(handle: TempoHandle, config: TempoModuleConfig) -> impl Widget {
    let theme = expect_context::<ThemeColors>();

    container()
        .width(225)
        .layout(Flex::column().spacing(16))
        .child(header(handle, theme))
        .child(grid(handle, config.clone(), theme))
        .child(timezones(handle, config, theme))
}

fn header(handle: TempoHandle, theme: ThemeColors) -> impl Widget {
    container().width(fill()).child(move || {
        let locale = chrono_locale();
        let date = handle.date.get();
        let resettable = handle.selected_date.with(|d| d.is_some());

        let content = container()
            .layout(Flex::column().spacing(4))
            .child(
                text(date.format_localized("%A", locale).to_string())
                    .color(theme.text)
                    .font_size(12),
            )
            .child(
                text(date.format_localized("%d %B %Y", locale).to_string())
                    .color(theme.text)
                    .font_size(14),
            );

        let mut b = button().size(ButtonSize::Large).content(content);
        if resettable {
            b = b.on_click(move || handle.selected_date.set(None));
        }
        Some(b)
    })
}

fn grid(handle: TempoHandle, config: TempoModuleConfig, theme: ThemeColors) -> impl Widget {
    container().width(fill()).child(move || {
        let locale = chrono_locale();
        let today = naive_date(&config, handle.tz_index.get(), handle.date.get());
        let selected_date = handle.selected_date.get().unwrap_or(today);

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

        // Month navigation
        let prev_date = selected_date.checked_sub_months(Months::new(1));
        let next_date = selected_date.checked_add_months(Months::new(1));
        let nav = container()
            .width(fill())
            .layout(
                Flex::row()
                    .spacing(4)
                    .cross_alignment(CrossAlignment::Center),
            )
            .child(
                icon_button()
                    .icon(StaticIcon::LeftChevron)
                    .on_click(move || handle.selected_date.set(prev_date)),
            )
            .child(
                container()
                    .width(fill())
                    .layout(Flex::row().main_alignment(MainAlignment::Center))
                    .child(
                        text(selected_date.format_localized("%B", locale).to_string())
                            .color(theme.text)
                            .font_size(14),
                    ),
            )
            .child(
                icon_button()
                    .icon(StaticIcon::RightChevron)
                    .on_click(move || handle.selected_date.set(next_date)),
            );

        // Weekday header (Mon-first)
        let mut week_header = container().width(fill()).layout(Flex::row().spacing(4));
        for wd in [
            Weekday::Mon,
            Weekday::Tue,
            Weekday::Wed,
            Weekday::Thu,
            Weekday::Fri,
            Weekday::Sat,
            Weekday::Sun,
        ] {
            week_header = week_header.child(
                container()
                    .width(fill())
                    .layout(Flex::row().main_alignment(MainAlignment::Center))
                    .child(
                        text(
                            NaiveDate::from_isoywd_opt(2000, 20, wd)
                                .expect("valid NaiveDate")
                                .format_localized("%a", locale)
                                .to_string(),
                        )
                        .color(theme.text)
                        .font_size(11),
                    ),
            );
        }

        // Day grid
        let mut grid_col = container().width(fill()).layout(Flex::column().spacing(2));
        for _ in 0..weeks_in_month {
            let mut week_row = container().width(fill()).layout(Flex::row().spacing(4));
            for _ in 0..7 {
                let day = current;
                current = current.succ_opt().unwrap_or(current);

                let color = if day == today {
                    theme.success
                } else if day == selected_date {
                    theme.primary
                } else if day.month0() != current_month {
                    Color::rgba(theme.text.r, theme.text.g, theme.text.b, 0.2)
                } else {
                    theme.text
                };

                let mut cell = container()
                    .width(fill())
                    .height(24)
                    .corner_radius(8)
                    .layout(
                        Flex::row()
                            .main_alignment(MainAlignment::Center)
                            .cross_alignment(CrossAlignment::Center),
                    )
                    .child(
                        text(day.format_localized("%-d", locale).to_string())
                            .color(color)
                            .font_size(11),
                    );
                if day != today {
                    cell = cell
                        .hover_state(|s| s.lighter(0.1))
                        .on_click(move || handle.selected_date.set(Some(day)));
                }
                week_row = week_row.child(cell);
            }
            grid_col = grid_col.child(week_row);
        }

        Some(
            container()
                .width(fill())
                .layout(Flex::column().spacing(12))
                .child(nav)
                .child(week_header)
                .child(grid_col),
        )
    })
}

fn timezones(handle: TempoHandle, config: TempoModuleConfig, theme: ThemeColors) -> impl Widget {
    container().width(fill()).child(move || {
        if config.timezones.is_empty() {
            return None;
        }
        let active = handle.tz_index.get();
        let date = handle.date.get();

        let mut col = container().width(fill()).layout(Flex::column().spacing(2));
        for (index, tz_name) in config.timezones.iter().enumerate() {
            let label = format!(
                "{}: {}",
                tz_name,
                time_str(&config, "%d %h %R", index, date, None)
            );
            if index == active {
                col = col.child(
                    container()
                        .width(fill())
                        .padding([2, 8])
                        .child(text(label).color(theme.success).font_size(12)),
                );
            } else {
                col = col.child(
                    button()
                        .kind(ButtonKind::Transparent)
                        .fill_width(true)
                        .content(text(label).color(theme.text).font_size(12))
                        .on_click(move || handle.tz_index.set(index)),
                );
            }
        }
        Some(col)
    })
}
