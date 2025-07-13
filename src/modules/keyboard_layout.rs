use hyprland::{
    ctl::switch_xkb_layout::SwitchXKBLayoutCmdTypes, event_listener::AsyncEventListener,
    shared::HyprData,
};
use iced::{Element, Subscription, stream::channel, widget::text};
use log::{debug, error};
use std::{
    any::TypeId,
    sync::{Arc, RwLock},
};

use crate::{app, config::KeyboardLayoutModuleConfig};

use super::{Module, OnModulePress};

fn get_multiple_layout_flag() -> bool {
    match hyprland::keyword::Keyword::get("input:kb_layout") {
        Ok(layouts) => layouts.value.to_string().split(",").count() > 1,
        Err(_) => false,
    }
}

fn get_active_layout() -> String {
    hyprland::data::Devices::get()
        .ok()
        .and_then(|devices| {
            devices
                .keyboards
                .iter()
                .find(|k| k.main)
                .map(|keyboard| keyboard.active_keymap.to_string())
        })
        .unwrap_or_else(|| "unknown".to_string())
}

#[derive(Debug, Clone)]
pub struct KeyboardLayout {
    multiple_layout: bool,
    active: String,
}

impl Default for KeyboardLayout {
    fn default() -> Self {
        Self {
            multiple_layout: get_multiple_layout_flag(),
            active: get_active_layout(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    LayoutConfigChanged(bool),
    ActiveLayoutChanged(String),
    ChangeLayout,
}

impl KeyboardLayout {
    pub fn update(&mut self, message: Message) {
        match message {
            Message::ActiveLayoutChanged(layout) => {
                self.active = layout;
            }
            Message::LayoutConfigChanged(layout_flag) => self.multiple_layout = layout_flag,
            Message::ChangeLayout => {
                let res =
                    hyprland::ctl::switch_xkb_layout::call("all", SwitchXKBLayoutCmdTypes::Next);

                if let Err(e) = res {
                    error!("failed to keymap change: {e:?}");
                }
            }
        }
    }
}

impl Module for KeyboardLayout {
    type ViewData<'a> = &'a KeyboardLayoutModuleConfig;
    type SubscriptionData<'a> = ();

    fn view(
        &self,
        config: Self::ViewData<'_>,
    ) -> Option<(Element<app::Message>, Option<OnModulePress>)> {
        if !self.multiple_layout {
            None
        } else {
            let active = match config.labels.get(&self.active) {
                Some(value) => value.to_string(),
                None => self.active.clone(),
            };
            Some((
                text(active).into(),
                Some(OnModulePress::Action(Box::new(
                    app::Message::KeyboardLayout(Message::ChangeLayout),
                ))),
            ))
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

                        event_listener.add_layout_changed_handler({
                            let output = output.clone();
                            move |e| {
                                debug!("keymap changed: {e:?}");
                                let output = output.clone();
                                Box::pin(async move {
                                    if let Ok(mut output) = output.write() {
                                        output
                                            .try_send(Message::ActiveLayoutChanged(
                                                get_active_layout(),
                                            ))
                                            .expect("error getting keymap: layout changed event");
                                    }
                                })
                            }
                        });

                        event_listener.add_config_reloaded_handler({
                            let output = output.clone();
                            move || {
                                let output = output.clone();
                                Box::pin(async move {
                                    if let Ok(mut output) = output.write() {
                                        output
                                        .try_send(Message::LayoutConfigChanged(
                                            get_multiple_layout_flag(),
                                        ))
                                        .expect(
                                            "error sending message: layout config changed event",
                                        );
                                    }
                                })
                            }
                        });

                        let res = event_listener.start_listener_async().await;

                        if let Err(e) = res {
                            error!("restarting keymap listener due to error: {e:?}");
                        }
                    }
                }),
            )
            .map(app::Message::KeyboardLayout),
        )
    }
}
