use crate::{
    config::{WindowTitleConfig, WindowTitleMode},
    services::{ReadOnlyService, ServiceEvent, compositor::CompositorService},
    theme::AshellTheme,
    utils::truncate_text,
};
use iced::{
    Element, Subscription,
    widget::{container, text},
};

#[derive(Debug, Clone)]
pub enum Message {
    ServiceEvent(ServiceEvent<CompositorService>),
    ConfigReloaded(WindowTitleConfig),
}

pub struct WindowTitle {
    config: WindowTitleConfig,
    service: Option<CompositorService>,
    value: Option<String>,
}

impl WindowTitle {
    pub fn new(config: WindowTitleConfig) -> Self {
        Self {
            config,
            service: None,
            value: None,
        }
    }

    pub fn update(&mut self, message: Message) {
        match message {
            Message::ServiceEvent(event) => match event {
                ServiceEvent::Init(service) => {
                    self.service = Some(service);
                    self.recalculate_value();
                }
                ServiceEvent::Update(event) => {
                    if let Some(service) = &mut self.service {
                        service.update(event);
                        self.recalculate_value();
                    }
                }
                _ => {}
            },
            Message::ConfigReloaded(cfg) => {
                self.config = cfg;
                self.recalculate_value();
            }
        }
    }

    fn recalculate_value(&mut self) {
        if let Some(service) = &self.service {
            self.value = service.active_window.as_ref().map(|w| {
                let raw_title = match self.config.mode {
                    WindowTitleMode::Title => w.title(),
                    WindowTitleMode::Class => w.class(),
                    WindowTitleMode::InitialTitle => match w.initial_title() {
                        Ok(v) => v,
                        Err(e) => {
                            log::warn!("{}", e);
                            ""
                        }
                    },
                    WindowTitleMode::InitialClass => match w.initial_class() {
                        Ok(v) => v,
                        Err(e) => {
                            log::warn!("{}", e);
                            ""
                        }
                    },
                };

                if self.config.truncate_title_after_length > 0 {
                    truncate_text(raw_title, self.config.truncate_title_after_length)
                } else {
                    raw_title.to_string()
                }
            });
        }
    }

    pub fn get_value(&self) -> Option<String> {
        self.value.clone()
    }

    pub fn view(&'_ self, theme: &AshellTheme, title: String) -> Element<'_, Message> {
        container(
            text(title)
                .size(theme.font_size.sm)
                .wrapping(text::Wrapping::None),
        )
        .clip(true)
        .into()
    }

    pub fn subscription(&self) -> Subscription<Message> {
        CompositorService::subscribe().map(Message::ServiceEvent)
    }
}
