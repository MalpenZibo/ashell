use iced::{widget::button, Element};
use crate::{
    components::icons::{icon, Icons},
    style::HeaderButtonStyle,
};

#[derive(Clone, Debug)]
pub enum Message {
    OpenLauncher,
}

pub fn launcher<'a>() -> Element<'a, Message> {
    button(icon(Icons::Launcher))
        .on_press(Message::OpenLauncher)
        .style(iced::theme::Button::custom(HeaderButtonStyle))
        .into()
}
