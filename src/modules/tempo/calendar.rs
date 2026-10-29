use chrono::{
    Datelike, Days, FixedOffset, Local, Months, NaiveDate, NaiveDateTime, TimeZone, Utc, Weekday,
};
use chrono_tz::Tz;
use iced::{
    Border, Element, Length, Theme,
    alignment::{Horizontal, Vertical},
    widget::{Column, Row, column, container, row, text},
};

use crate::{
    components::{
        ButtonHierarchy, ButtonKind, ButtonSize, divider,
        icons::{StaticIcon, icon_button},
        styled_button,
    },
    theme::{AshellTheme, Paint, use_theme},
};
use ashell_i18n::chrono_locale;

use super::{Message, Tempo};

impl Tempo {
    pub(super) fn time_str(
        &'_ self,
        format: &str,
        timezone_index: usize,
        utc_datetime: Option<NaiveDateTime>,
    ) -> String {
        let format_requests_name = format.contains("%Z");
        let utc_now = self.date.with_timezone(&Utc);
        let naive_utc = utc_datetime.unwrap_or_else(|| utc_now.naive_utc());
        let locale = chrono_locale();

        self.config
            .timezones
            .get(timezone_index)
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

    pub(super) fn naive_date(&'_ self, timezone_index: usize) -> NaiveDate {
        let utc_now = self.date.with_timezone(&Utc);

        self.config
            .timezones
            .get(timezone_index)
            .and_then(|tz_name| {
                if let Ok(offset) = tz_name.parse::<FixedOffset>() {
                    return Some(offset.from_utc_datetime(&utc_now.naive_utc()).date_naive());
                }

                if let Ok(tz) = tz_name.parse::<Tz>() {
                    return Some(tz.from_utc_datetime(&utc_now.naive_utc()).date_naive());
                }

                None
            })
            .unwrap_or_else(|| self.date.date_naive())
    }

    pub(super) fn calendar<'a>(&'a self) -> Element<'a, Message> {
        use_theme(|theme| self.calendar_with_theme(theme))
    }

    fn calendar_with_theme<'a>(&'a self, theme: &AshellTheme) -> Element<'a, Message> {
        let locale = chrono_locale();
        let today = self.naive_date(self.current_timezone_index);
        let selected_date = self.selected_date.unwrap_or(today);

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

        let muted = theme.palette.text.scale_alpha(0.55);
        let faint = theme.palette.text.scale_alpha(0.3);

        let title: Element<'a, Message> = container(
            text(selected_date.format_localized("%B %Y", locale).to_string())
                .size(theme.font_size.md)
                .align_x(Horizontal::Center),
        )
        .width(Length::Fill)
        .center_x(Length::Fill)
        .into();

        let header = row![
            icon_button::<Message>(StaticIcon::LeftChevron)
                .kind(ButtonKind::Transparent)
                .size(ButtonSize::Small)
                .on_press(Message::ChangeSelectDate(
                    selected_date.checked_sub_months(Months::new(1)),
                )),
            styled_button(title)
                .kind(ButtonKind::Transparent)
                .size(ButtonSize::Small)
                .width(Length::Fill)
                .on_press(Message::ChangeSelectDate(None)),
            icon_button::<Message>(StaticIcon::RightChevron)
                .kind(ButtonKind::Transparent)
                .size(ButtonSize::Small)
                .on_press(Message::ChangeSelectDate(
                    selected_date.checked_add_months(Months::new(1))
                ))
        ]
        .width(Length::Fill)
        .spacing(theme.space.xxs)
        .align_y(Vertical::Center);

        let weekdays = Row::with_children(
            [
                Weekday::Mon,
                Weekday::Tue,
                Weekday::Wed,
                Weekday::Thu,
                Weekday::Fri,
                Weekday::Sat,
                Weekday::Sun,
            ]
            .into_iter()
            .map(|i| {
                text(
                    NaiveDate::from_isoywd_opt(2000, 20, i)
                        .expect("valid NaiveDate")
                        .format_localized("%a", locale)
                        .to_string(),
                )
                .size(theme.font_size.xs)
                .color(muted)
                .align_x(Horizontal::Center)
                .width(Length::Fill)
                .into()
            })
            .collect::<Vec<Element<'a, Message>>>(),
        )
        .width(Length::Fill);

        let grid = Column::with_children(
            (0..weeks_in_month)
                .map(|_| {
                    Row::with_children(
                        (0..7)
                            .map(|_| {
                                let day = current;
                                current = current.succ_opt().unwrap_or(current);
                                day_cell(
                                    day,
                                    today,
                                    selected_date,
                                    current_month,
                                    theme.font_size.sm,
                                    faint,
                                )
                            })
                            .collect::<Vec<Element<'a, Message>>>(),
                    )
                    .spacing(2.0)
                    .width(Length::Fill)
                    .into()
                })
                .collect::<Vec<Element<'a, Message>>>(),
        )
        .width(Length::Fill)
        .spacing(2.0);

        let mut content = column![header, weekdays, grid]
            .spacing(theme.space.sm)
            .width(Length::Fixed(240.0));

        if !self.config.timezones.is_empty() {
            let radius = theme.radius.sm;
            let rows = Column::with_children(
                self.config
                    .timezones
                    .iter()
                    .enumerate()
                    .map(|(index, tz_name)| {
                        let selected = self.current_timezone_index == index;
                        let content: Element<'a, Message> = row![
                            text(tz_name).size(theme.font_size.sm).width(Length::Fill),
                            text(self.time_str("%R", index, None))
                                .size(theme.font_size.sm)
                                .color_maybe(if selected { None } else { Some(muted) }),
                        ]
                        .spacing(theme.space.sm)
                        .align_y(Vertical::Center)
                        .into();

                        if selected {
                            container(content)
                                .padding([theme.space.xxs, theme.space.sm])
                                .width(Length::Fill)
                                .style(move |theme: &Theme| container::Style {
                                    background: Some(
                                        Paint::surface(
                                            theme,
                                            theme.extended_palette().background.weak.color,
                                        )
                                        .into(),
                                    ),
                                    border: Border::default().rounded(radius),
                                    ..Default::default()
                                })
                                .into()
                        } else {
                            styled_button(content)
                                .kind(ButtonKind::Transparent)
                                .size(ButtonSize::Small)
                                .width(Length::Fill)
                                .on_press(Message::SetTimezone(index))
                                .into()
                        }
                    })
                    .collect::<Vec<Element<'a, Message>>>(),
            )
            .spacing(theme.space.xxs);

            content = content.push(
                column![divider(), rows]
                    .spacing(theme.space.sm)
                    .width(Length::Fill),
            );
        }

        content.into()
    }
}

fn day_cell<'a>(
    day: NaiveDate,
    today: NaiveDate,
    selected: NaiveDate,
    current_month: u32,
    font_size: f32,
    faint: iced::Color,
) -> Element<'a, Message> {
    let locale = chrono_locale();
    let is_today = day == today;
    let is_selected = day == selected;

    let (kind, hierarchy) = if is_selected {
        (ButtonKind::Solid, ButtonHierarchy::Primary)
    } else if is_today {
        (ButtonKind::Outline, ButtonHierarchy::Primary)
    } else {
        (ButtonKind::Transparent, ButtonHierarchy::Secondary)
    };

    let color = if is_today || is_selected || day.month0() == current_month {
        None
    } else {
        Some(faint)
    };

    styled_button(Element::from(
        text(day.format_localized("%-d", locale).to_string())
            .size(font_size)
            .align_x(Horizontal::Center)
            .color_maybe(color),
    ))
    .kind(kind)
    .hierarchy(hierarchy)
    .size(ButtonSize::Small)
    .width(Length::Fill)
    .height(Length::Fixed(28.0))
    .on_press(Message::ChangeSelectDate(Some(day)))
    .into()
}
