use iced::{
    widget::{container, row, slider},
    Alignment, Command, Element, Length, Subscription,
};

use crate::{
    components::icons::{icon, Icons},
    utils::Commander,
};

use super::Message;

#[derive(Debug, Clone)]
pub enum BrightnessMessage {
    Changed(f64, bool),
}

pub struct Brightness {
    commander: Commander<f64>,
    value: i32,
}

impl Brightness {
    pub fn new() -> Self {
        Self {
            commander: Commander::new(),
            value: 0,
        }
    }

    pub fn update<Message>(&mut self, msg: BrightnessMessage) -> Command<Message> {
        match msg {
            BrightnessMessage::Changed(value, externa_source) => {
                if (value - (self.value as f64 / 100.)).abs() > 0.01 {
                    self.value = (value * 100.).round() as i32;
                    if !externa_source {
                        self.commander.send(value).unwrap();
                    }
                }
                iced::Command::none()
            }
        }
    }

    pub fn brightness_slider<'a>(&self) -> Element<'a, Message> {
        row!(
            container(icon(Icons::Brightness)).padding([8, 11]),
            slider(0..=100, self.value, |v| Message::Brightness(
                BrightnessMessage::Changed(v as f64 / 100., false)
            ))
            .step(1)
            .width(Length::Fill),
        )
        .align_items(Alignment::Center)
        .spacing(8)
        .into()
    }

    pub fn subscription(&self) -> Subscription<BrightnessMessage> {
        crate::utils::brightness::subscription(self.commander.give_receiver())
    }
}
