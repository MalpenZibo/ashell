use crate::{
    app::Message,
    components::icons::{icon, Icons},
};
use iced::Element;

use super::{Module, OnModulePress};

#[derive(Default, Debug, Clone)]
pub struct Launcher;

impl Module for Launcher {
    type Data<'a> = ();

    fn view<'a>(&self, _: Self::Data<'a>) -> Option<(Element<Message>, Option<OnModulePress>)> {
        Some((
            icon(Icons::Launcher).into(),
            Some(OnModulePress::Action(Message::OpenLauncher)),
        ))
    }
}
