use crate::{
    components::{
        format_indicator,
        icons::{StaticIcon, icon_mono},
        slider_control,
    },
    config::SettingsFormat,
    services::{
        ReadOnlyService, Service, ServiceEvent,
        brightness::{BrightnessCommand, BrightnessService},
    },
    theme::AshellTheme,
    utils::IndicatorState,
    utils::remote_value,
};
use iced::{
    Element, Subscription, Task,
    mouse::ScrollDelta,
    widget::{Text, text},
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

    pub fn slider<'a>(&'a self, theme: &'a AshellTheme) -> Option<Element<'a, Message>> {
        self.service.as_ref().map(|service| {
            slider_control(
                theme,
                StaticIcon::Brightness,
                0..=service.max,
                service.current.value(),
                Message::Changed,
                Self::on_scroll(service.current.value(), service.max),
            )
            .into()
        })
    }

    pub fn brightness_indicator<'a>(
        &'a self,
        theme: &'a AshellTheme,
    ) -> Option<Element<'a, Message>> {
        self.service.as_ref().map(|service| {
            let scroll_handler = Self::on_scroll(service.current.value(), service.max);

            format_indicator(
                theme,
                self.config,
                icon_mono(StaticIcon::Brightness).into(),
                Self::percent_text(service).into(),
                IndicatorState::Normal,
            )
            .on_scroll(scroll_handler)
            .into()
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
