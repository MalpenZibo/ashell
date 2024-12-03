use crate::{
    components::icons::{icon, Icons},
    services::{
        brightness::{BrightnessData, BrightnessService},
        ServiceEvent,
    },
};
use iced::{
    widget::{container, row, slider},
    Alignment, Element, Length,
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
