use crate::{config::ClockModuleConfig, theme::AshellTheme};
use chrono::{DateTime, Local};
use iced::{Element, Subscription, time::every, widget::text};
use log::warn;
use std::time::Duration;

#[derive(Debug, Clone)]
pub enum Message {
    Update,
}

pub struct Clock {
    config: ClockModuleConfig,
    date: DateTime<Local>,
}

impl Clock {
    pub fn new(config: ClockModuleConfig) -> Self {
        warn!(
            "Clock module is deprecated and will be removed in a future release. Please migrate to the Tempo module."
        );
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
        }
    }

    pub fn view(&'_ self, _: &AshellTheme) -> Element<'_, Message> {
        text(self.date.format(&self.config.format).to_string()).into()
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
