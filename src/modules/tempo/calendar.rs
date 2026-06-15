use chrono::{
    Datelike, Days, FixedOffset, Local, Months, NaiveDate, NaiveDateTime, TimeZone, Utc, Weekday,
};
use chrono_tz::Tz;
use iced::{
    Element, Length, Theme,
    alignment::{Horizontal, Vertical},
    widget::{Column, Row, column, container, row, text},
};

use crate::{
    components::{
        ButtonKind, ButtonSize,
        icons::{StaticIcon, icon_button},
        styled_button,
    },
    i18n::chrono_locale,
    theme::{AshellTheme, use_theme},
};

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
        let selected_date = self
            .selected_date
            .unwrap_or(self.naive_date(self.current_timezone_index));

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

        let calendar = column![
            row![
                icon_button::<Message>(StaticIcon::LeftChevron)
                    .kind(ButtonKind::Solid)
                    .on_press(Message::ChangeSelectDate(
                        selected_date.checked_sub_months(Months::new(1)),
                    )),
                text(selected_date.format_localized("%B", locale).to_string())
                    .size(theme.font_size.md)
                    .width(Length::Fill)
                    .align_x(Horizontal::Center),
                icon_button::<Message>(StaticIcon::RightChevron)
                    .kind(ButtonKind::Solid)
                    .on_press(Message::ChangeSelectDate(
                        selected_date.checked_add_months(Months::new(1))
                    ))
            ]
            .width(Length::Fill)
            .align_y(Vertical::Center),
            Row::with_children(
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
                    .align_x(Horizontal::Center)
                    .width(Length::Fill)
                    .into()
                })
                .collect::<Vec<Element<'a, Message>>>(),
            )
            .width(Length::Fill)
            .spacing(theme.space.sm),
            Column::with_children(
                (0..weeks_in_month)
                    .map(|_| {
                        Row::with_children(
                            (0..7)
                                .map(|_| {
                                    let day = current;
                                    current = current.succ_opt().unwrap_or(current);

                                    styled_button(Element::from(
                                        text(day.format_localized("%-d", locale).to_string())
                                            .align_x(Horizontal::Center)
                                            .color_maybe({
                                                if day
                                                    == self.naive_date(self.current_timezone_index)
                                                {
                                                    Some(theme.iced_theme.palette().success)
                                                } else if day == selected_date {
                                                    Some(theme.iced_theme.palette().primary)
                                                } else if day.month0() != current_month {
                                                    Some(
                                                        theme
                                                            .iced_theme
                                                            .palette()
                                                            .text
                                                            .scale_alpha(0.2),
                                                    )
                                                } else {
                                                    None
                                                }
                                            }),
                                    ))
                                    .on_press_maybe(
                                        if day != self.naive_date(self.current_timezone_index) {
                                            Some(Message::ChangeSelectDate(Some(day)))
                                        } else {
                                            None
                                        },
                                    )
                                    .size(ButtonSize::Small)
                                    .width(Length::Fill)
                                    .into()
                                })
                                .collect::<Vec<Element<'a, Message>>>(),
                        )
                        .spacing(theme.space.xs)
                        .width(Length::Fill)
                        .into()
                    })
                    .collect::<Vec<Element<'a, Message>>>(),
            ),
        ]
        .spacing(theme.space.md);

        let timezones = Column::with_children(
            self.config
                .timezones
                .iter()
                .enumerate()
                .map(|(index, tz_name)| {
                    if self.current_timezone_index == index {
                        container(
                            text(format!(
                                "{}: {}",
                                tz_name,
                                self.time_str("%d %h %R", index, None)
                            ))
                            .wrapping(text::Wrapping::Word),
                        )
                        .padding([theme.space.xxs, theme.space.sm])
                        .width(Length::Fill)
                        .style(|theme: &Theme| container::Style {
                            text_color: Some(theme.palette().success),
                            ..Default::default()
                        })
                        .into()
                    } else {
                        styled_button(format!(
                            "{}: {}",
                            tz_name,
                            self.time_str("%d %h %R", index, None)
                        ))
                        .width(Length::Fill)
                        .on_press(Message::SetTimezone(index))
                        .into()
                    }
                })
                .collect::<Vec<Element<'a, Message>>>(),
        );

        column!(
            styled_button(Element::from(
                column!(
                    text(self.date.format_localized("%A", locale).to_string())
                        .size(theme.font_size.sm),
                    text(self.date.format_localized("%d %B %Y", locale).to_string())
                        .size(theme.font_size.md),
                )
                .spacing(theme.space.xs),
            ),)
            .size(ButtonSize::Large)
            .kind(ButtonKind::Outline)
            .on_press_maybe(if self.selected_date.is_some() {
                Some(Message::ChangeSelectDate(None))
            } else {
                None
            })
            .width(Length::Fill),
            calendar,
            timezones,
        )
        .spacing(theme.space.lg)
        .width(225)
        .into()
    }
}
