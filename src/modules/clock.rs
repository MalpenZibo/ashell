use crate::style::left_header_pills;
use chrono::{DateTime, Local};
use iced::{
    widget::{container, text},
    Element,
};
use std::time::Duration;

pub struct Clock {
    date: DateTime<Local>,
}

#[derive(Debug, Clone)]
pub enum Message {
    Update,
}

impl Clock {
    pub fn new() -> Self {
        Self { date: Local::now() }
    }

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

    pub fn subscription(&self) -> iced::Subscription<Message> {
        iced::time::every(Duration::from_secs(20)).map(|_| Message::Update)
    }
}
