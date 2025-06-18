use crate::{
    app::{self, App, Message},
    components::icons::{Icons, icon},
};
use iced::{Element, window::Id};

use super::{Module, Module2, OnModulePress};

#[derive(Default, Debug, Clone)]
pub struct AppLauncher;

impl Module2<AppLauncher> for App {
    fn view(&self, _: Id) -> Option<(Element<app::Message>, Option<OnModulePress>)> {
        if self.config.app_launcher_cmd.is_some() {
            Some((
                icon(Icons::AppLauncher).into(),
                Some(OnModulePress::Action(Box::new(Message::OpenLauncher))),
            ))
        } else {
            None
        }
    }
}
