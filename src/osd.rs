use std::time::Duration;

use iced::{
    Alignment, Border, Element, Length, Task, Theme,
    widget::{container, progress_bar, row, text},
};
use tokio::time::sleep;

use crate::{
    components::icons::{Icon, StaticIcon},
    config::OsdConfig,
    modules::settings::audio::AudioSettings,
    modules::settings::network::NetworkSettings,
    services::idle_inhibitor::IdleInhibitorManager,
    t,
    theme::use_theme,
};

pub struct Osd {
    config: OsdConfig,
    message: Option<OsdMessage>,
    timeout_handle: Option<iced::task::Handle>,
}

#[derive(Debug, Clone, Copy)]
pub enum OsdMessage {
    Volume { value: f32, muted: bool },
    Microphone { value: f32, muted: bool },
    Brightness { value: f32 },
    Airplane { active: bool },
    IdleInhibitor { active: bool },
}

#[derive(Debug, Clone)]
pub enum Message {
    Show(OsdMessage),
    Hide,
    ConfigReloaded(OsdConfig),
}

pub enum Action {
    None,
    /// OSD state updated — caller should ensure the layer surface exists and
    /// run the returned timer task.
    Show(Task<Message>),
    /// Timer expired — caller must destroy the layer surface.
    Hide,
}

impl Osd {
    pub fn new(config: OsdConfig) -> Self {
        Self {
            config,
            message: None,
            timeout_handle: None,
        }
    }

    pub fn config(&self) -> &OsdConfig {
        &self.config
    }

    pub fn update(&mut self, message: Message) -> Action {
        match message {
            Message::Show(message) => {
                self.message = Some(message);

                if let Some(handle) = self.timeout_handle.take() {
                    handle.abort();
                }

                let timeout_ms = self.config.timeout;
                let (task, handle) = Task::perform(
                    async move {
                        sleep(Duration::from_millis(timeout_ms)).await;
                    },
                    |()| Message::Hide,
                )
                .abortable();
                self.timeout_handle = Some(handle);

                Action::Show(task)
            }

            Message::Hide => {
                self.message = None;
                if let Some(handle) = self.timeout_handle.take() {
                    handle.abort();
                }
                Action::Hide
            }

            Message::ConfigReloaded(config) => {
                self.config = config;
                Action::None
            }
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        if let Some(message) = self.message {
            let (space, font_size, radius) = use_theme(|t| (t.space, t.font_size, t.radius));

            let icon = match message {
                OsdMessage::Volume { value, muted } => AudioSettings::speaker_icon(value, muted),
                OsdMessage::Microphone { muted, .. } => AudioSettings::microphone_icon(muted),
                OsdMessage::Brightness { .. } => StaticIcon::Brightness,
                OsdMessage::Airplane { active } => NetworkSettings::airplane_mode_icon(active),
                OsdMessage::IdleInhibitor { active } => {
                    IdleInhibitorManager::idle_inhibitor_icon(active)
                }
            };

            let detail: Element<'_, Message> = match message {
                OsdMessage::Volume { value, muted } | OsdMessage::Microphone { value, muted } => {
                    let mut bar = progress_bar(0.0..=1.0, value).length(160.0).girth(8.0);
                    if muted {
                        bar = bar.style(progress_bar::secondary);
                    }
                    container(bar).center_x(Length::Fill).into()
                }
                OsdMessage::Brightness { value } => {
                    let bar = progress_bar(0.0..=1.0, value).length(160.0).girth(8.0);
                    container(bar).center_x(Length::Fill).into()
                }
                OsdMessage::Airplane { active } | OsdMessage::IdleInhibitor { active } => {
                    // For toggles, `muted` carries the active/enabled state.
                    let state_key = if active { "on" } else { "off" };
                    let label = match message {
                        OsdMessage::Airplane { .. } => t!("osd-airplane-toggle", state = state_key),
                        OsdMessage::IdleInhibitor { .. } => {
                            t!("osd-idle-inhibitor-toggle", state = state_key)
                        }
                        _ => unreachable!(),
                    };
                    container(text(label)).center_x(Length::Fill).into()
                }
            };

            let content = row![
                container(icon.to_text().size(font_size.xxl)).center_x(font_size.xxl),
                detail,
            ]
            .spacing(space.sm)
            .align_y(Alignment::Center);

            container(content)
                .padding([space.sm, space.md])
                .style(move |t: &Theme| container::Style {
                    background: Some(t.palette().background.into()),
                    border: Border::default()
                        .width(1)
                        .color(t.extended_palette().background.weakest.color)
                        .rounded(radius.xl),
                    text_color: Some(match message {
                        OsdMessage::IdleInhibitor { active: true } => t.palette().danger,
                        OsdMessage::Airplane { active: true } => t.palette().danger,
                        _ => t.palette().text,
                    }),
                    ..Default::default()
                })
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .into()
        } else {
            row![].into()
        }
    }
}
