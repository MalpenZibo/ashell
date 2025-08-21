use crate::{
    components::icons::{Icons, icon},
    services::{
        ReadOnlyService, Service, ServiceEvent,
        brightness::{BrightnessCommand, BrightnessService},
    },
    theme::AshellTheme,
};
use iced::{
    Alignment, Element, Length, Subscription, Task,
    widget::{container, row, slider},
};

#[derive(Debug, Clone)]
pub enum Message {
    Event(ServiceEvent<BrightnessService>),
    Change(u32),
    MenuOpened,
}

pub enum Action {
    None,
    Command(Task<Message>),
}

pub struct BrightnessSettings {
    service: Option<BrightnessService>,
}

impl BrightnessSettings {
    pub fn new() -> Self {
        Self { service: None }
    }

    pub fn update(&mut self, message: Message) -> Action {
        match message {
            Message::Event(event) => match event {
                ServiceEvent::Init(service) => {
                    self.service = Some(service);
                    Action::None
                }
                ServiceEvent::Update(data) => {
                    if let Some(service) = self.service.as_mut() {
                        service.update(data);
                    }
                    Action::None
                }
                _ => Action::None,
            },
            Message::Change(value) => match self.service.as_mut() {
                Some(service) => Action::Command(
                    service
                        .command(BrightnessCommand::Set(value))
                        .map(Message::Event),
                ),
                _ => Action::None,
            },
            Message::MenuOpened => {
                if let Some(service) = self.service.as_mut() {
                    Action::Command(
                        service
                            .command(BrightnessCommand::Refresh)
                            .map(Message::Event),
                    )
                } else {
                    Action::None
                }
            }
        }
    }

    pub fn slider(&'_ self, theme: &AshellTheme) -> Option<Element<'_, Message>> {
        self.service.as_ref().map(|service| {
            row!(
                container(icon(Icons::Brightness)).padding([theme.space.xs, theme.space.sm - 1]),
                slider(0..=100, service.current * 100 / service.max, |v| {
                    Message::Change(v * service.max / 100)
                })
                .step(1_u32)
                .width(Length::Fill),
            )
            .align_y(Alignment::Center)
            .spacing(theme.space.xs)
            .into()
        })
    }

    pub fn subscription(&self) -> Subscription<Message> {
        BrightnessService::subscribe().map(Message::Event)
    }
}
