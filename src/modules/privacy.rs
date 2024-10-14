use crate::{
    components::icons::{icon, Icons},
    services::{
        privacy::{PrivacyData, PrivacyService},
        ServiceEvent,
    },
};
use iced::{
    widget::{container, Row},
    Alignment, Element, Theme,
};

#[derive(Debug, Clone)]
pub enum PrivacyMessage {
    Event(ServiceEvent<PrivacyService>),
}

impl PrivacyData {
    pub fn view(&self) -> Option<Element<PrivacyMessage>> {
        if !self.no_access() {
            Some(
                container(
                    Row::new()
                        .push_maybe(self.screenshare_access().then(|| icon(Icons::ScreenShare)))
                        .push_maybe(self.webcam_access().then(|| icon(Icons::Webcam)))
                        .push_maybe(self.microphone_access().then(|| icon(Icons::Mic1)))
                        .align_items(Alignment::Center)
                        .spacing(8),
                )
                .padding([2, 8])
                .style(|theme: &Theme| container::Appearance {
                    background: Some(theme.palette().background.into()),
                    text_color: Some(theme.extended_palette().danger.weak.color),
                    ..Default::default()
                })
                .into(),
            )
        } else {
            None
        }
    }
}
