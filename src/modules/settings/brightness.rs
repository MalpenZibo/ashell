use crate::{
    components::icons::{StaticIcon, icon_mono},
    config::SettingsFormat,
    services::{
        ReadOnlyService, Service, ServiceEvent,
        brightness::{BrightnessCommand, BrightnessService},
    },
    theme::AshellTheme,
};
use iced::{
    Alignment, Element, Length, Subscription, Task,
    widget::{MouseArea, container, row, slider, text},
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
    config: SettingsFormat,
    service: Option<BrightnessService>,
}

impl BrightnessSettings {
    pub fn new(config: SettingsFormat) -> Self {
        Self {
            config,
            service: None,
        }
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
                container(icon_mono(StaticIcon::Brightness))
                    .center_x(32.)
                    .center_y(32.)
                    .clip(true),
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

    pub fn brightness_indicator<'a>(&'a self) -> Option<Element<'a, Message>> {
        self.service.as_ref().map(|service| {
            let percentage = service.current * 100 / service.max;
            let max_value = service.max;

            let scroll_handler = move |delta| {
                let cur_percentage = percentage;
                let delta = match delta {
                    iced::mouse::ScrollDelta::Lines { y, .. } => y,
                    iced::mouse::ScrollDelta::Pixels { y, .. } => y,
                };
                let new_percentage = if delta > 0.0 {
                    (cur_percentage + 5).min(100)
                } else {
                    cur_percentage - 5
                };
                // Convert percentage back to brightness value
                let new_brightness = new_percentage * max_value / 100;
                Message::Change(new_brightness)
            };

            match self.config {
                SettingsFormat::Icon => {
                    let icon = icon_mono(StaticIcon::Brightness);
                    MouseArea::new(icon).on_scroll(scroll_handler).into()
                }
                SettingsFormat::Percentage => MouseArea::new(text(format!("{}%", percentage)))
                    .on_scroll(scroll_handler)
                    .into(),
                SettingsFormat::IconAndPercentage => {
                    let icon = icon_mono(StaticIcon::Brightness);
                    MouseArea::new(
                        row!(icon, text(format!("{}%", percentage)))
                            .spacing(4)
                            .align_y(Alignment::Center),
                    )
                    .on_scroll(scroll_handler)
                    .into()
                }
            }
        })
    }

    pub fn subscription(&self) -> Subscription<Message> {
        BrightnessService::subscribe().map(Message::Event)
    }
}
