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
    theme::AshellTheme,
};

pub struct Osd {
    config: OsdConfig,
    state: Option<OsdState>,
    timeout_handle: Option<iced::task::Handle>,
}

struct OsdState {
    kind: OsdKind,
    /// Normalised value in 0.0..=1.0
    value: f32,
    muted: bool,
}

#[derive(Debug, Clone, Copy)]
pub enum OsdKind {
    Volume,
    Brightness,
    Airplane,
}

#[derive(Debug, Clone)]
pub enum Message {
    Show {
        kind: OsdKind,
        value: f32,
        muted: bool,
    },
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
            state: None,
            timeout_handle: None,
        }
    }

    pub fn config(&self) -> &OsdConfig {
        &self.config
    }

    pub fn update(&mut self, message: Message) -> Action {
        match message {
            Message::Show { kind, value, muted } => {
                self.state = Some(OsdState { kind, value, muted });

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
                self.state = None;
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

    pub fn view<'a>(&'a self, theme: &'a AshellTheme) -> Element<'a, Message> {
        let Some(state) = &self.state else {
            return row![].into();
        };

        let icon = match state.kind {
            OsdKind::Volume => AudioSettings::speaker_icon(state.muted, state.value),
            OsdKind::Brightness => StaticIcon::Brightness,
            OsdKind::Airplane => StaticIcon::Airplane,
        };

        let detail: Element<'_, Message> = match state.kind {
            OsdKind::Volume | OsdKind::Brightness => {
                let mut bar = progress_bar(0.0..=1.0, state.value)
                    .length(160.0)
                    .girth(8.0);
                if state.muted {
                    bar = bar.style(progress_bar::secondary);
                }
                container(bar).center_x(Length::Fill).into()
            }
            OsdKind::Airplane => {
                // For toggles, `muted` carries the active/enabled state.
                let label = if state.muted { "Enabled" } else { "Disabled" };
                container(text(label)).center_x(Length::Fill).into()
            }
        };

        let content = row![icon.to_text().size(theme.font_size.xxl), detail,]
            .spacing(theme.space.sm)
            .align_y(Alignment::Center);

        container(content)
            .padding([theme.space.sm, theme.space.md])
            .style(|t: &Theme| container::Style {
                background: Some(t.palette().background.into()),
                border: Border::default()
                    .width(1)
                    .color(theme.iced_theme.extended_palette().background.weakest.color)
                    .rounded(theme.radius.xl),

                ..Default::default()
            })
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into()
    }
}
