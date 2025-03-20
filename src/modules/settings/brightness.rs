use crate::{
    components::icons::{Icons, icon},
    services::{
        ServiceEvent,
        brightness::{BrightnessData, BrightnessService},
    },
};
use iced::{
    Alignment, Element, Length,
    widget::{container, row, slider},
};

use super::Message;

#[derive(Debug, Clone)]
pub enum BrightnessMessage {
    Event(ServiceEvent<BrightnessService>),
    Change(u32),
}

impl BrightnessData {
    pub fn brightness_slider(&self) -> Element<Message> {
        row!(
            container(icon(Icons::Brightness)).padding([8, 11]),
            slider(0..=100, self.current * 100 / self.max, |v| {
                Message::Brightness(BrightnessMessage::Change(v * self.max / 100))
            })
            .step(1_u32)
            .width(Length::Fill),
        )
        .align_y(Alignment::Center)
        .spacing(8)
        .into()
    }
}
