use crate::{
    app::Message,
    components::icons::{icon, Icons},
    style::HeaderButtonStyle,
};
use iced::{theme, widget::button, Element};

pub fn clipboard<'a>() -> Element<'a, Message> {
    button(icon(Icons::Clipboard))
        .padding([2, 7])
        .on_press(Message::OpenClipboard)
        .style(theme::Button::custom(HeaderButtonStyle::Full))
        .into()
}
