use crate::{
    app::{self},
    components::icons::{icon, Icons},
};
use iced::Element;

use super::{Module, OnModulePress};

#[derive(Default, Debug, Clone)]
pub struct Clipboard;

impl Module for Clipboard {
    type ViewData<'a> = ();
    type SubscriptionData<'a> = ();

    fn view(
        &self,
        _: Self::ViewData<'_>,
    ) -> Option<(Element<app::Message>, Option<OnModulePress>)> {
        Some((
            icon(Icons::Clipboard).into(),
            Some(OnModulePress::Action(app::Message::OpenClipboard)),
        ))
    }
}
