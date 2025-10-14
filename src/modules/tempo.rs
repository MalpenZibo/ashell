use crate::{
    components::icons::{StaticIcon, icon},
    config::ClockModuleConfig,
    theme::AshellTheme,
};
use chrono::{DateTime, Datelike, Days, Local, Months, NaiveDate, Weekday};
use iced::{
    Element, Length, Subscription,
    alignment::{Horizontal, Vertical},
    time::every,
    widget::{Column, Row, button, column, row, text},
};
use std::time::Duration;

#[derive(Debug, Clone)]
pub enum Message {
    Update,
    ChangeSelectDate(Option<NaiveDate>),
}

pub struct Tempo {
    config: ClockModuleConfig,
    date: DateTime<Local>,
    selected_date: Option<NaiveDate>,
}

impl Tempo {
    pub fn new(config: ClockModuleConfig) -> Self {
        Self {
            config,
            date: Local::now(),
            selected_date: None,
        }
    }

    pub fn update(&mut self, message: Message) {
        match message {
            Message::Update => {
                self.date = Local::now();
            }
            Message::ChangeSelectDate(selected_date) => self.selected_date = selected_date,
        }
    }

    pub fn view(&'_ self, _: &AshellTheme) -> Element<'_, Message> {
        text(self.date.format(&self.config.format).to_string()).into()
    }

    pub fn menu_view<'a>(&'a self, theme: &'a AshellTheme) -> Element<'a, Message> {
        let selected_date = self.selected_date.unwrap_or(self.date.date_naive());

        let current_month = selected_date.month0();
        let first_day_month = selected_date.with_day0(0).unwrap_or_default();
        let day_of_week_first_day = first_day_month.weekday();

        let mut current = first_day_month
            .checked_sub_days(Days::new(day_of_week_first_day.number_from_monday() as u64))
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

        let calendar = Column::new()
            .push(
                row!(
                    button(icon(StaticIcon::LeftChevron))
                        .on_press(Message::ChangeSelectDate(
                            selected_date.checked_sub_months(Months::new(1)),
                        ))
                        .padding([theme.space.xs, theme.space.md])
                        .style(theme.settings_button_style()),
                    text(selected_date.format("%B").to_string())
                        .size(theme.font_size.md)
                        .width(Length::Fill)
                        .align_x(Horizontal::Center),
                    button(icon(StaticIcon::RightChevron))
                        .on_press(Message::ChangeSelectDate(
                            selected_date.checked_add_months(Months::new(1))
                        ))
                        .padding([theme.space.xs, theme.space.md])
                        .style(theme.settings_button_style())
                )
                .width(Length::Fill)
                .align_y(Vertical::Center),
            )
            .push(
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
                        text(i.to_string())
                            .align_x(Horizontal::Center)
                            .width(Length::Fill)
                            .into()
                    })
                    .collect::<Vec<Element<'a, Message>>>(),
                )
                .width(Length::Fill)
                .spacing(theme.space.sm),
            )
            .push(Column::with_children(
                (0..weeks_in_month)
                    .map(|_| {
                        Row::with_children(
                            (0..7)
                                .map(|_| {
                                    let day = current;
                                    current = current.succ_opt().unwrap_or(current);

                                    button(
                                        text(day.format("%d").to_string())
                                            .align_x(Horizontal::Center)
                                            .color_maybe({
                                                if day == self.date.date_naive() {
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
                                    )
                                    .on_press_maybe(if day != self.date.date_naive() {
                                        Some(Message::ChangeSelectDate(Some(day)))
                                    } else {
                                        None
                                    })
                                    .width(Length::Fill)
                                    .style(theme.ghost_button_style())
                                    .into()
                                })
                                .collect::<Vec<Element<'a, Message>>>(),
                        )
                        .spacing(theme.space.sm)
                        .width(Length::Fill)
                        .into()
                    })
                    .collect::<Vec<Element<'a, Message>>>(),
            ))
            .spacing(theme.space.md)
            .width(Length::Fixed(400.));

        column!(
            button(
                column!(
                    text(self.date.format("%A").to_string()).size(theme.font_size.md),
                    text(self.date.format("%d %B %Y").to_string()).size(theme.font_size.lg),
                )
                .spacing(theme.space.xs)
            )
            .on_press_maybe(if self.selected_date.is_some() {
                Some(Message::ChangeSelectDate(None))
            } else {
                None
            })
            .style(theme.ghost_button_style()),
            calendar
        )
        .spacing(theme.space.lg)
        .into()
    }

    pub fn subscription(&self) -> Subscription<Message> {
        let second_specifiers = [
            "%S",  // Seconds (00-60)
            "%T",  // Hour:Minute:Second
            "%X",  // Locale time representation with seconds
            "%r",  // 12-hour clock time with seconds
            "%:z", // UTC offset with seconds
            "%s",  // Unix timestamp (seconds since epoch)
        ];
        let interval = if second_specifiers
            .iter()
            .any(|&spec| self.config.format.contains(spec))
        {
            Duration::from_secs(1)
        } else {
            Duration::from_secs(5)
        };

        every(interval).map(|_| Message::Update)
    }
}
