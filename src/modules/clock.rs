use crate::app;

use super::{Module, OnModulePress};
use chrono::{DateTime, Local};
use iced::{Element, Subscription, time::every, widget::text};
use std::time::Duration;

pub struct Clock {
    date: DateTime<Local>,
}

impl Default for Clock {
    fn default() -> Self {
        Self { date: Local::now() }
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    Update,
}

impl Clock {
    pub fn update(&mut self, message: Message) {
        match message {
            Message::Update => {
                self.date = Local::now();
            }
        }
    }
}

impl Module for Clock {
    type ViewData<'a> = &'a str;
    type SubscriptionData<'a> = &'a str;
    fn view(
        &self,
        format: Self::ViewData<'_>,
    ) -> Option<(Element<app::Message>, Option<OnModulePress>)> {
        Some((text(self.date.format(format).to_string()).into(), None))
    }

    fn subscription(
        &self,
        format: Self::SubscriptionData<'_>,
    ) -> Option<Subscription<app::Message>> {
        let second_specifiers = [
            "%S",  // Seconds (00-60)
            "%T",  // Hour:Minute:Second
            "%X",  // Locale time representation with seconds
            "%r",  // 12-hour clock time with seconds
            "%:z", // UTC offset with seconds
            "%s",  // Unix timestamp (seconds since epoch)
        ];
        let interval = if second_specifiers.iter().any(|&spec| format.contains(spec)) {
            Duration::from_secs(1)
        } else {
            Duration::from_secs(5)
        };

        Some(
            every(interval)
                .map(|_| Message::Update)
                .map(app::Message::Clock),
        )
    }
}
