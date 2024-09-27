use crate::{
    app::Message,
    components::icons::{icon, Icons},
    style::HeaderButtonStyle,
};
use iced::{theme, widget::button, Element};

pub fn launcher<'a>() -> Element<'a, Message> {
    button(icon(Icons::Launcher))
        .padding([2, 7])
        .on_press(Message::OpenLauncher)
        .style(theme::Button::custom(HeaderButtonStyle::Full))
        .into()
}

