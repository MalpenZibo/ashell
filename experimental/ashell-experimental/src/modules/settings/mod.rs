//! The machine's state, and what of it can be set from the bar: the
//! `battery`, `peripheral_battery`, `power_profile`, `bluetooth`,
//! `idle_inhibitor` and `brightness` items, and the `settings` panel.

mod config;
mod items;
mod panel;
#[cfg(test)]
pub mod sample;

use ashell_services::bluetooth::{Bluetooth, BluetoothCommand};
use ashell_services::brightness::{Brightness, BrightnessCommand};
use ashell_services::idle_inhibitor::IdleInhibitor;
use ashell_services::upower::{UPower, UPowerCommand};
use guido::prelude::*;
use toml::Table;

use super::{ItemView, Spec};
use crate::config::{Action, Gestures, Item, ItemId, Panel};
use crate::services::{Linked, link};

pub use config::SettingsConfig;

/// The module's state, shared by every item it draws.
#[derive(Clone, Copy)]
pub struct Settings {
    config: Memo<SettingsConfig>,
    upower: Linked<UPower>,
    bluetooth: Linked<Bluetooth>,
    brightness: Linked<Brightness>,
    idle: Idle,
}

/// The idle inhibitor: ashell's own, since other clients' cannot be seen.
#[derive(Clone, Copy)]
struct Idle {
    /// `None` once the compositor turned it down.
    inhibited: Signal<Option<bool>>,
    toggle: Service<()>,
}

impl Settings {
    pub fn new(config: Memo<SettingsConfig>) -> Self {
        Self {
            config,
            upower: link(),
            bluetooth: link(),
            brightness: link(),
            idle: Idle::new(),
        }
    }

    pub fn item(self, item: &Item) -> ItemView {
        items::view(self, item)
    }

    /// Its one panel.
    pub fn panel(self, _panel: Panel) -> Container {
        panel::view(self)
    }

    fn act(self, action: &str) {
        match action {
            NEXT_POWER_PROFILE => self.upower.commands.send(UPowerCommand::TogglePowerProfile),
            TOGGLE_BLUETOOTH => self.bluetooth.commands.send(BluetoothCommand::Toggle),
            TOGGLE_IDLE_INHIBITOR => self.idle.toggle.send(()),
            BRIGHTNESS_UP => self.step_brightness(true),
            BRIGHTNESS_DOWN => self.step_brightness(false),
            _ => {}
        }
    }

    /// Sets the backlight to `percent`, never under its lowest level that
    /// lights.
    fn set_brightness(self, percent: f32) {
        let Some(data) = self.brightness.data.get_untracked() else {
            return;
        };
        let value = (percent / 100.0 * data.max as f32).round() as u32;

        self.brightness
            .commands
            .send(BrightnessCommand(value.max(data.max.min(1))));
    }

    /// A twentieth of the range, as ashell's. Down stops at 1, not 0: a raw 0
    /// blanks the screen.
    fn step_brightness(self, up: bool) {
        let Some(data) = self.brightness.data.get_untracked() else {
            return;
        };
        let step = (data.max / 20).max(1);
        let value = if up {
            (data.current + step).min(data.max)
        } else {
            data.current.saturating_sub(step).max(data.max.min(1))
        };

        self.brightness.commands.send(BrightnessCommand(value));
    }
}

impl Idle {
    /// Connects to the compositor on the first toggle, not before: a bar
    /// that never inhibits keeps no surface for it.
    fn new() -> Self {
        let inhibited = create_signal(Some(false));
        let writer = inhibited.writer();

        let toggle = create_service::<(), _, _>(move |mut rx, _| async move {
            if rx.recv().await.is_none() {
                return;
            }
            let mut inhibitor = match IdleInhibitor::connect() {
                Ok(inhibitor) => inhibitor,
                Err(err) => {
                    log::warn!("settings: no idle inhibitor: {err}");
                    writer.set(None);
                    return;
                }
            };

            loop {
                if let Err(err) = inhibitor.toggle() {
                    log::warn!("settings: the idle inhibitor did not toggle: {err}");
                }
                writer.set(Some(inhibitor.is_inhibited()));

                if rx.recv().await.is_none() {
                    return;
                }
            }
        });

        Self {
            inhibited: inhibited.read_only(),
            toggle,
        }
    }
}

const NEXT_POWER_PROFILE: &str = "next_power_profile";
const TOGGLE_BLUETOOTH: &str = "toggle_bluetooth";
const TOGGLE_IDLE_INHIBITOR: &str = "toggle_idle_inhibitor";
const BRIGHTNESS_UP: &str = "brightness_up";
const BRIGHTNESS_DOWN: &str = "brightness_down";

pub const SPEC: Spec = Spec {
    actions,
    default_gestures,
    check_options,
    states: items::states,
    notable: items::notable,
    panel_width,
    ..Spec::bare("settings")
};

fn actions(id: ItemId) -> &'static [&'static str] {
    match id {
        ItemId::PowerProfile => &[NEXT_POWER_PROFILE],
        ItemId::Bluetooth => &[TOGGLE_BLUETOOTH],
        ItemId::IdleInhibitor => &[TOGGLE_IDLE_INHIBITOR],
        ItemId::Brightness => &[BRIGHTNESS_UP, BRIGHTNESS_DOWN],
        _ => &[],
    }
}

/// ashell's: the wheel on the brightness sets it.
fn default_gestures(id: ItemId) -> Gestures {
    match id {
        ItemId::Brightness => Gestures {
            scroll_up: Some(Action::Named(BRIGHTNESS_UP)),
            scroll_down: Some(Action::Named(BRIGHTNESS_DOWN)),
            ..Gestures::default()
        },
        _ => Gestures::default(),
    }
}

fn check_options(id: ItemId, options: &Table) -> Vec<String> {
    items::Options::read(id, options).1
}

fn panel_width(_panel: Panel) -> f32 {
    panel::width()
}
