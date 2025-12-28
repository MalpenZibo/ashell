use crate::{
    components::icons::{DynamicIcon, StaticIcon, icon},
    config::CustomModuleDef,
    theme::AshellTheme,
    utils::launcher::execute_command,
};
use iced::widget::canvas;
use iced::{
    Element, Length, Subscription, Theme,
    stream::channel,
    widget::{Stack, row, text},
};
use iced::{
    mouse::Cursor,
    widget::{
        canvas::{Cache, Geometry, Path, Program},
        container,
    },
};
use log::{error, info};
use serde::Deserialize;
use std::{any::TypeId, process::Stdio};
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::Command,
};

#[derive(Debug, Clone)]
pub struct Custom {
    config: CustomModuleDef,
    data: CustomListenData,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct CustomListenData {
    pub alt: String,
    pub text: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Message {
    LaunchCommand,
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
        theme: &Theme,
        bounds: iced::Rectangle,
        _cursor: Cursor,
    ) -> Vec<Geometry> {
        let cache = Cache::new(); // Use a local cache for simplicity here

        vec![cache.draw(renderer, bounds.size(), |frame| {
            let center = frame.center();
            // Use a smaller radius so the circle doesn't touch the canvas edges
            let radius = 2.0; // Creates a 4px diameter circle
            let circle = Path::circle(center, radius);
            frame.fill(&circle, theme.palette().danger);
        })]
    }
}

impl Custom {
    pub fn new(config: CustomModuleDef) -> Self {
        Self {
            config,
            data: CustomListenData::default(),
        }
    }

    pub fn update(&mut self, msg: Message) {
        match msg {
            Message::LaunchCommand => {
                execute_command(self.config.command.clone());
            }
            Message::Update(data) => {
                self.data = data;
            }
        }
    }

    pub fn view(&'_ self, theme: &AshellTheme) -> Element<'_, Message> {
        let mut icon_element = self.config.icon.as_ref().map_or_else(
            || icon(StaticIcon::None),
            |text| icon(DynamicIcon(text.clone())),
        );

        if let Some(icons_map) = &self.config.icons {
            for (re, icon_str) in icons_map {
                if re.is_match(&self.data.alt) {
                    icon_element = icon(DynamicIcon(icon_str.clone()));
                    break; // Use the first match
                }
            }
        }

        // Wrap the icon in a container to apply padding
        let padded_icon_container = container(icon_element).padding([0, 1]);

        let show_alert = if let Some(re) = &self.config.alert
            && re.is_match(&self.data.alt)
        {
            true
        } else {
            false
        };

        let icon_with_alert = if show_alert {
            let alert_canvas = canvas(AlertIndicator)
                .width(Length::Fixed(theme.space.xs as f32)) // Size of the dot
                .height(Length::Fixed(theme.space.xs as f32));

            // Container to position the dot at the top-right
            let alert_indicator_container = container(alert_canvas)
                .width(Length::Fill) // Take full width of the stack item
                .height(Length::Fill) // Take full height
                .align_x(iced::alignment::Horizontal::Right)
                .align_y(iced::alignment::Vertical::Top);

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

        if let Some(text_element) = maybe_text_element {
            row![icon_with_alert, text_element]
                .spacing(theme.space.xs)
                .into()
        } else {
            icon_with_alert
        }
    }

    pub fn subscription(&self) -> Subscription<(String, Message)> {
        let id = TypeId::of::<Self>();
        let name = self.config.name.clone();
        if let Some(listen_cmd) = self.config.listen_cmd.clone() {
            Subscription::run_with_id(
                (id, name.clone(), listen_cmd.clone()),
                channel(10, async move |mut output| {
                    let command = Command::new("bash")
                        .arg("-c")
                        .arg(&listen_cmd)
                        .stdout(Stdio::piped())
                        .spawn();

                    match command {
                        Ok(mut child) => {
                            if let Some(stdout) = child.stdout.take() {
                                let mut reader = BufReader::new(stdout).lines();

                                // Ensure the child process is spawned in the runtime so it can
                                // make progress on its own while we await for any output.
                                tokio::spawn(async move {
                                    let status = child
                                        .wait()
                                        .await
                                        .expect("child process encountered an error");

                                    info!("child status was: {status}");
                                });

                                while let Some(line) = reader.next_line().await.ok().flatten() {
                                    match serde_json::from_str(&line) {
                                        Ok(event) => output
                                            .try_send((name.clone(), Message::Update(event)))
                                            .unwrap(),
                                        Err(e) => {
                                            error!("Failed to parse JSON: {e} for line {line}");
                                        }
                                    }
                                }
                            } else {
                                error!("Failed to capture stdout for command: {listen_cmd}");
                            }
                        }
                        Err(error) => {
                            error!("Failed to execute command: {error}");
                        }
                    }
                }),
            )
        } else {
            Subscription::none()
        }
    }
}
