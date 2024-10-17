use crate::style::left_header_pills;
use chrono::{DateTime, Local};
use iced::{
    time::every,
    widget::{container, text},
    Element, Subscription,
};
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

    pub fn view(&self, format: &str) -> Element<Message> {
        container(text(self.date.format(format).to_string()))
            .padding([2, 8])
            .style(left_header_pills)
            .into()
    }

    pub fn subscription(&self) -> Subscription<Message> {
        every(Duration::from_secs(5)).map(|_| Message::Update)
    }
}
