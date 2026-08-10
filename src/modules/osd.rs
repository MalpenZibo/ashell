use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

use guido::prelude::*;

use crate::components::{StaticIcon, icon};
use crate::config::{Config, OsdConfig};
use crate::t;
use crate::theme::ThemeColors;

const OSD_WIDTH: u32 = 250;
const OSD_HEIGHT: u32 = 64;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum OsdKind {
    Volume,
    Microphone,
    Brightness,
    Airplane,
    IdleInhibitor,
}

#[derive(Clone, Debug)]
pub struct OsdShow {
    version: u64,
    pub kind: OsdKind,
    /// Normalized value (may exceed 1.0 for volume overdrive).
    pub value: f32,
    /// Progress upper bound (>= 1.0).
    pub scale: f32,
    /// Muted for the bar kinds; active for the toggle kinds.
    pub flag: bool,
}

impl PartialEq for OsdShow {
    fn eq(&self, other: &Self) -> bool {
        self.version == other.version
    }
}

/// On-screen display: transient overlay surface at the bottom of the screen,
/// auto-hidden after the configured timeout. Triggered by IPC commands only,
/// like upstream.
#[derive(Clone)]
pub struct OsdTrigger {
    show: RwSignal<Option<OsdShow>>,
    counter: Rc<Cell<u64>>,
    enabled: bool,
}

impl OsdTrigger {
    pub fn show(&self, kind: OsdKind, value: f32, scale: f32, flag: bool) {
        if !self.enabled {
            return;
        }
        let version = self.counter.get() + 1;
        self.counter.set(version);
        self.show.set(Some(OsdShow {
            version,
            kind,
            value,
            scale,
            flag,
        }));
    }
}

pub fn create() -> OsdTrigger {
    let config = with_context::<Config, _>(|c| c.osd).unwrap_or_default();
    let show = create_signal(None::<OsdShow>);
    let trigger = OsdTrigger {
        show,
        counter: Rc::new(Cell::new(0)),
        enabled: config.enabled,
    };
    if !config.enabled {
        return trigger;
    }

    let theme = expect_context::<ThemeColors>();
    let surface: Rc<RefCell<Option<SurfaceHandle>>> = Rc::new(RefCell::new(None));
    let expired = create_signal(0u64);
    let expired_w = expired.writer();

    // Show: create the surface if needed, (re)arm the hide timer
    let slot = surface.clone();
    create_effect(move || {
        let Some(s) = show.get() else {
            return;
        };
        if slot.borrow().is_none() {
            *slot.borrow_mut() = Some(spawn_surface(
                SurfaceConfig::new()
                    .width(OSD_WIDTH)
                    .height(OSD_HEIGHT)
                    .anchor(Anchor::BOTTOM)
                    .layer(Layer::Overlay)
                    .margin(0, 0, 48, 0)
                    .exclusive_zone(Some(0))
                    .keyboard_interactivity(KeyboardInteractivity::None)
                    .background_color(Color::TRANSPARENT)
                    .namespace("ashell-osd"),
                move || osd_view(show, config, theme),
            ));
        }
        let version = s.version;
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(config.timeout)).await;
            expired_w.set(version);
        });
    })
    .detach();

    // Hide when the newest timer fires (older timers are ignored)
    let slot = surface;
    create_effect(move || {
        let v = expired.get();
        let current = show.with_untracked(|s| s.as_ref().map(|s| s.version));
        if current == Some(v)
            && let Some(h) = slot.borrow_mut().take()
        {
            h.close();
            show.set(None);
        }
    })
    .detach();

    trigger
}

fn osd_view(show: RwSignal<Option<OsdShow>>, config: OsdConfig, theme: ThemeColors) -> impl Widget {
    container()
        .width(fill())
        .height(fill())
        .background(theme.background)
        .corner_radius(16)
        .border(1, theme.background.lighter(0.15))
        .layout(
            Flex::row()
                .spacing(8)
                .cross_alignment(CrossAlignment::Center)
                .main_alignment(MainAlignment::Center),
        )
        .child(move || {
            let s = show.get()?;
            let kind_icon = match s.kind {
                OsdKind::Volume => {
                    if s.flag {
                        StaticIcon::Speaker0
                    } else {
                        match (s.value * 100.0) as i32 {
                            0..=33 => StaticIcon::Speaker1,
                            34..=66 => StaticIcon::Speaker2,
                            _ => StaticIcon::Speaker3,
                        }
                    }
                }
                OsdKind::Microphone => {
                    if s.flag {
                        StaticIcon::Mic0
                    } else {
                        StaticIcon::Mic1
                    }
                }
                OsdKind::Brightness => StaticIcon::Brightness,
                OsdKind::Airplane => StaticIcon::Airplane,
                OsdKind::IdleInhibitor => {
                    if s.flag {
                        StaticIcon::EyeOpened
                    } else {
                        StaticIcon::EyeClosed
                    }
                }
            };

            let mut row = container()
                .layout(
                    Flex::row()
                        .spacing(12)
                        .cross_alignment(CrossAlignment::Center),
                )
                .child(icon().kind(kind_icon).color(theme.text).font_size(20));

            match s.kind {
                OsdKind::Volume | OsdKind::Microphone | OsdKind::Brightness => {
                    let frac = (s.value / s.scale).clamp(0.0, 1.0);
                    let overdrive = s.kind == OsdKind::Volume && s.value > 1.0;
                    let fill_color = if overdrive {
                        theme.danger
                    } else if s.flag {
                        theme.background.lighter(0.3)
                    } else {
                        theme.primary
                    };
                    row = row.child(
                        container()
                            .width(160)
                            .height(8)
                            .corner_radius(4)
                            .background(theme.background.lighter(0.1))
                            .child(
                                container()
                                    .width(160.0 * frac)
                                    .height(8)
                                    .corner_radius(4)
                                    .background(fill_color),
                            ),
                    );
                    let show_pct = match s.kind {
                        OsdKind::Volume | OsdKind::Microphone => config.show_volume_percentage,
                        OsdKind::Brightness => config.show_brightness_percentage,
                        _ => false,
                    };
                    if show_pct {
                        row = row.child(
                            text(format!("{}%", (s.value * 100.0).round()))
                                .color(theme.text)
                                .font_size(12),
                        );
                    }
                }
                OsdKind::Airplane => {
                    let state = if s.flag { "on" } else { "off" };
                    row = row.child(
                        text(t!("osd-airplane-toggle", state = state))
                            .color(if s.flag { theme.danger } else { theme.text })
                            .font_size(13),
                    );
                }
                OsdKind::IdleInhibitor => {
                    let state = if s.flag { "on" } else { "off" };
                    row = row.child(
                        text(t!("osd-idle-inhibitor-toggle", state = state))
                            .color(if s.flag { theme.danger } else { theme.text })
                            .font_size(13),
                    );
                }
            }

            Some(row)
        })
}
