use hyprland::event_listener::AsyncEventListener;
use iced::{Element, Subscription, stream::channel, widget::text};
use log::{debug, error};
use std::{
    any::TypeId,
    sync::{Arc, RwLock},
};

use crate::theme::AshellTheme;

#[derive(Debug, Clone)]
pub enum Message {
    SubmapChanged(String),
}

pub struct KeyboardSubmap {
    submap: String,
}

impl Default for KeyboardSubmap {
    fn default() -> Self {
        Self {
            submap: "".to_string(),
        }
    }
}

impl KeyboardSubmap {
    pub fn update(&mut self, message: Message) {
        match message {
            Message::SubmapChanged(submap) => {
                self.submap = submap;
            }
        }
    }

    pub fn view(&self, _: &AshellTheme) -> Option<Element<Message>> {
        if !self.submap.is_empty() {
            Some(text(&self.submap).into())
        } else {
            None
        }
    }

    pub fn subscription(&self) -> Subscription<Message> {
        let id = TypeId::of::<Self>();

        Subscription::run_with_id(
            id,
            channel(10, async |output| {
                let output = Arc::new(RwLock::new(output));
                loop {
                    let mut event_listener = AsyncEventListener::new();

                    event_listener.add_sub_map_changed_handler({
                        let output = output.clone();
                        move |new_submap| {
                            debug!("submap changed: {new_submap:?}");
                            let output = output.clone();
                            Box::pin(async move {
                                if let Ok(mut output) = output.write() {
                                    output
                                        .try_send(Message::SubmapChanged(new_submap))
                                        .expect("error getting submap: submap changed event");
                                }
                            })
                        }
                    });

                    let res = event_listener.start_listener_async().await;

                    if let Err(e) = res {
                        error!("restarting submap listener due to error: {e:?}");
                    }
                }
            }),
        )
    }
}
