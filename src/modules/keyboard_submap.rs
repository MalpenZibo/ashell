use crate::{
    services::{ReadOnlyService, Service, ServiceEvent, compositor::CompositorService},
    theme::AshellTheme,
};
use iced::{Element, Subscription, widget::text};

#[derive(Debug, Clone)]
pub enum Message {
    ServiceEvent(ServiceEvent<CompositorService>),
}

#[derive(Debug, Clone)]
pub struct KeyboardSubmap {
    service: Option<CompositorService>,
}

impl KeyboardSubmap {
    pub fn default() -> Self {
        Self { service: None }
    }

    pub fn update(&mut self, message: Message) {
        match message {
            Message::ServiceEvent(event) => match event {
                ServiceEvent::Init(s) => self.service = Some(s),
                ServiceEvent::Update(e) => {
                    if let Some(service) = &mut self.service {
                        service.update(e);
                    }
                }
                _ => {}
            },
        }
    }

    pub fn view(&self, _: &AshellTheme) -> Option<Element<Message>> {
        let submap = self.service.as_ref()?.submap.as_ref()?;

        if !submap.is_empty() {
            Some(text(submap).into())
        } else {
            None
        }
    }

    pub fn subscription(&self) -> Subscription<Message> {
        CompositorService::subscribe().map(Message::ServiceEvent)
    }
}
