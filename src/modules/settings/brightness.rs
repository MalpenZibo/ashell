use crate::{
    components::icons::{StaticIcon, icon_mono},
    config::SettingsFormat,
    services::{
        ReadOnlyService, Service, ServiceEvent,
        brightness::{BrightnessCommand, BrightnessService},
    },
    theme::AshellTheme,
    utils::remote_value,
};
use iced_layershell::{
    Alignment, Element, Subscription, Task,
    mouse::ScrollDelta,
    widget::{MouseArea, Text, container, row, slider, text},
};

#[derive(Debug, Clone)]
pub enum Message {
    Event(ServiceEvent<BrightnessService>),
    Changed(remote_value::Message<u32>),
    MenuOpened,
    ConfigReloaded(SettingsFormat),
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

    fn on_scroll(current: u32, max: u32) -> impl Fn(ScrollDelta) -> Message {
        move |delta| {
            let y = match delta {
                ScrollDelta::Lines { y, .. } => y,
                ScrollDelta::Pixels { y, .. } => y,
            };
            let step = (5 * max / 100).max(1);
            let new = if y > 0.0 {
                (current + step).min(max)
            } else {
                current.saturating_sub(step)
            };
            Message::Changed(remote_value::Message::RequestAndTimeout(new))
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
            Message::Changed(message) => {
                if let Some(service) = self.service.as_mut() {
                    if let Some(value) = message.value() {
                        let _ = service.command(BrightnessCommand(value));
                    }
                    return Action::Command(service.current.update(message).map(Message::Changed));
                }
                Action::None
            }
            Message::MenuOpened => {
                if let Some(service) = self.service.as_mut() {
                    service.sync_brightness();
                }
                Action::None
            }
            Message::ConfigReloaded(format) => {
                self.config = format;
                Action::None
            }
        }
    }

    pub fn slider<'a>(&'a self, theme: &AshellTheme) -> Option<Element<'a, Message>> {
        self.service.as_ref().map(|service| {
            row!(
                container(icon_mono(StaticIcon::Brightness))
                    .center_x(32.)
                    .center_y(32.)
                    .clip(true),
                MouseArea::new(
                    Element::<'a, remote_value::Message<u32>>::from(
                        slider(
                            0..=service.max,
                            service.current.value(),
                            remote_value::Message::Request,
                        )
                        .on_release(remote_value::Message::Timeout),
                    )
                    .map(Message::Changed)
                )
                .on_scroll(Self::on_scroll(service.current.value(), service.max))
            )
            .align_y(Alignment::Center)
            .spacing(theme.space.xs)
            .into()
        })
    }

    pub fn brightness_indicator<'a>(
        &'a self,
        theme: &'a AshellTheme,
    ) -> Option<Element<'a, Message>> {
        self.service.as_ref().map(|service| {
            let scroll_handler = Self::on_scroll(service.current.value(), service.max);

            match self.config {
                SettingsFormat::Icon => {
                    let icon = icon_mono(StaticIcon::Brightness);
                    MouseArea::new(icon).on_scroll(scroll_handler).into()
                }
                SettingsFormat::Percentage | SettingsFormat::Time => {
                    MouseArea::new(Self::percent_text(service))
                        .on_scroll(scroll_handler)
                        .into()
                }
                SettingsFormat::IconAndPercentage | SettingsFormat::IconAndTime => {
                    let icon = icon_mono(StaticIcon::Brightness);
                    MouseArea::new(
                        row!(icon, Self::percent_text(service))
                            .spacing(theme.space.xxs)
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

    pub fn percent_text<'a>(service: &BrightnessService) -> Text<'a> {
        let percent = (service.current.value() * 100)
            .checked_div(service.max)
            .unwrap_or(0); // Always show 0%, if max_brightness happens to be 0
        text(format!("{percent}%"))
    }
}
