use hyprland::event_listener::AsyncEventListener;
use iced::{Element, Subscription, stream::channel, widget::text};
use log::{debug, error};
use std::{
    any::TypeId,
    sync::{Arc, RwLock},
};

use crate::app;

use super::{Module, OnModulePress};

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
}

impl Module for KeyboardSubmap {
    type ViewData<'a> = ();
    type SubscriptionData<'a> = ();

    fn view(
        &self,
        _: Self::ViewData<'_>,
    ) -> Option<(Element<app::Message>, Option<OnModulePress>)> {
        if self.submap.is_empty() {
            None
        } else {
            Some((text(&self.submap).into(), None))
        }
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
            .map(app::Message::KeyboardSubmap),
        )
    }
}
