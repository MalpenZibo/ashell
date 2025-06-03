use crate::{
    app::{self, Message},
    components::icons::{Icons, icon},
};
use iced::Element;

use super::{Module, OnModulePress};

#[derive(Default, Debug, Clone)]
pub struct AppLauncher;

impl Module for AppLauncher {
    type ViewData<'a> = &'a Option<String>;
    type SubscriptionData<'a> = ();

    fn view(
        &self,
        config: Self::ViewData<'_>,
    ) -> Option<(Element<app::Message>, Option<OnModulePress>)> {
        if config.is_some() {
            Some((
                icon(Icons::AppLauncher).into(),
                Some(OnModulePress::Action(Box::new(Message::OpenLauncher))),
            ))
        } else {
            None
        }
    }
}
