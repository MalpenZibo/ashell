use crate::{
    components::icons::{StaticIcon, icon},
    config::Orientation,
    services::{ReadOnlyService, ServiceEvent, privacy::PrivacyService},
    theme::use_theme,
};
use iced::{
    Alignment, Element, Subscription,
    widget::{Column, Row, container},
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

    pub fn view(&'_ self) -> Option<Element<'_, Message>> {
        let (space, orientation) = use_theme(|theme| (theme.space, theme.orientation));
        if let Some(service) = self.service.as_ref()
            && !service.no_access()
        {
            let content: Element<'_, Message> = match orientation {
                Orientation::Horizontal => Row::with_capacity(3)
                    .push(
                        service
                            .screenshare_access()
                            .then(|| icon(StaticIcon::ScreenShare)),
                    )
                    .push(service.webcam_access().then(|| icon(StaticIcon::Webcam)))
                    .push(service.microphone_access().then(|| icon(StaticIcon::Mic1)))
                    .align_y(Alignment::Center)
                    .spacing(space.xs)
                    .into(),
                Orientation::Vertical => Column::with_capacity(3)
                    .push(
                        service
                            .screenshare_access()
                            .then(|| icon(StaticIcon::ScreenShare)),
                    )
                    .push(service.webcam_access().then(|| icon(StaticIcon::Webcam)))
                    .push(service.microphone_access().then(|| icon(StaticIcon::Mic1)))
                    .align_x(Alignment::Center)
                    .spacing(space.xs)
                    .into(),
            };

            Some(
                container(content)
                    .style(|theme| container::Style {
                        text_color: Some(theme.palette().warning),
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
