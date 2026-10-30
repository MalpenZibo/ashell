use std::time::Duration;

use iced::{
    Background, Border, Color, Element, Length, Task, Theme,
    widget::{blur_container, container, progress_bar, row, text},
};
use tokio::time::sleep;

use crate::{
    components,
    components::icons::{Icon, StaticIcon},
    components::motion::BASE,
    config::{OsdConfig, Surface},
    modules::settings::audio::AudioSettings,
    modules::settings::network::NetworkSettings,
    services::idle_inhibitor,
    theme::{surface_border, use_theme},
};
use ashell_i18n::t;

pub struct Osd {
    config: OsdConfig,
    state: Option<OsdState>,
    timeout_handle: Option<iced::task::Handle>,
    animations_enabled: bool,
    /// Set while the fade-out plays, so a re-show revives the surface instead
    /// of waiting for teardown.
    hiding: bool,
}

struct OsdState {
    kind: OsdKind,
    /// Normalised value relative to 100% hardware (can exceed 1.0 for overdrive)
    value: f32,
    scale: f32,
    muted: bool,
}

#[derive(Debug, Clone, Copy)]
pub enum OsdKind {
    Volume,
    Microphone,
    Brightness,
    Airplane,
    IdleInhibitor,
}

#[derive(Debug, Clone)]
pub enum Message {
    Show {
        kind: OsdKind,
        value: f32,
        scale: f32,
        muted: bool,
    },
    Hide,
    ConfigReloaded {
        config: OsdConfig,
        animations_enabled: bool,
    },
}

pub enum Action {
    None,
    /// OSD state updated — caller should ensure the layer surface exists and
    /// run the returned timer task.
    Show(Task<Message>),
    /// Fade finished (or animation disabled) — caller must destroy the layer
    /// surface.
    Hide,
}

/// Ink for the OSD's text and own-coloured paint: danger for a muted toggle,
/// the theme text colour otherwise.
fn osd_ink(theme: &Theme, state: &OsdState) -> Color {
    match (state.kind, state.muted) {
        (OsdKind::IdleInhibitor | OsdKind::Airplane, true) => theme.palette().danger,
        _ => theme.palette().text,
    }
}

/// `t` of the way to `style`, since iced has no alpha layer and a widget only
/// fades the colours it paints itself.
fn fade_progress_bar(style: progress_bar::Style, t: f32) -> progress_bar::Style {
    let fade = |background: Background| match background {
        Background::Color(color) => Background::Color(color.scale_alpha(t)),
        other => other,
    };

    progress_bar::Style {
        background: fade(style.background),
        bar: fade(style.bar),
        border: Border {
            color: style.border.color.scale_alpha(t),
            ..style.border
        },
    }
}

impl Osd {
    pub fn new(config: OsdConfig) -> Self {
        Self {
            config,
            state: None,
            timeout_handle: None,
            animations_enabled: false,
            hiding: false,
        }
    }

    pub fn config(&self) -> &OsdConfig {
        &self.config
    }

    pub fn update(&mut self, message: Message) -> Action {
        match message {
            Message::Show {
                kind,
                value,
                scale,
                muted,
            } => {
                self.state = Some(OsdState {
                    kind,
                    value,
                    scale,
                    muted,
                });
                // A show during the fade-out cancels it and fades back in.
                self.hiding = false;

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
                if let Some(handle) = self.timeout_handle.take() {
                    handle.abort();
                }

                if !self.animations_enabled || self.hiding {
                    self.state = None;
                    return Action::Hide;
                }

                // Keep the content around for the fade-out; the `Hide` that
                // follows tears the surface down.
                self.hiding = true;
                Action::Show(self.fade_out_timer())
            }

            Message::ConfigReloaded {
                config,
                animations_enabled,
            } => {
                self.config = config;
                let dropped_mid_fade_out =
                    self.animations_enabled && !animations_enabled && self.hiding;
                self.animations_enabled = animations_enabled;

                // Motion switched off during a fade-out: hide without waiting
                // for the timer, which still sees a surface on screen.
                if dropped_mid_fade_out && self.state.is_some() {
                    self.state = None;
                    Action::Hide
                } else {
                    Action::None
                }
            }
        }
    }

    /// Fires [`Message::Hide`] once the fade-out has finished, so the surface
    /// is not destroyed around a half-faded OSD.
    fn fade_out_timer(&self) -> Task<Message> {
        // A little margin so the final frame lands before teardown.
        let delay = BASE + Duration::from_millis(24);
        Task::perform(async move { sleep(delay).await }, |()| Message::Hide)
    }

    pub fn view(&self) -> Element<'_, Message> {
        let Some(state) = &self.state else {
            return row![].into();
        };

        let (space, font_size, radius, blur) =
            use_theme(|t| (t.space, t.font_size, t.radius, t.surface(Surface::Osd).blur));

        let overdrive = matches!(state.kind, OsdKind::Volume) && state.value > 1.0;

        let icon = match state.kind {
            OsdKind::Volume => AudioSettings::speaker_icon(state.muted, state.value, overdrive),
            OsdKind::Microphone => AudioSettings::microphone_icon(state.muted),
            OsdKind::Brightness => StaticIcon::Brightness,
            OsdKind::Airplane => NetworkSettings::airplane_mode_icon(state.muted),
            OsdKind::IdleInhibitor => idle_inhibitor::icon(state.muted),
        };

        let show_percentage = match state.kind {
            OsdKind::Volume | OsdKind::Microphone => self.config.show_volume_percentage,
            OsdKind::Brightness => self.config.show_brightness_percentage,
            _ => false,
        };

        // Nothing in the panel paints until the OSD is on screen, so entering
        // draws at zero and the fade decides how much shows.
        let ink = use_theme(|t| osd_ink(&t.surface(Surface::Osd).iced_theme, state));
        let entering = if self.state.is_some() { 0.0 } else { 1.0 };

        let detail: Element<'_, Message> = match state.kind {
            OsdKind::Volume | OsdKind::Microphone | OsdKind::Brightness => {
                let bar = progress_bar(0.0..=state.scale, state.value)
                    .length(160.0)
                    .girth(8.0);
                let style = if state.muted {
                    crate::theme::progress_bar_secondary
                } else if overdrive {
                    crate::theme::progress_bar_danger
                } else {
                    crate::theme::progress_bar_primary
                };
                let bar = bar.style(move |t: &Theme| fade_progress_bar(style(t), entering));
                if show_percentage {
                    let pct = (state.value * 100.0).round() as u32;
                    row![
                        container(bar).center_x(Length::Fill),
                        text(format!("{pct}%")).size(font_size.sm),
                    ]
                    .spacing(space.sm)
                    .align_y(iced::Alignment::Center)
                    .into()
                } else {
                    container(bar).center_x(Length::Fill).into()
                }
            }
            OsdKind::Airplane | OsdKind::IdleInhibitor => {
                // For toggles, `muted` carries the active/enabled state.
                let state_key = if state.muted { "on" } else { "off" };
                let label = match state.kind {
                    OsdKind::Airplane => t!("osd-airplane-toggle", state = state_key),
                    OsdKind::IdleInhibitor => t!("osd-idle-inhibitor-toggle", state = state_key),
                    _ => unreachable!(),
                };
                container(text(label)).center_x(Length::Fill).into()
            }
        };

        let content = row![
            container(
                icon.to_text()
                    .size(font_size.xxl)
                    .center()
                    .color(ink.scale_alpha(entering))
            )
            .center_x(font_size.xxl),
            detail,
        ]
        .spacing(space.sm)
        .align_y(iced::Alignment::Center);

        let panel_style = move |_: &Theme| container::Style {
            text_color: Some(ink),
            ..Default::default()
        };

        let (background, border) = use_theme(|t| {
            let theme = &t.surface(Surface::Osd).iced_theme;
            (theme.palette().background, surface_border(theme, radius.xl))
        });

        let content: Element<'_, Message> = if blur {
            blur_container(content)
                .padding([space.sm, space.md])
                .style(panel_style)
                .center(Length::Fill)
                .into()
        } else {
            container(content)
                .padding([space.sm, space.md])
                .style(panel_style)
                .center(Length::Fill)
                .width(Length::Fill)
                .height(Length::Fill)
                .into()
        };

        // The surface is created and destroyed with each OSD, so it is always
        // entering: fade it in rather than letting it pop.
        components::panel(self.state.is_some(), content)
            .background(background)
            .border(border)
            .ink(ink)
            .into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn osd(animations_enabled: bool) -> Osd {
        let mut osd = Osd::new(OsdConfig::default());
        osd.update(Message::ConfigReloaded {
            config: OsdConfig::default(),
            animations_enabled,
        });
        osd
    }

    fn show() -> Message {
        Message::Show {
            kind: OsdKind::Volume,
            value: 0.5,
            scale: 1.0,
            muted: false,
        }
    }

    #[test]
    fn hiding_keeps_content_alive_for_the_fade() {
        let mut osd = osd(true);
        assert!(matches!(osd.update(show()), Action::Show(_)));

        // Hide starts a fade-out and returns its timer instead of tearing the
        // surface down straight away.
        assert!(matches!(osd.update(Message::Hide), Action::Show(_)));
        assert!(osd.state.is_some());
        assert!(osd.hiding);
    }

    #[test]
    fn hide_after_fade_tears_down() {
        let mut osd = osd(true);
        osd.update(show());
        osd.update(Message::Hide);

        // The timer fires `Hide` a second time; that is the teardown.
        assert!(matches!(osd.update(Message::Hide), Action::Hide));
        assert!(osd.state.is_none());
    }

    #[test]
    fn showing_again_cancels_the_fade_out() {
        let mut osd = osd(true);
        osd.update(show());
        osd.update(Message::Hide);
        osd.update(show());

        assert!(osd.state.is_some());
        assert!(!osd.hiding, "the OSD must not vanish under the new value");
    }

    #[test]
    fn without_animations_hide_is_immediate() {
        let mut osd = osd(false);
        osd.update(show());

        assert!(matches!(osd.update(Message::Hide), Action::Hide));
        assert!(osd.state.is_none());
    }

    #[test]
    fn disabling_animations_mid_fade_out_hides_now() {
        let mut osd = osd(true);
        osd.update(show());
        osd.update(Message::Hide);

        let action = osd.update(Message::ConfigReloaded {
            config: OsdConfig::default(),
            animations_enabled: false,
        });
        assert!(matches!(action, Action::Hide));
        assert!(osd.state.is_none());
    }
}
