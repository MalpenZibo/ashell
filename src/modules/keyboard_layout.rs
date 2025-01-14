use hyprland::{
    ctl::switch_xkb_layout::SwitchXKBLayoutCmdTypes, event_listener::AsyncEventListener,
    shared::HyprData,
};
use iced::{stream::channel, widget::text, Element, Subscription};
use log::{debug, error};
use std::{
    any::TypeId,
    sync::{Arc, RwLock},
};

use crate::app;

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
                    error!("failed to keymap change: {:?}", e);
                }
            }
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

                    event_listener.add_layout_changed_handler({
                        let output = output.clone();
                        move |e| {
                            debug!("keymap changed: {:?}", e);
                            let output = output.clone();
                            Box::pin(async move {
                                if let Ok(mut output) = output.write() {
                                    output
                                        .try_send(Message::ActiveLayoutChanged(get_active_layout()))
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
                        error!("restarting keymap listener due to error: {:?}", e);
                    }
                }
            }),
        )
    }
}

impl Module for KeyboardLayout {
    type Data<'a> = ();

    fn view(&self, _: Self::Data<'_>) -> Option<(Element<app::Message>, Option<OnModulePress>)> {
        if !self.multiple_layout {
            None
        } else {
            Some((
                text(&self.active).into(),
                Some(OnModulePress::Action(app::Message::KeyboardLayout(
                    Message::ChangeLayout,
                ))),
            ))
        }
    }
}
