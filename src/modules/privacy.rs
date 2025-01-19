use super::{Module, OnModulePress};
use crate::{
    app,
    components::icons::{icon, Icons},
    services::{
        privacy::{PrivacyData, PrivacyService},
        ReadOnlyService, ServiceEvent,
    },
};
use iced::{widget::Row, Alignment, Element, Subscription};

#[derive(Debug, Clone)]
pub enum PrivacyMessage {
    Event(ServiceEvent<PrivacyService>),
}

impl Module for PrivacyData {
    type ViewData<'a> = ();
    type SubscriptionData<'a> = ();

    fn view(
        &self,
        _: Self::ViewData<'_>,
    ) -> Option<(Element<app::Message>, Option<OnModulePress>)> {
        if !self.no_access() {
            Some((
                Row::new()
                    .push_maybe(self.screenshare_access().then(|| icon(Icons::ScreenShare)))
                    .push_maybe(self.webcam_access().then(|| icon(Icons::Webcam)))
                    .push_maybe(self.microphone_access().then(|| icon(Icons::Mic1)))
                    .align_y(Alignment::Center)
                    .spacing(8)
                    .into(),
                None,
            ))
        } else {
            None
        }
    }

    fn subscription(&self, _: Self::SubscriptionData<'_>) -> Option<Subscription<app::Message>> {
        Some(PrivacyService::subscribe().map(|e| app::Message::Privacy(PrivacyMessage::Event(e))))
    }
}
