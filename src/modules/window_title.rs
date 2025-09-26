use crate::{
    config::{WindowTitleConfig, WindowTitleMode},
    theme::AshellTheme,
    utils::truncate_text,
};
use hyprland::{data::Client, event_listener::AsyncEventListener, shared::HyprDataActiveOptional};
use iced::{Element, Subscription, stream::channel, widget::text};
use log::{debug, error};
use std::{
    any::TypeId,
    sync::{Arc, RwLock},
};

fn get_window(config: &WindowTitleConfig) -> Option<String> {
    Client::get_active().ok().and_then(|w| {
        w.map(|w| match config.mode {
            WindowTitleMode::Title => w.title,
            WindowTitleMode::Class => w.class,
        })
        .map(|v| {
            if config.truncate_title_after_length > 0 {
                truncate_text(&v, config.truncate_title_after_length)
            } else {
                v
            }
        })
    })
}

#[derive(Debug, Clone)]
pub enum Message {
    TitleChanged,
}

pub struct WindowTitle {
    config: WindowTitleConfig,
    value: Option<String>,
}

impl WindowTitle {
    pub fn new(config: WindowTitleConfig) -> Self {
        let init = get_window(&config);

        Self {
            value: init,
            config,
        }
    }

    pub fn update(&mut self, message: Message) {
        match message {
            Message::TitleChanged => {
                self.value = get_window(&self.config);
            }
        }
    }

    pub fn get_value(&self) -> Option<String> {
        self.value.clone()
    }

    pub fn view(&'_ self, theme: &AshellTheme, title: String) -> Element<'_, Message> {
        text(title.to_string())
            .size(theme.font_size.sm)
            .wrapping(text::Wrapping::WordOrGlyph)
            .into()
    }

    pub fn subscription(&self) -> Subscription<Message> {
        let id = TypeId::of::<Self>();

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
                                    debug!("Sending title changed message");
                                    output.try_send(Message::TitleChanged).unwrap();
                                }
                            })
                        }
                    });

                    event_listener.add_active_window_changed_handler({
                        let output = output.clone();
                        move |e| {
                            let output = output.clone();
                            Box::pin(async move {
                                debug!("Active window changed: {e:?}");
                                if let Ok(mut output) = output.write() {
                                    debug!("Sending title changed message");
                                    output.try_send(Message::TitleChanged).unwrap();
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
                                    output.try_send(Message::TitleChanged).unwrap();
                                }
                            })
                        }
                    });

                    debug!("Starting title listener");

                    let res = event_listener.start_listener_async().await;

                    if let Err(e) = res {
                        error!("restarting active window listener due to error: {e:?}");
                    }
                }
            }),
        )
    }
}
