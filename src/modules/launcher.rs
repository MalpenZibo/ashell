use crate::{
    app::{self, Message},
    components::icons::{icon, Icons},
};
use iced::Element;

use super::{Module, OnModulePress};

#[derive(Default, Debug, Clone)]
pub struct Launcher;

impl Module for Launcher {
    type Data<'a> = ();

    fn view(&self, _: Self::Data<'_>) -> Option<(Element<app::Message>, Option<OnModulePress>)> {
        Some((
            icon(Icons::Launcher).into(),
            Some(OnModulePress::Action(Message::OpenLauncher)),
        ))
    }
}
