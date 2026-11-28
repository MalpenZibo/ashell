//! Airplane mode as a radio policy over rfkill: every Wi-Fi and Bluetooth
//! radio blocked. gsd-rfkill does the same over every radio type.
//!
//! It works without a network daemon, and both network daemons use the same
//! switches: NetworkManager reports `WirelessEnabled` false while the wlan
//! switches are soft-blocked and unblocks them when it is set back to true,
//! and an iwd adapter's `Powered` is its wlan switch's soft block. Wi-Fi turned
//! off in NetworkManager before airplane mode stays off after it.

use crate::{
    rfkill::{self, Control, Kind, Switch, Switches},
    stream::{channel, send},
};
use anyhow::Context;
use futures::{Stream, StreamExt};
use log::{debug, warn};
use std::pin::pin;

const RADIOS: [Kind; 2] = [Kind::Wlan, Kind::Bluetooth];

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum AirplaneMode {
    /// No Wi-Fi or Bluetooth rfkill switch, or no rfkill at all.
    #[default]
    Unavailable,
    /// At least one Wi-Fi or Bluetooth radio is unblocked.
    Off,
    /// Every Wi-Fi and Bluetooth radio is blocked, in software or by a
    /// hardware switch.
    On,
}

impl AirplaneMode {
    pub fn from_switches(switches: &Switches) -> Self {
        let mut radios = radios(switches).peekable();
        if radios.peek().is_none() {
            Self::Unavailable
        } else if radios.all(Switch::blocked) {
            Self::On
        } else {
            Self::Off
        }
    }

    /// Whether `command` turns airplane mode on, `None` when it is unavailable.
    fn target(self, command: AirplaneCommand) -> Option<bool> {
        match (self, command) {
            (Self::Unavailable, _) => None,
            (_, AirplaneCommand::Set(enable)) => Some(enable),
            (Self::Off, AirplaneCommand::Toggle) => Some(true),
            (Self::On, AirplaneCommand::Toggle) => Some(false),
        }
    }
}

fn radios(switches: &Switches) -> impl Iterator<Item = &Switch> {
    switches
        .values()
        .filter(|switch| RADIOS.contains(&switch.kind))
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AirplaneData {
    pub mode: AirplaneMode,
    /// Whether this process may change the rfkill switches. Without it,
    /// airplane mode can only be followed.
    pub writable: bool,
    /// Every Wi-Fi and Bluetooth radio is blocked by a hardware switch, which
    /// keeps airplane mode on.
    pub hardware_blocked: bool,
}

impl AirplaneData {
    fn new(switches: &Switches, writable: bool) -> Self {
        let mode = AirplaneMode::from_switches(switches);
        if mode == AirplaneMode::Unavailable {
            return Self::default();
        }

        Self {
            mode,
            writable,
            hardware_blocked: radios(switches).all(|switch| switch.hard),
        }
    }

    pub fn is_on(&self) -> bool {
        self.mode == AirplaneMode::On
    }

    /// Whether airplane mode can be turned on or off from here.
    pub fn can_change(&self) -> bool {
        self.writable && self.mode != AirplaneMode::Unavailable
    }

    /// Whether `command` turns airplane mode on, `None` when it can't change
    /// anything. Sending the result as [`AirplaneCommand::Set`] acts on this
    /// state, not on whatever it is by the time the command runs.
    pub fn target(&self, command: AirplaneCommand) -> Option<bool> {
        self.can_change()
            .then(|| self.mode.target(command))
            .flatten()
    }

    /// Whether airplane mode is on after `command`, `None` when it can't
    /// change anything. A hardware block keeps it on.
    pub fn after(&self, command: AirplaneCommand) -> Option<bool> {
        self.target(command)
            .map(|enable| enable || self.hardware_blocked)
    }
}

/// Turning airplane mode on soft-blocks every Wi-Fi and Bluetooth switch,
/// off soft-unblocks them all. A hardware block can't be lifted from here,
/// so it keeps airplane mode on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AirplaneCommand {
    Toggle,
    Set(bool),
}

#[derive(Debug, Clone)]
pub struct Airplane {
    _private: (),
}

impl Airplane {
    pub async fn connect() -> anyhow::Result<Self> {
        Ok(Self { _private: () })
    }

    /// Yields the current state first, then a snapshot whenever it changes.
    pub fn updates(&self) -> impl Stream<Item = AirplaneData> + Send + 'static {
        channel(16, |mut output| async move {
            let mut last = None;
            let mut warned = false;
            let mut switches = pin!(rfkill::switches());

            while let Some(state) = switches.next().await {
                let data = state.map_or_else(AirplaneData::default, |state| {
                    AirplaneData::new(&state.switches, state.writable)
                });
                if data.mode != AirplaneMode::Unavailable
                    && !data.writable
                    && !std::mem::replace(&mut warned, true)
                {
                    warn!(
                        "No write access to /dev/rfkill, airplane mode can't be changed: it needs an active systemd-logind seat or the rfkill group"
                    );
                }
                if !send(&mut output, &mut last, data).await {
                    return;
                }
            }
        })
    }

    pub async fn execute(&self, command: AirplaneCommand) -> anyhow::Result<()> {
        let control = Control::open().context("open /dev/rfkill for writing")?;
        let enable = AirplaneMode::from_switches(control.switches())
            .target(command)
            .context("No Wi-Fi or Bluetooth radio to block")?;
        debug!("Setting airplane mode to {enable}");

        control
            .set_soft_blocks(&RADIOS, enable)
            .context("write /dev/rfkill")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wwan() -> Kind {
        Kind::from_raw(5)
    }

    fn switches(radios: &[(Kind, bool, bool)]) -> Switches {
        radios
            .iter()
            .zip(0..)
            .map(|(&(kind, soft, hard), idx)| (idx, Switch { kind, soft, hard }))
            .collect()
    }

    fn mode(radios: &[(Kind, bool, bool)]) -> AirplaneMode {
        AirplaneMode::from_switches(&switches(radios))
    }

    #[test]
    fn unavailable_without_radio_switches() {
        assert_eq!(mode(&[]), AirplaneMode::Unavailable);
        assert_eq!(mode(&[(wwan(), true, false)]), AirplaneMode::Unavailable);
    }

    #[test]
    fn on_when_every_radio_is_blocked() {
        assert_eq!(
            mode(&[(Kind::Bluetooth, true, false), (Kind::Wlan, true, false)]),
            AirplaneMode::On
        );
    }

    #[test]
    fn off_when_any_radio_is_unblocked() {
        assert_eq!(
            mode(&[(Kind::Bluetooth, false, false), (Kind::Wlan, true, false)]),
            AirplaneMode::Off
        );
    }

    #[test]
    fn hard_block_counts_as_blocked() {
        assert_eq!(
            mode(&[(Kind::Bluetooth, true, false), (Kind::Wlan, false, true)]),
            AirplaneMode::On
        );
    }

    #[test]
    fn other_radio_types_are_ignored() {
        assert_eq!(
            mode(&[(Kind::Wlan, true, false), (wwan(), false, false)]),
            AirplaneMode::On
        );
    }

    #[test]
    fn toggle_turns_off_unless_hardware_blocked() {
        let soft = AirplaneData::new(
            &switches(&[(Kind::Wlan, true, false), (Kind::Bluetooth, true, true)]),
            true,
        );
        assert_eq!(soft.target(AirplaneCommand::Toggle), Some(false));
        assert_eq!(soft.after(AirplaneCommand::Toggle), Some(false));

        let hard = AirplaneData::new(
            &switches(&[(Kind::Wlan, false, true), (Kind::Bluetooth, true, true)]),
            true,
        );
        assert!(hard.hardware_blocked);
        assert_eq!(hard.target(AirplaneCommand::Toggle), Some(false));
        assert_eq!(hard.after(AirplaneCommand::Toggle), Some(true));
        assert_eq!(hard.after(AirplaneCommand::Set(false)), Some(true));
    }

    #[test]
    fn no_change_without_write_access_or_radios() {
        let read_only = AirplaneData::new(&switches(&[(Kind::Wlan, false, false)]), false);
        assert!(!read_only.can_change());
        assert_eq!(read_only.target(AirplaneCommand::Toggle), None);
        assert!(!AirplaneData::default().can_change());
        assert_eq!(
            AirplaneData::default().after(AirplaneCommand::Set(true)),
            None
        );
    }
}
