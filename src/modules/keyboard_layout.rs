use crate::{
    config::KeyboardLayoutModuleConfig,
    services::{
        ReadOnlyService, Service, ServiceEvent,
        compositor::{CompositorCommand, CompositorService},
    },
    theme::AshellTheme,
};
use iced::{Element, Subscription, Task, widget::text};

#[derive(Debug, Clone)]
pub enum Message {
    ServiceEvent(ServiceEvent<CompositorService>),
    ChangeLayout,
}

pub struct KeyboardLayout {
    config: KeyboardLayoutModuleConfig,
    service: Option<CompositorService>,
}

impl KeyboardLayout {
    pub fn new(config: KeyboardLayoutModuleConfig) -> Self {
        Self {
            config,
            service: None,
        }
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::ServiceEvent(event) => {
                match event {
                    ServiceEvent::Init(s) => self.service = Some(s),
                    ServiceEvent::Update(e) => {
                        if let Some(service) = &mut self.service {
                            service.update(e);
                        }
                    }
                    _ => {}
                }
                Task::none()
            }
            Message::ChangeLayout => {
                if let Some(service) = &mut self.service {
                    return service
                        .command(CompositorCommand::NextLayout)
                        .map(Message::ServiceEvent);
                }
                Task::none()
            }
        }
    }

    pub fn view(&self, _: &AshellTheme) -> Option<Element<'_, Message>> {
        let service = self.service.as_ref()?;
        let active_layout = &service.keyboard_layout;

        // Fallback to displaying the layout ID/Name if no label config exists
        let label = match self.config.labels.get(active_layout) {
            Some(value) => value.to_string(),
            None => active_layout.clone(),
        };

        // Returns plain text matching original implementation style.
        // (Assuming parent container or mouse area handles interactions if any)
        Some(text(label).into())
    }

    pub fn subscription(&self) -> Subscription<Message> {
        CompositorService::subscribe().map(Message::ServiceEvent)
    }
}
