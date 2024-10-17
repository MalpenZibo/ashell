use crate::style::header_pills;
use hyprland::{data::Client, event_listener::AsyncEventListener, shared::HyprDataActiveOptional};
use iced::{
    stream::channel,
    widget::{container, text},
    Element, Subscription,
};
use log::{debug, error};
use std::{
    any::TypeId,
    sync::{Arc, RwLock},
};

pub struct Title {
    value: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Message {
    TitleChanged(Option<String>),
}

impl Default for Title {
    fn default() -> Self {
        let init = Client::get_active().ok().and_then(|w| w.map(|w| w.title));

        Self { value: init }
    }
}

impl Title {
    pub fn update(&mut self, message: Message, truncate_title_after_length: u32) {
        match message {
            Message::TitleChanged(value) => {
                if let Some(value) = value {
                    let length = value.len();

                    self.value = Some(if length > truncate_title_after_length as usize {
                        let split = truncate_title_after_length as usize / 2;
                        let first_part = value.chars().take(split).collect::<String>();
                        let last_part = value.chars().skip(length - split).collect::<String>();
                        format!("{}...{}", first_part, last_part)
                    } else {
                        value
                    });
                } else {
                    self.value = None;
                }
            }
        }
    }

    pub fn view(&self) -> Option<Element<Message>> {
        self.value.as_ref().map(|value| {
            container(text(value).size(12))
                .padding([2, 7])
                .style(header_pills)
                .into()
        })
    }

    pub fn subscription(&self) -> Subscription<Message> {
        let id = TypeId::of::<Self>();
        Subscription::run_with_id(
            id,
            channel(10, |output| async move {
                let output = Arc::new(RwLock::new(output));
                loop {
                    let mut event_listener = AsyncEventListener::new();

                    event_listener.add_workspace_changed_handler({
                        let output = output.clone();
                        move |_| {
                            let output = output.clone();
                            Box::pin(async move {
                                debug!("Window closed");
                                if let Ok(mut output) = output.write() {
                                    let current =
                                        Client::get_active().ok().and_then(|w| w.map(|w| w.title));

                                    debug!("Sending title changed message");
                                    output.try_send(Message::TitleChanged(current)).unwrap();
                                }
                            })
                        }
                    });

                    event_listener.add_active_window_changed_handler({
                        let output = output.clone();
                        move |e| {
                            let output = output.clone();
                            Box::pin(async move {
                                debug!("Active window changed: {:?}", e);
                                if let Ok(mut output) = output.write() {
                                    debug!("Sending title changed message");
                                    output
                                        .try_send(Message::TitleChanged(e.map(|e| e.title)))
                                        .unwrap();
                                }
                            })
                        }
                    });

                    event_listener.add_window_closed_handler({
                        let output = output.clone();
                        move |_| {
                            let output = output.clone();
                            Box::pin(async move {
                                debug!("Window closed");
                                if let Ok(mut output) = output.write() {
                                    debug!("Sending title changed message");
                                    output.try_send(Message::TitleChanged(None)).unwrap();
                                }
                            })
                        }
                    });

                    debug!("Starting title listener");

                    let res = event_listener.start_listener_async().await;

                    if let Err(e) = res {
                        error!("restarting active window listener due to error: {:?}", e);
                    }
                }
            }),
        )
    }
}
