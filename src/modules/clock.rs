use crate::{config::ClockModuleConfig, theme::AshellTheme};
use chrono::{DateTime, Local};
use iced::{Element, Subscription, time::every, widget::text};
use std::time::Duration;

#[derive(Debug, Clone)]
pub enum Message {
    Update,
    CycleFormat,
}

pub struct Clock {
    config: ClockModuleConfig,
    date: DateTime<Local>,
    current_format_index: usize,
}

impl Clock {
    pub fn new(config: ClockModuleConfig) -> Self {
        Self {
            config,
            date: Local::now(),
            current_format_index: 0,
        }
    }

    pub fn update(&mut self, message: Message) {
        match message {
            Message::Update => {
                self.date = Local::now();
            }
            Message::CycleFormat => {
                if !self.config.formats.is_empty() {
                    self.current_format_index =
                        (self.current_format_index + 1) % self.config.formats.len();
                }
            }
        }
    }

    pub fn view(&'_ self, _: &AshellTheme) -> Element<'_, Message> {
        let format = if !self.config.formats.is_empty() {
            &self.config.formats[self.current_format_index]
        } else {
            &self.config.format
        };
        text(self.date.format(format).to_string()).into()
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

        let current_format = if !self.config.formats.is_empty() {
            &self.config.formats[self.current_format_index]
        } else {
            &self.config.format
        };

        let interval = if second_specifiers
            .iter()
            .any(|&spec| current_format.contains(spec))
        {
            Duration::from_secs(1)
        } else {
            Duration::from_secs(5)
        };

        every(interval).map(|_| Message::Update)
    }
}
