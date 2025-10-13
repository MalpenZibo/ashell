use crate::{
    components::icons::{StaticIcon, icon},
    config::ClockModuleConfig,
    theme::AshellTheme,
};
use chrono::{Date, DateTime, Datelike, Days, Local};
use iced::{
    Element, Length, Subscription, Theme,
    alignment::{Horizontal, Vertical},
    time::every,
    widget::{Column, Row, button, column, container, row, text},
};
use std::time::Duration;

#[derive(Debug, Clone)]
pub enum Message {
    Update,
    ChangeSelectDate,
}

pub struct Tempo {
    config: ClockModuleConfig,
    date: DateTime<Local>,
}

impl Tempo {
    pub fn new(config: ClockModuleConfig) -> Self {
        Self {
            config,
            date: Local::now(),
        }
    }

    pub fn update(&mut self, message: Message) {
        match message {
            Message::Update => {
                self.date = Local::now();
            }
            Message::ChangeSelectDate => {
                // Currently does nothing, but could be extended to change the date
            }
        }
    }

    pub fn view(&'_ self, _: &AshellTheme) -> Element<'_, Message> {
        text(self.date.format(&self.config.format).to_string()).into()
    }

    pub fn menu_view<'a>(&'a self, theme: &'a AshellTheme) -> Element<'_, Message> {
        let current_month = self.date.date_naive().month0();
        let first_day_month = self.date.date_naive().with_day0(0).unwrap();
        let day_of_week_first_day = first_day_month.weekday();

        let mut start = first_day_month
            .checked_sub_days(Days::new(day_of_week_first_day.number_from_monday() as u64))
            .unwrap();

        let mut weeks: Vec<Vec<Element<'_, Message>>> = Vec::new();
        weeks.push(
            ["M", "T", "W", "T", "F", "S", "S"]
                .into_iter()
                .map(|i| {
                    text(i)
                        .align_x(Horizontal::Center)
                        .width(Length::Fixed(40.0))
                        .into()
                })
                .collect::<Vec<_>>(),
        );

        for w in 0..5 {
            let mut week_row: Vec<Element<'_, Message>> = Vec::new();
            for d in 0..7 {
                week_row.push(
                    button(
                        text(start.format("%d").to_string())
                            .align_x(Horizontal::Center)
                            .color_maybe({
                                if start == self.date.date_naive() {
                                    Some(theme.iced_theme.palette().success)
                                } else if start.month0() != current_month {
                                    Some(theme.iced_theme.palette().text.scale_alpha(0.2))
                                } else {
                                    None
                                }
                            })
                            .width(Length::Fixed(20.0)),
                    )
                    .on_press_maybe(if start != self.date.date_naive() {
                        Some(Message::ChangeSelectDate)
                    } else {
                        None
                    })
                    .style(theme.ghost_button_style())
                    .into(),
                );

                start = start.succ_opt().unwrap();
            }

            weeks.push(week_row);
        }

        let test = Column::with_children(
            weeks
                .into_iter()
                .map(|days| Row::with_children(days).spacing(theme.space.sm).into())
                .collect::<Vec<_>>(),
        );

        column!(
            text(self.date.format("%A").to_string()).size(theme.font_size.md),
            text(self.date.format("%d %B %Y").to_string()).size(theme.font_size.lg),
            row!(
                button(icon(StaticIcon::LeftChevron))
                    .on_press(Message::ChangeSelectDate)
                    .padding([theme.space.xs, theme.space.md])
                    .style(theme.settings_button_style()),
                text(self.date.format("%B").to_string())
                    .size(theme.font_size.md)
                    .width(Length::Fill)
                    .align_x(Horizontal::Center),
                button(icon(StaticIcon::RightChevron))
                    .on_press(Message::ChangeSelectDate)
                    .padding([theme.space.xs, theme.space.md])
                    .style(theme.settings_button_style())
            )
            .align_y(Vertical::Center),
            test
        )
        .spacing(theme.space.md)
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
