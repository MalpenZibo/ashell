use crate::style::header_pills;
use hyprland::event_listener::AsyncEventListener;
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

#[derive(Debug, Clone)]
pub enum Message {
    SubmapChanged(String),
}

impl KeyboardSubmap {
    pub fn update(&mut self, message: Message) {
        match message {
            Message::SubmapChanged(submap) => {
                self.submap = submap;
            }
        }
    }

    pub fn view(&self) -> Option<Element<Message>> {
        if self.submap.is_empty() {
            None
        } else {
            Some(
                container(text(&self.submap))
                    .padding([2, 8])
                    .style(header_pills)
                    .into(),
            )
        }
    }

    pub fn subscription(&self) -> Subscription<Message> {
        let id = TypeId::of::<Self>();

        Subscription::run_with_id(
            id,
            channel(10, |output| async move {
                let output = Arc::new(RwLock::new(output));
                loop {
                    let mut event_listener = AsyncEventListener::new();

                    event_listener.add_sub_map_changed_handler({
                        let output = output.clone();
                        move |new_submap| {
                            debug!("submap changed: {:?}", new_submap);
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
                        error!("restarting submap listener due to error: {:?}", e);
                    }
                }
            }),
        )
    }
}
