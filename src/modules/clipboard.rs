use crate::{
    app::{self},
    components::icons::{icon, Icons},
};
use iced::Element;

use super::{Module, OnModulePress};

#[derive(Default, Debug, Clone)]
pub struct Clipboard;

impl Module for Clipboard {
    type Data<'a> = ();

    fn view(&self, _: Self::Data<'_>) -> Option<(Element<app::Message>, Option<OnModulePress>)> {
        Some((
            icon(Icons::Clipboard).into(),
            Some(OnModulePress::Action(app::Message::OpenClipboard)),
        ))
    }
}
