use super::{Module, OnModulePress};
use crate::{
    app,
    components::icons::{Icons, icon},
    services::{ReadOnlyService, ServiceEvent, privacy::PrivacyService},
};
use iced::{
    Alignment, Element, Subscription, Task,
    widget::{Row, container},
};

#[derive(Debug, Clone)]
pub enum PrivacyMessage {
    Event(ServiceEvent<PrivacyService>),
}

#[derive(Debug, Default, Clone)]
pub struct Privacy {
    pub service: Option<PrivacyService>,
}

impl Privacy {
    pub fn update(&mut self, message: PrivacyMessage) -> Task<crate::app::Message> {
        match message {
            PrivacyMessage::Event(event) => match event {
                ServiceEvent::Init(service) => {
                    self.service = Some(service);
                    Task::none()
                }
                ServiceEvent::Update(data) => {
                    if let Some(privacy) = self.service.as_mut() {
                        privacy.update(data);
                    }
                    Task::none()
                }
                ServiceEvent::Error(_) => Task::none(),
            },
        }
    }
}

impl Module for Privacy {
    type ViewData<'a> = ();
    type SubscriptionData<'a> = ();

    fn view(
        &self,
        _: Self::ViewData<'_>,
    ) -> Option<(Element<app::Message>, Option<OnModulePress>)> {
        if let Some(service) = self.service.as_ref() {
            if !service.no_access() {
                Some((
                    container(
                        Row::new()
                            .push_maybe(
                                service
                                    .screenshare_access()
                                    .then(|| icon(Icons::ScreenShare)),
                            )
                            .push_maybe(service.webcam_access().then(|| icon(Icons::Webcam)))
                            .push_maybe(service.microphone_access().then(|| icon(Icons::Mic1)))
                            .align_y(Alignment::Center)
                            .spacing(8),
                    )
                    .style(|theme| container::Style {
                        text_color: Some(theme.extended_palette().danger.weak.color),
                        ..Default::default()
                    })
                    .into(),
                    None,
                ))
            } else {
                None
            }
        } else {
            None
        }
    }

    fn subscription(&self, _: Self::SubscriptionData<'_>) -> Option<Subscription<app::Message>> {
        Some(PrivacyService::subscribe().map(|e| app::Message::Privacy(PrivacyMessage::Event(e))))
    }
}
