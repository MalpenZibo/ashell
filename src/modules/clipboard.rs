use crate::{
    app::{self},
    components::icons::{Icons, icon},
};
use iced::Element;

use super::{Module, OnModulePress};

#[derive(Default, Debug, Clone)]
pub struct Clipboard;

impl Module for Clipboard {
    type ViewData<'a> = &'a Option<String>;
    type SubscriptionData<'a> = ();

    fn view(
        &self,
        config: Self::ViewData<'_>,
    ) -> Option<(Element<app::Message>, Option<OnModulePress>)> {
        if config.is_some() {
            Some((
                icon(Icons::Clipboard).into(),
                Some(OnModulePress::Action(Box::new(app::Message::OpenClipboard))),
            ))
        } else {
            None
        }
    }
}
