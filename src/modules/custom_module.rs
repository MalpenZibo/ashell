use std::{any::TypeId, process::Stdio};

use crate::{
    app::{self},
    components::icons::{Icons, icon, icon_raw},
    config::CustomModuleDef,
};
use iced::{widget::canvas, Padding};
use iced::{mouse::Cursor, widget::{canvas::{Cache, Geometry, Path, Program}, container}};
use iced::{
    Alignment, Color, Element, Point, Subscription, Theme,
    stream::channel,
    widget::{
        Stack,
        row, text,
    }, Length,
};
use log::error;
use serde::Deserialize;
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::Command,
};

use super::{Module, OnModulePress};

#[derive(Default, Debug, Clone)]
pub struct Custom {
    data: CustomListenData,
}

impl Custom {
    pub fn update(&mut self, msg: Message) {
        match msg {
            Message::Update(data) => {
                self.data = data;
            }
        }
    }
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct CustomListenData {
    pub alt: String,
    pub text: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Message {
    Update(CustomListenData),
}

// Define a struct for the canvas program
#[derive(Debug, Clone, Copy, Default)]
struct AlertIndicator;

impl<Message> Program<Message> for AlertIndicator {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &iced::Renderer,
        _theme: &Theme,
        bounds: iced::Rectangle,
        _cursor: Cursor,
    ) -> Vec<Geometry> {
        let cache = Cache::new(); // Use a local cache for simplicity here

        vec![cache.draw(renderer, bounds.size(), |frame| {
            let center = frame.center();
            // Use a smaller radius so the circle doesn't touch the canvas edges
            let radius = 2.0; // Creates a 4px diameter circle
            let circle = Path::circle(center, radius);
            frame.fill(&circle, Color::from_rgb8(243, 139, 168)); // Red color
        })]
    }
}


impl Module for Custom {
    type ViewData<'a> = &'a CustomModuleDef;
    type SubscriptionData<'a> = &'a CustomModuleDef;

    fn view(
        &self,
        config: Self::ViewData<'_>,
    ) -> Option<(Element<app::Message>, Option<OnModulePress>)> {
        let mut icon_element = config
            .icon
            .as_ref()
            .map_or_else(|| icon(Icons::None), |text| icon_raw(text.clone()));

        if let Some(icons_map) = &config.icons {
            for (pattern_str, icon_str) in icons_map {
                if let Ok(re) = regex::Regex::new(pattern_str) {
                    if re.is_match(&self.data.alt) {
                        icon_element = icon_raw(icon_str.clone());
                        break; // Use the first match
                    }
                } else {
                    error!("Invalid regex pattern in config: {}", pattern_str);
                }
            }
        }

        // Wrap the icon in a container to apply padding
        let padded_icon_container = container(icon_element).padding([0,1]);

        let mut show_alert = false;
        if let Some(alert_pattern) = &config.alert {
            if let Ok(re) = regex::Regex::new(alert_pattern) {
                if re.is_match(&self.data.alt) {
                    show_alert = true;
                }
            } else {
                error!("Invalid alert regex pattern in config: {}", alert_pattern);
            }
        }

        let icon_with_alert = if show_alert {
            let alert_canvas = canvas(AlertIndicator)
                .width(Length::Fixed(5.0)) // Size of the dot
                .height(Length::Fixed(5.0));

            // Container to position the dot at the top-right
            let alert_indicator_container = container(alert_canvas)
                .width(Length::Fill) // Take full width of the stack item
                .height(Length::Fill) // Take full height
                .align_x(iced::alignment::Horizontal::Right)
                .align_y(iced::alignment::Vertical::Top);
                // Optional: Add padding to nudge it slightly
                // .padding([2, 2, 0, 0]); // top, right, bottom, left

            Stack::new()
                .push(padded_icon_container) // Padded icon is the base layer
                .push(alert_indicator_container) // Dot container on top
                .into()
        } else {
            padded_icon_container.into() // No alert, just the padded icon
        };

        let maybe_text_element = self.data.text.as_ref().and_then(|text_content| {
            if !text_content.is_empty() {
                Some(text(text_content.clone()))
            } else {
                None
            }
        });

        let row_content = if let Some(text_element) = maybe_text_element {
            row![icon_with_alert, text_element].spacing(8).into()
        } else {
            icon_with_alert
        };

        Some((
            row_content,
            Some(OnModulePress::Action(app::Message::LaunchCommand(
                config.command.clone(),
            ))),
        ))
    }

    fn subscription(
        &self,
        config: Self::SubscriptionData<'_>,
    ) -> Option<Subscription<app::Message>> {
        if let Some(check_cmd) = config.listen_cmd.clone() {
            let id = TypeId::of::<Self>();
            let name = config.name.clone();

            Some(Subscription::run_with_id(
                format!("{:?}-{}", id, name),
                channel(10, async move |mut output| {
                    let command = Command::new("bash")
                        .arg("-c")
                        .arg(&check_cmd)
                        .stdout(Stdio::piped())
                        .spawn();

                    match command {
                        Ok(mut child) => {
                            let mut reader = BufReader::new(child.stdout.take().unwrap()).lines();

                            // Ensure the child process is spawned in the runtime so it can
                            // make progress on its own while we await for any output.
                            tokio::spawn(async move {
                                let status = child
                                    .wait()
                                    .await
                                    .expect("child process encountered an error");

                                println!("child status was: {}", status);
                            });

                            while let Some(line) = reader.next_line().await.unwrap() {
                                match serde_json::from_str(&line) {
                                    Ok(event) => output
                                        .try_send(app::Message::CustomUpdate(
                                            name.clone(),
                                            Message::Update(event),
                                        ))
                                        .unwrap(),
                                    Err(e) => {
                                        error!("Failed to parse JSON: {} for line {}", e, line);
                                    }
                                }
                            }
                        }
                        Err(error) => {
                            error!("Failed to execute command: {}", error);
                        }
                    }
                }),
            ))
        } else {
            None
        }
    }
}
