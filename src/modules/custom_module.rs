use crate::{
    app::{self, Message},
    components::icons::{Icons, icon, icon_raw},
};
use iced::Element;

use super::{Module, OnModulePress};

#[derive(Default, Debug, Clone)]
pub struct Custom;

impl Module for Custom {
    type ViewData<'a> = (&'a String, &'a Option<String>);
    type SubscriptionData<'a> = ();

    fn view(
        &self,
        config: Self::ViewData<'_>,
    ) -> Option<(Element<app::Message>, Option<OnModulePress>)> {
        Some((
            config.1.as_ref().map_or_else(
                || icon(Icons::None).into(),
                |text| icon_raw(text.clone()).into(),
            ),
            Some(OnModulePress::Action(Message::LaunchCommand(config.0.clone()))),
        ))
    }
}
