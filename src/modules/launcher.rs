use crate::{
    components::icons::{icon, Icons},
    style::HeaderButtonStyle,
};
use iced::{
    widget::{button, container},
    Element,
};

#[derive(Clone, Debug)]
pub enum Message {
    OpenLauncher,
}

pub fn launcher<'a>() -> Element<'a, Message> {
    button(container(icon(Icons::Launcher).size(18)))
        .padding([0, 6])
        .on_press(Message::OpenLauncher)
        .style(iced::theme::Button::custom(HeaderButtonStyle::Full))
        .into()
}
