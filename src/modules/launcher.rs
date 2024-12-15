use crate::{
    app::Message,
    components::icons::{icon, Icons},
    style::HeaderButtonStyle,
};
use iced::{widget::button, Element};

pub fn launcher<'a>() -> Element<'a, Message> {
    button(icon(Icons::Launcher))
        .padding([2, 7])
        .on_press(Message::OpenLauncher)
        .style(HeaderButtonStyle::Full.into_style())
        .into()
}
