use crate::{
    components::icons::{DynamicIcon, StaticIcon, icon},
    components::{ButtonHierarchy, ButtonKind, ButtonUIRef, position_button},
    config::CustomModuleDef,
    theme::use_theme,
    utils::launcher::execute_command,
};
use iced::widget::canvas;
use iced::{
    Element, Length, Subscription, SurfaceId, Theme,
    stream::channel,
    widget::{Space, Stack, column, row, text},
};
use iced::{
    mouse::Cursor,
    widget::{
        canvas::{Cache, Geometry, Path, Program},
        container,
    },
};
use log::{error, info, warn};
use serde::Deserialize;
use std::process::Stdio;
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::Command,
};

#[derive(Debug, Clone)]
pub struct Custom {
    pub config: CustomModuleDef,
    data: CustomListenData,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct CustomListenData {
    pub alt: String,
    pub text: Option<String>,
    #[serde(default)]
    pub tooltip: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Message {
    LaunchCommand,
    LaunchRightClickCommand,
    LaunchMiddleClickCommand,
    LaunchScrollUpCommand,
    LaunchScrollDownCommand,
    Update(CustomListenData),
    TooltipHover(ButtonUIRef, SurfaceId),
    TooltipUnhover(SurfaceId),
}

pub enum Action {
    None,
    OpenTooltipMenu(SurfaceId, ButtonUIRef),
    CloseTooltipMenu(SurfaceId),
    /// The tooltip went away while possibly shown: its hover button is gone
    /// from the view, so no unhover will ever close it.
    CloseAllTooltipMenus,
}

// Define a struct for the canvas program
#[derive(Debug, Clone, Copy, Default)]
struct AlertIndicator;

impl<Message> Program<Message> for AlertIndicator {
    type State = Cache;

    fn draw(
        &self,
        cache: &Self::State,
        renderer: &iced::Renderer,
        theme: &Theme,
        bounds: iced::Rectangle,
        _cursor: Cursor,
    ) -> Vec<Geometry> {
        let geometry = cache.draw(renderer, bounds.size(), |frame| {
            let center = frame.center();
            // Use a smaller radius so the circle doesn't touch the canvas edges
            let radius = 2.0; // Creates a 4px diameter circle
            let circle = Path::circle(center, radius);
            frame.fill(&circle, theme.palette().danger);
        });

        vec![geometry]
    }
}

impl Custom {
    pub fn new(config: CustomModuleDef) -> Self {
        Self {
            config,
            data: CustomListenData::default(),
        }
    }

    /// Builds the module for a reloaded config. The `listen_cmd` subscription is
    /// keyed on `(name, listen_cmd)`, so when both are unchanged the running
    /// process is kept and will not reprint: carry its last payload over.
    pub fn reconfigure(previous: Option<Self>, config: CustomModuleDef) -> Self {
        let data = previous
            .filter(|prev| {
                prev.config.name == config.name && prev.config.listen_cmd == config.listen_cmd
            })
            .map(|prev| prev.data)
            .unwrap_or_default();

        Self { config, data }
    }

    pub fn module_type(&self) -> crate::config::CustomModuleType {
        self.config.r#type
    }

    #[cfg(test)]
    pub(crate) fn data(&self) -> &CustomListenData {
        &self.data
    }

    pub fn update(&mut self, msg: Message) -> Action {
        match msg {
            Message::LaunchCommand => {
                if let Some(cmd) = &self.config.command {
                    execute_command(cmd);
                }
                Action::None
            }
            Message::LaunchRightClickCommand => {
                if let Some(cmd) = &self.config.on_right_click {
                    execute_command(cmd);
                }
                Action::None
            }
            Message::LaunchMiddleClickCommand => {
                if let Some(cmd) = &self.config.on_middle_click {
                    execute_command(cmd);
                }
                Action::None
            }
            Message::LaunchScrollUpCommand => {
                if let Some(cmd) = &self.config.on_scroll_up {
                    execute_command(cmd);
                }
                Action::None
            }
            Message::LaunchScrollDownCommand => {
                if let Some(cmd) = &self.config.on_scroll_down {
                    execute_command(cmd);
                }
                Action::None
            }
            Message::Update(data) => {
                let had_tooltip = self.tooltip().is_some();
                self.data = data;
                if had_tooltip && self.tooltip().is_none() {
                    Action::CloseAllTooltipMenus
                } else {
                    Action::None
                }
            }
            Message::TooltipHover(ui_ref, id) => Action::OpenTooltipMenu(id, ui_ref),
            Message::TooltipUnhover(id) => Action::CloseTooltipMenu(id),
        }
    }

    fn tooltip(&self) -> Option<&str> {
        self.data.tooltip.as_deref().filter(|t| !t.is_empty())
    }

    pub fn view(&'_ self, id: SurfaceId) -> Element<'_, Message> {
        let space = use_theme(|theme| theme.space);
        let content = match self.config.r#type {
            crate::config::CustomModuleType::Text => self
                .data
                .text
                .as_ref()
                .and_then(|text_content| {
                    if !text_content.is_empty() {
                        Some(text(text_content.clone()).into())
                    } else {
                        None
                    }
                })
                .unwrap_or_else(|| Space::new().width(Length::Shrink).into()),
            crate::config::CustomModuleType::Button => {
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

                let show_alert = self
                    .config
                    .alert
                    .as_ref()
                    .is_some_and(|re| re.is_match(&self.data.alt));

                let icon_with_alert = if show_alert {
                    let alert_canvas = canvas(AlertIndicator)
                        .width(Length::Fixed(space.xs)) // Size of the dot
                        .height(Length::Fixed(space.xs));

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
                    row![icon_with_alert, text_element].spacing(space.xs).into()
                } else {
                    icon_with_alert
                }
            }
        };

        if self.tooltip().is_some() {
            let hover_style = use_theme(|theme| {
                theme.button_style(ButtonKind::Transparent, ButtonHierarchy::Secondary)
            });
            position_button(content)
                .width(Length::Shrink)
                .height(Length::Shrink)
                .padding(0)
                .style(hover_style)
                .on_hover_with_position(move |ui_ref| Message::TooltipHover(ui_ref, id))
                .on_unhover(Message::TooltipUnhover(id))
                .into()
        } else {
            content
        }
    }

    pub fn tooltip_view(&'_ self) -> Element<'_, Message> {
        let space = use_theme(|theme| theme.space);
        let lines: Vec<Element<'_, Message>> = self
            .tooltip()
            .unwrap_or_default()
            .lines()
            .map(|line| text(line).into())
            .collect();
        column(lines).spacing(space.xs).into()
    }

    pub fn subscription(&self) -> Subscription<(String, Message)> {
        let name = self.config.name.clone();
        if let Some(listen_cmd) = self.config.listen_cmd.clone() {
            Subscription::run_with((name, listen_cmd), |data| {
                let (name, listen_cmd) = data.clone();
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
                                let mut buf = String::new();

                                // Ensure the child process is spawned in the runtime so it can
                                // make progress on its own while we await for any output.
                                tokio::spawn(async move {
                                    match child.wait().await {
                                        Ok(status) => info!("child status was: {status}"),
                                        Err(e) => warn!("child process encountered an error: {e}"),
                                    }
                                });

                                while let Some(line) = reader.next_line().await.ok().flatten() {
                                    buf.push_str(&line);
                                    buf.push('\n');
                                    match serde_json::from_str::<CustomListenData>(&buf) {
                                        Ok(event) => {
                                            buf.clear();
                                            if let Err(e) = output
                                                .try_send((name.clone(), Message::Update(event)))
                                            {
                                                error!(
                                                    "Failed to send update for custom module '{name}': {e}"
                                                );
                                            }
                                        }
                                        Err(e) if e.is_eof() => {
                                            if buf.len() > 1 << 20 {
                                                warn!(
                                                    "custom module '{name}': dropping {} bytes of unterminated JSON",
                                                    buf.len()
                                                );
                                                buf.clear();
                                            }
                                        }
                                        Err(e) => {
                                            error!(
                                                "Failed to parse JSON for custom module '{name}': {e} (payload: {buf})"
                                            );
                                            buf.clear();
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
                })
            })
        } else {
            Subscription::none()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn probe() -> Custom {
        let config = toml::from_str("name = \"probe\"\ntype = \"Text\"")
            .expect("test config should deserialize");
        Custom::new(config)
    }

    fn listen_data(tooltip: Option<&str>) -> Message {
        Message::Update(CustomListenData {
            alt: String::new(),
            text: None,
            tooltip: tooltip.map(str::to_owned),
        })
    }

    /// The hover button only exists while there is a tooltip, so dropping the
    /// tooltip must close any open one: the unhover that normally does it will
    /// never fire.
    #[test]
    fn update_closes_tooltip_when_it_goes_away() {
        let mut custom = probe();
        assert!(matches!(
            custom.update(listen_data(Some("line"))),
            Action::None
        ));
        assert!(matches!(
            custom.update(listen_data(Some(""))),
            Action::CloseAllTooltipMenus
        ));
        assert!(matches!(custom.update(listen_data(None)), Action::None));
    }
}
