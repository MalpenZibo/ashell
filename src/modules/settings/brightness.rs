use crate::{
    components::{format_indicator, icons::StaticIcon, slider_control},
    config::SettingsFormat,
    modules::settings::EventSource,
    services::{
        ReadOnlyService, Service, ServiceEvent,
        brightness::{BrightnessCommand, BrightnessService},
    },
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
    Changed(remote_value::Message<u32>, EventSource),
    MenuOpened,
    ConfigReloaded(BrightnessSettingsConfig),
}

pub enum Action {
    None,
    Command(Task<Message>),
}

#[derive(Debug, Clone)]
pub struct BrightnessSettingsConfig {
    pub indicator_format: SettingsFormat,
    pub step: u32,
}

impl BrightnessSettingsConfig {
    pub fn new(indicator_format: SettingsFormat, step: u32) -> Self {
        Self {
            indicator_format,
            step,
        }
    }
}

pub struct BrightnessSettings {
    config: BrightnessSettingsConfig,
    service: Option<BrightnessService>,
}

impl BrightnessSettings {
    pub fn new(config: BrightnessSettingsConfig) -> Self {
        Self {
            config,
            service: None,
        }
    }

    pub fn current_brightness(&self) -> Option<(u32, u32)> {
        self.service.as_ref().map(|s| (s.current.value(), s.max))
    }

    pub fn brightness_adjust(&mut self, up: bool) -> Action {
        let Some((cur, max)) = self.current_brightness() else {
            return Action::None;
        };
        let step = (self.config.step * max / 100).max(1);
        let new_val = if up {
            (cur + step).min(max)
        } else {
            cur.saturating_sub(step)
        };
        self.update(Message::Changed(
            remote_value::Message::RequestAndTimeout(new_val),
            EventSource::Irelevant,
        ))
    }

    fn on_scroll(
        current: u32,
        max: u32,
        event_source: EventSource,
    ) -> impl Fn(ScrollDelta) -> Message {
        move |delta| {
            let y = match delta {
                ScrollDelta::Lines { y, .. } => y,
                ScrollDelta::Pixels { y, .. } => y,
            };
            let step = (max / 100).max(1);
            let new = if y > 0.0 {
                (current + step).min(max)
            } else {
                current.saturating_sub(step)
            };
            Message::Changed(remote_value::Message::RequestAndTimeout(new), event_source)
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
            Message::Changed(message, event_source) => {
                if let Some(service) = self.service.as_mut() {
                    if let Some(value) = message.value() {
                        let _ = service.command(BrightnessCommand(value));
                    }
                    return Action::Command(
                        service
                            .current
                            .update(message)
                            .map(move |msg| Message::Changed(msg, event_source)),
                    );
                }
                Action::None
            }
            Message::MenuOpened => {
                if let Some(service) = self.service.as_mut() {
                    service.sync_brightness();
                }
                Action::None
            }
            Message::ConfigReloaded(config) => {
                self.config = config;
                Action::None
            }
        }
    }

    pub fn slider<'a>(&'a self) -> Option<Element<'a, Message>> {
        self.service.as_ref().map(|service| {
            slider_control(
                StaticIcon::Brightness,
                0..=service.max,
                service.current.value(),
                Message::Changed,
                Self::on_scroll(service.current.value(), service.max, EventSource::Irelevant),
            )
            .into()
        })
    }

    pub fn brightness_indicator<'a>(&'a self) -> Option<Element<'a, Message>> {
        self.service.as_ref().map(|service| {
            let scroll_handler =
                Self::on_scroll(service.current.value(), service.max, EventSource::Irelevant);

            format_indicator(
                self.config.indicator_format,
                StaticIcon::Brightness,
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
