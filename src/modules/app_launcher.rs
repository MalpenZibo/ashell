use super::{Module2, OnModulePress};
use crate::{
    app::{self, App, Message},
    components::icons::{Icons, icon},
};
use iced::Element;

#[derive(Default, Debug, Clone)]
pub struct AppLauncher;

impl Module2<AppLauncher> for App {
    type ViewData<'a> = ();
    type MenuViewData<'a> = ();
    type SubscriptionData<'a> = ();

    fn view(
        &self,
        _: Self::ViewData<'_>,
    ) -> Option<(Element<app::Message>, Option<OnModulePress>)> {
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
