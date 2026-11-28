//! The machine's state, and what of it can be set from the bar: the
//! `battery`, `peripheral_battery`, `power_profile`, `bluetooth`,
//! `idle_inhibitor`, `brightness`, `volume` and `microphone` items, and the
//! `settings` panel.

mod config;
mod items;
mod panel;
#[cfg(test)]
pub mod sample;

use std::time::{Duration, Instant};

use ashell_services::audio::{Audio, AudioCommand, AudioData, Device, NORMAL_VOLUME, Route};
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
    audio: Linked<Audio>,
    bluetooth: Linked<Bluetooth>,
    brightness: Linked<Brightness>,
    idle: Idle,
    asked: [Asked; 3],
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
            audio: link(),
            bluetooth: link(),
            brightness: link(),
            idle: Idle::new(),
            asked: std::array::from_fn(|_| Asked::new()),
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
            TOGGLE_MUTE => self.audio.commands.send(Sound::Output.toggle_mute()),
            VOLUME_UP => self.step_volume(Sound::Output, true),
            VOLUME_DOWN => self.step_volume(Sound::Output, false),
            TOGGLE_MICROPHONE_MUTE => self.audio.commands.send(Sound::Input.toggle_mute()),
            MICROPHONE_UP => self.step_volume(Sound::Input, true),
            MICROPHONE_DOWN => self.step_volume(Sound::Input, false),
            _ => {}
        }
    }

    /// `volume_step` of 100%, up to the sound's limit or where it already is
    /// above it.
    fn step_volume(self, sound: Sound, up: bool) {
        let Some(current) = self
            .audio
            .data
            .with_untracked(|data| sound.device(data).map(|d| d.volume))
        else {
            return;
        };
        let current = self.asked(Some(sound)).or(current);
        let (step, max) = self.config.into_signal().with_untracked(|config| {
            (
                volume(config.volume_step as f32),
                volume(sound.max_percent(config) as f32),
            )
        });
        let volume = if up {
            (current + step).min(max.max(current))
        } else {
            current.saturating_sub(step)
        };

        self.audio
            .commands
            .send(sound.set(self.asked(Some(sound)).ask(volume)));
    }

    /// Sets `sound` to `percent` of 100%, unmuting it.
    fn set_volume(self, sound: Sound, percent: f32) {
        if self.audio.data.with_untracked(|data| sound.muted(data)) {
            self.audio.commands.send(sound.unmute());
        }

        self.audio
            .commands
            .send(sound.set(self.asked(Some(sound)).ask(volume(percent))));
    }

    /// Sets the backlight to `percent`, never under its lowest level that
    /// lights.
    fn set_brightness(self, percent: f32) {
        let Some(data) = self.brightness.data.get_untracked() else {
            return;
        };
        let value = (percent / 100.0 * data.max as f32).round() as u32;

        self.brightness.commands.send(BrightnessCommand(
            self.asked(None).ask(value.max(data.max.min(1))),
        ));
    }

    /// A twentieth of the range, as ashell's. Down stops at 1, not 0: a raw 0
    /// blanks the screen.
    fn step_brightness(self, up: bool) {
        let Some(data) = self.brightness.data.get_untracked() else {
            return;
        };
        let current = self.asked(None).or(data.current);
        let step = (data.max / 20).max(1);
        let value = if up {
            (current + step).min(data.max)
        } else {
            current.saturating_sub(step).max(data.max.min(1))
        };

        self.brightness
            .commands
            .send(BrightnessCommand(self.asked(None).ask(value)));
    }

    /// What was last asked of `sound`, or of the backlight.
    fn asked(self, sound: Option<Sound>) -> Asked {
        self.asked[match sound {
            Some(Sound::Output) => 0,
            Some(Sound::Input) => 1,
            None => 2,
        }]
    }
}

/// The last level asked of a device, as ashell's: for a second a step starts
/// from it rather than from the device's last report, so notches that come
/// faster than the device answers add up instead of each repeating the same
/// step.
#[derive(Clone, Copy)]
struct Asked(RwSignal<Option<(u32, Instant)>>);

impl Asked {
    const HELD: Duration = Duration::from_secs(1);

    fn new() -> Self {
        Self(create_signal(None))
    }

    /// The level asked less than a second ago, `reported` otherwise.
    fn or(self, reported: u32) -> u32 {
        self.0.with_untracked(|asked| match asked {
            Some((level, at)) if at.elapsed() < Self::HELD => *level,
            _ => reported,
        })
    }

    fn ask(self, level: u32) -> u32 {
        self.0.set(Some((level, Instant::now())));
        level
    }
}

/// The output or the input: what the volume or the microphone controls.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Sound {
    Output,
    Input,
}

impl Sound {
    /// The default sink, or the default source.
    fn device(self, data: &AudioData) -> Option<&Device> {
        match self {
            Sound::Output => data.active_sink(),
            Sound::Input => data.active_source(),
        }
    }

    /// The name of the default sink, or of the default source.
    fn default(self, data: &AudioData) -> &str {
        match self {
            Sound::Output => &data.default_sink,
            Sound::Input => &data.default_source,
        }
    }

    /// What can be chosen for it: each device, through each of its ports.
    fn routes(self, data: &AudioData) -> Box<dyn Iterator<Item = Route<'_>> + '_> {
        match self {
            Sound::Output => Box::new(data.sink_routes()),
            Sound::Input => Box::new(data.source_routes()),
        }
    }

    /// Whether there is more than one route to choose from.
    fn several(self, data: &AudioData) -> bool {
        match self {
            Sound::Output => data.has_multiple_sinks(),
            Sound::Input => data.has_multiple_sources(),
        }
    }

    fn muted(self, data: &AudioData) -> bool {
        self.device(data).is_some_and(|device| device.is_mute)
    }

    /// How loud, in percent of 100%: zero while muted, as ashell's slider.
    fn percent(self, data: &AudioData) -> Option<f32> {
        self.device(data).map(|device| {
            if device.is_mute {
                0.0
            } else {
                percent(device.volume)
            }
        })
    }

    /// How loud it can be set, in percent: the output to `max_volume`, as
    /// ashell's, the input to 100.
    fn max_percent(self, config: &SettingsConfig) -> u32 {
        match self {
            Sound::Output => config.max_volume,
            Sound::Input => 100,
        }
    }

    fn set(self, volume: u32) -> AudioCommand {
        match self {
            Sound::Output => AudioCommand::SinkVolume(volume),
            Sound::Input => AudioCommand::SourceVolume(volume),
        }
    }

    /// Makes `device` the default, through `port` when it has several.
    fn choose(self, device: String, port: Option<String>) -> AudioCommand {
        match self {
            Sound::Output => AudioCommand::DefaultSink(device, port),
            Sound::Input => AudioCommand::DefaultSource(device, port),
        }
    }

    fn toggle_mute(self) -> AudioCommand {
        match self {
            Sound::Output => AudioCommand::ToggleSinkMute,
            Sound::Input => AudioCommand::ToggleSourceMute,
        }
    }

    fn unmute(self) -> AudioCommand {
        match self {
            Sound::Output => AudioCommand::SetSinkMute(false),
            Sound::Input => AudioCommand::SetSourceMute(false),
        }
    }
}

/// A device's raw `volume`, in percent of 100%.
fn percent(volume: u32) -> f32 {
    volume as f32 * 100.0 / NORMAL_VOLUME as f32
}

/// `percent` of 100%, as a device's raw volume.
fn volume(percent: f32) -> u32 {
    (percent / 100.0 * NORMAL_VOLUME as f32).round() as u32
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
const TOGGLE_MUTE: &str = "toggle_mute";
const VOLUME_UP: &str = "volume_up";
const VOLUME_DOWN: &str = "volume_down";
const TOGGLE_MICROPHONE_MUTE: &str = "toggle_microphone_mute";
const MICROPHONE_UP: &str = "microphone_up";
const MICROPHONE_DOWN: &str = "microphone_down";

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
        ItemId::Volume => &[TOGGLE_MUTE, VOLUME_UP, VOLUME_DOWN],
        ItemId::Microphone => &[TOGGLE_MICROPHONE_MUTE, MICROPHONE_UP, MICROPHONE_DOWN],
        _ => &[],
    }
}

/// ashell's: the wheel on the brightness, the volume and the microphone
/// sets them.
fn default_gestures(id: ItemId) -> Gestures {
    match id {
        ItemId::Brightness => wheel(BRIGHTNESS_UP, BRIGHTNESS_DOWN),
        ItemId::Volume => wheel(VOLUME_UP, VOLUME_DOWN),
        ItemId::Microphone => wheel(MICROPHONE_UP, MICROPHONE_DOWN),
        _ => Gestures::default(),
    }
}

fn wheel(up: &'static str, down: &'static str) -> Gestures {
    Gestures {
        scroll_up: Some(Action::Named(up)),
        scroll_down: Some(Action::Named(down)),
        ..Gestures::default()
    }
}

fn check_options(id: ItemId, options: &Table) -> Vec<String> {
    items::Options::read(id, options).1
}

fn panel_width(_panel: Panel) -> f32 {
    panel::width()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_step_starts_from_the_level_just_asked() {
        let asked = Asked::new();

        assert_eq!(asked.or(40), 40);
        asked.ask(45);
        assert_eq!(asked.or(40), 45);
    }
}
