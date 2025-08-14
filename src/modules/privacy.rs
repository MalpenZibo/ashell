use crate::{
    components::icons::{Icons, icon},
    services::{ReadOnlyService, ServiceEvent, privacy::PrivacyService},
    theme::AshellTheme,
};
use iced::{
    Alignment, Element, Subscription,
    widget::{Row, container},
};

#[derive(Debug, Clone)]
pub enum Message {
    Event(ServiceEvent<PrivacyService>),
}

#[derive(Debug, Default, Clone)]
pub struct Privacy {
    pub service: Option<PrivacyService>,
}

impl Privacy {
    pub fn update(&mut self, message: Message) {
        match message {
            Message::Event(event) => match event {
                ServiceEvent::Init(service) => {
                    self.service = Some(service);
                }
                ServiceEvent::Update(data) => {
                    if let Some(privacy) = self.service.as_mut() {
                        privacy.update(data);
                    }
                }
                ServiceEvent::Error(_) => {}
            },
        }
    }

    pub fn view(&self, theme: &AshellTheme) -> Option<Element<Message>> {
        if let Some(service) = self.service.as_ref()
            && !service.no_access()
        {
            Some(
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
                        .spacing(theme.space.xs),
                )
                .style(|theme| container::Style {
                    text_color: Some(theme.extended_palette().danger.weak.color),
                    ..Default::default()
                })
                .into(),
            )
        } else {
            None
        }
    }

    pub fn subscription(&self) -> Subscription<Message> {
        PrivacyService::subscribe().map(Message::Event)
    }
}
