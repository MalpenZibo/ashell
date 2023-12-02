use crate::style::left_header_pills;
use chrono::Local;
use iced::{
    widget::{container, text},
    Element,
};
use std::time::Duration;

fn get_date() -> String {
    let local = Local::now();
    local.format("%a %d %b %R").to_string()
}

pub struct Clock {
    date: String,
}

#[derive(Debug, Clone)]
pub enum Message {
    Update,
}

impl Clock {
    pub fn new() -> Self {
        Self { date: get_date() }
    }

    pub fn update(&mut self, message: Message) {
        match message {
            Message::Update => {
                self.date = get_date();
            }
        }
    }

    pub fn view(&self) -> Element<Message> {
        container(text(&self.date))
            .padding([4, 8])
            .style(left_header_pills)
            .into()
    }

    pub fn subscription(&self) -> iced::Subscription<Message> {
        iced::time::every(Duration::from_secs(20)).map(|_| Message::Update)
    }
}
