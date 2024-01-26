use hyprland::{data::Client, event_listener::EventListener, shared::HyprDataActiveOptional};
use iced::{
    widget::{container, text}, Element 
};
use std::cell::RefCell;

use crate::style::header_pills;

pub struct Title {
    value: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Message {
    TitleChanged(Option<String>),
}

impl Title {
    pub fn new() -> Self {
        let init = Client::get_active()
            .ok()
            .and_then(|w| w.map(|w| w.initial_title));

        Self { value: init }
    }

    pub fn update(&mut self, message: Message) {
        match message {
            Message::TitleChanged(value) => {
                self.value = value;
            }
        }
    }

    pub fn view(&self) -> Option<Element<Message>> {
        self.value.as_ref().map(|value| {
            container(text(value).size(12))
                .padding([6, 8])
                .style(header_pills)
                .into()
        })
    }

    pub fn subscription(&self) -> iced::Subscription<Message> {
        iced::subscription::channel("title-listener", 10, |output| async move {
            let output = RefCell::new(output);
            let mut event_listener = EventListener::new();

            event_listener.add_active_window_change_handler({
                let output = output.clone();
                move |e| {
                    let mut output = output.borrow_mut();
                    output
                        .try_send(Message::TitleChanged(e.map(|e| e.window_title)))
                        .unwrap();
                }
            });

            event_listener.add_window_close_handler({
                let output = output.clone();
                move |_| {
                    let mut output = output.borrow_mut();
                    output.try_send(Message::TitleChanged(None)).unwrap();
                }
            });

            event_listener
                .start_listener_async()
                .await
                .expect("failed to start active window listener");

            panic!("Exiting hyprland event listener");
        })
    }
}
