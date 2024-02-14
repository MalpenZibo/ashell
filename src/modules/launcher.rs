use crate::{
    components::icons::{icon, Icons},
    style::HeaderButtonStyle,
};
use iced::{widget::button, Element, Length};

#[derive(Clone, Debug)]
pub enum Message {
    OpenLauncher,
}

pub fn launcher<'a>() -> Element<'a, Message> {
    button(icon(Icons::Launcher))
        .padding([2, 7])
        .on_press(Message::OpenLauncher)
        .style(iced::theme::Button::custom(HeaderButtonStyle::Full))
        .into()
}
