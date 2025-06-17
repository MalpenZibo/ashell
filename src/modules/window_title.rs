use crate::{app, utils::truncate_text};
use hyprland::{data::Client, event_listener::AsyncEventListener, shared::HyprDataActiveOptional};
use iced::{Element, Subscription, stream::channel, widget::text};
use log::{debug, error};
use std::{
    any::TypeId,
    sync::{Arc, RwLock},
};

use super::{Module, OnModulePress};

pub struct WindowTitle {
    value: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Message {
    TitleChanged(Option<String>),
}

impl Default for WindowTitle {
    fn default() -> Self {
        let init = Client::get_active().ok().and_then(|w| w.map(|w| w.title));

        Self { value: init }
    }
}

impl WindowTitle {
    pub fn update(&mut self, message: Message, truncate_title_after_length: u32) {
        match message {
            Message::TitleChanged(value) => {
                if let Some(value) = value {
                    self.value = Some(truncate_text(&value, truncate_title_after_length));
                } else {
                    self.value = None;
                }
            }
        }
    }
}

impl Module for WindowTitle {
    type ViewData<'a> = ();
    type SubscriptionData<'a> = ();

    fn view(
        &self,
        _: Self::ViewData<'_>,
    ) -> Option<(Element<app::Message>, Option<OnModulePress>)> {
        self.value.as_ref().map(|value| {
            (
                text(value)
                    .size(12)
                    .wrapping(text::Wrapping::WordOrGlyph)
                    .into(),
                None,
            )
        })
    }

    fn subscription(&self, _: Self::SubscriptionData<'_>) -> Option<Subscription<app::Message>> {
        let id = TypeId::of::<Self>();

        Some(
            Subscription::run_with_id(
                id,
                channel(10, async |output| {
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
                                        let current = Client::get_active()
                                            .ok()
                                            .and_then(|w| w.map(|w| w.title));

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
            .map(app::Message::WindowTitle),
        )
    }
}
