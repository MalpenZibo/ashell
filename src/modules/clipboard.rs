use crate::{
    app::Message,
    components::icons::{icon, Icons},
    style::HeaderButtonStyle,
};
use iced::{widget::button, Element};

pub fn clipboard<'a>() -> Element<'a, Message> {
    button(icon(Icons::Clipboard))
        .padding([2, 7])
        .on_press(Message::OpenClipboard)
        .style(HeaderButtonStyle::Full.into_style())
        .into()
}
