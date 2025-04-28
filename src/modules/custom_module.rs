use crate::{
    app::{self, Message},
    components::icons::{icon, icon_raw, Icons},
};
use iced::Element;

use super::{Module, OnModulePress};

#[derive(Default, Debug, Clone)]
pub struct Custom;

impl Module for Custom {
    type ViewData<'a> = (
        &'a Option<String>,
        &'a Option<String>
    );
    type SubscriptionData<'a> = ();

    fn view(
        &self,
        config: Self::ViewData<'_>,
    ) -> Option<(Element<app::Message>, Option<OnModulePress>)> {
        if config.0.is_some() {
            Some((
                config.1.as_ref().map_or_else(
                    || icon(Icons::AppLauncher).into(),
                    |text| icon_raw(text.clone()).into()
                ),
                Some(OnModulePress::Action(Message::OpenLauncher)),
            ))
        } else {
            None
        }
    }
}
