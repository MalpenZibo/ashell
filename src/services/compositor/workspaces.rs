use crate::services::{
    compositor::{CompositorService, CompositorCommand, CompositorEvent},
    Service, ServiceEvent,
};
use iced::{Element, Subscription, widget::{button, row}};

#[derive(Debug, Clone)]
pub enum Message {
    CompositorEvent(ServiceEvent<CompositorService>),
    ClickWorkspace(i32),
}

pub struct WorkspacesModule {
    service: Option<CompositorService>,
}

impl WorkspacesModule {
    pub fn new() -> Self {
        Self { service: None }
    }

    pub fn update(&mut self, message: Message) -> iced::Task<Message> {
        match message {
            // 1. Handle Service Events
            Message::CompositorEvent(event) => {
                match event {
                    ServiceEvent::Init(s) => self.service = Some(s),
                    ServiceEvent::Update(e) => {
                        if let Some(s) = &mut self.service {
                            s.update(e);
                        }
                    }
                    _ => {}
                }
                iced::Task::none()
            }
            // 2. Handle User Interaction via Service Command
            Message::ClickWorkspace(id) => {
                if let Some(s) = &mut self.service {
                     // We map the service command result back to our message type
                     return s.command(CompositorCommand::FocusWorkspace(id))
                        .map(Message::CompositorEvent);
                }
                iced::Task::none()
            }
        }
    }

    // 3. Subscribe to the Service
    pub fn subscription(&self) -> Subscription<Message> {
        CompositorService::subscribe().map(Message::CompositorEvent)
    }

    pub fn view(&self) -> Element<Message> {
        let Some(service) = &self.service else {
            return text("Loading...").into();
        };

        row(
            service.workspaces.iter().map(|ws| {
                button(ws.name.as_str())
                    .on_press(Message::ClickWorkspace(ws.id))
                    .into()
            })
        ).into()
    }
}
