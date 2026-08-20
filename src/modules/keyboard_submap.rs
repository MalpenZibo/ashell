use crate::{
    config::Orientation,
    services::{ReadOnlyService, ServiceEvent, compositor::CompositorService},
    theme::use_theme,
};
use iced::{Element, Subscription, widget::text};
use unicode_segmentation::UnicodeSegmentation;

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

    pub fn view(&self) -> Option<Element<'_, Message>> {
        let submap = self.service.as_ref()?.submap.as_ref()?;

        if !submap.is_empty() {
            let orientation = use_theme(|t| t.orientation);
            let display = if orientation == Orientation::Vertical {
                truncate_graphemes(submap, 3)
            } else {
                submap.clone()
            };
            Some(text(display).into())
        } else {
            None
        }
    }

    pub fn subscription(&self) -> Subscription<Message> {
        CompositorService::subscribe().map(Message::ServiceEvent)
    }
}

fn truncate_graphemes(s: &str, max: usize) -> String {
    s.graphemes(true).take(max).collect()
}
