use crate::{
    app::Message,
    components::icons::{icon, Icons},
};
use iced::Element;

use super::{Module, OnModulePress};

#[derive(Default, Debug, Clone)]
pub struct Clipboard;

impl Module for Clipboard {
    type Data<'a> = ();

    fn view<'a>(&self, _: Self::Data<'a>) -> Option<(Element<Message>, Option<OnModulePress>)> {
        Some((
            icon(Icons::Clipboard).into(),
            Some(OnModulePress::Action(Message::OpenClipboard)),
        ))
    }
}
