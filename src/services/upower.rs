use super::{ReadOnlyService, Service, ServiceEvent};
use crate::{
    components::icons::StaticIcon,
    utils::{IndicatorState, remote_value::Remote},
};
use ashell_services::{throttle::ThrottleExt, upower::UPower};
use iced::{
    Subscription, Task,
    futures::{SinkExt, StreamExt},
    stream::channel,
};
use log::{error, info, warn};
use std::{any::TypeId, ops::Deref, pin::pin, time::Duration};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio_stream::wrappers::UnboundedReceiverStream;

pub use ashell_services::upower::{
    BatteryData, BatteryStatus, Peripheral, PeripheralDeviceKind, PowerProfile, UPowerCommand,
    UPowerData,
};

const KEYBOARD_BATTERY_ICONS: [StaticIcon; 5] = [
    StaticIcon::KeyboardBatteryCharging,
    StaticIcon::KeyboardBatteryFull,
    StaticIcon::KeyboardBatteryMedium,
    StaticIcon::KeyboardBatteryLow,
    StaticIcon::KeyboardBatteryAlert,
];

const MOUSE_BATTERY_ICONS: [StaticIcon; 5] = [
    StaticIcon::MouseBatteryCharging,
    StaticIcon::MouseBatteryFull,
    StaticIcon::MouseBatteryMedium,
    StaticIcon::MouseBatteryLow,
    StaticIcon::MouseBatteryAlert,
];

const HEADPHONE_BATTERY_ICONS: [StaticIcon; 5] = [
    StaticIcon::HeadphoneBatteryCharging,
    StaticIcon::HeadphoneBatteryFull,
    StaticIcon::HeadphoneBatteryMedium,
    StaticIcon::HeadphoneBatteryLow,
    StaticIcon::HeadphoneBatteryAlert,
];

const GAMEPAD_BATTERY_ICONS: [StaticIcon; 5] = [
    StaticIcon::GamepadBatteryCharging,
    StaticIcon::GamepadBatteryFull,
    StaticIcon::GamepadBatteryMedium,
    StaticIcon::GamepadBatteryLow,
    StaticIcon::GamepadBatteryAlert,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(usize)]
enum BatLevel {
    Charging = 0,
    Full = 1,
    Medium = 2,
    Low = 3,
    Alert = 4,
}

pub trait BatteryDataExt {
    fn get_indicator_state(&self) -> IndicatorState;

    fn get_icon(&self) -> StaticIcon;
}

impl BatteryDataExt for BatteryData {
    fn get_indicator_state(&self) -> IndicatorState {
        match self {
            BatteryData {
                status: BatteryStatus::Charging(_),
                ..
            } => IndicatorState::Success,
            BatteryData {
                status: BatteryStatus::Discharging(_),
                capacity,
                ..
            } if *capacity < 20 => IndicatorState::Danger,
            BatteryData {
                status: BatteryStatus::Discharging(_),
                ..
            } => IndicatorState::Normal,
            BatteryData {
                status: BatteryStatus::NotCharging | BatteryStatus::Unknown | BatteryStatus::Full,
                ..
            } => IndicatorState::Normal,
        }
    }

    fn get_icon(&self) -> StaticIcon {
        match self {
            BatteryData {
                status: BatteryStatus::Charging(_),
                ..
            } => StaticIcon::BatteryCharging,
            BatteryData {
                status: BatteryStatus::Full,
                ..
            } => StaticIcon::Battery4,
            BatteryData {
                status: BatteryStatus::Discharging(_),
                capacity,
                ..
            } => battery_level_icon(*capacity),
            BatteryData {
                status: BatteryStatus::NotCharging,
                capacity,
                ..
            } => battery_level_icon(*capacity),
            // No dedicated unknown battery icon. Use Battery0 as a safe fallback
            // instead of incorrectly showing a full battery.
            BatteryData {
                status: BatteryStatus::Unknown,
                ..
            } => StaticIcon::Battery0,
        }
    }
}

pub trait PeripheralExt {
    fn get_icon_state(&self) -> StaticIcon;
}

impl PeripheralExt for Peripheral {
    fn get_icon_state(&self) -> StaticIcon {
        let get_type_icon =
            |bat_level: BatLevel| -> StaticIcon { battery_icon(self.kind, bat_level) };

        match self.data {
            BatteryData {
                status: BatteryStatus::Charging(_),
                ..
            } => get_type_icon(BatLevel::Charging),
            BatteryData {
                status: BatteryStatus::Discharging(_),
                capacity,
                ..
            } if capacity < 10 => get_type_icon(BatLevel::Alert),
            BatteryData {
                status: BatteryStatus::Discharging(_),
                capacity,
                ..
            } if capacity < 40 => get_type_icon(BatLevel::Low),
            BatteryData {
                status: BatteryStatus::Discharging(_),
                capacity,
                ..
            } if capacity < 70 => get_type_icon(BatLevel::Medium),
            BatteryData {
                status: BatteryStatus::Discharging(_),
                ..
            } => get_type_icon(BatLevel::Full),
            // Peripheral icons are coarse category/status icons.
            // NotCharging is not active discharge, so keep the full-like state.
            BatteryData {
                status: BatteryStatus::NotCharging | BatteryStatus::Full,
                ..
            } => get_type_icon(BatLevel::Full),
            BatteryData {
                status: BatteryStatus::Unknown,
                ..
            } => get_type_icon(BatLevel::Alert),
        }
    }
}

pub trait PeripheralDeviceKindExt {
    fn get_icon(self) -> StaticIcon;
}

impl PeripheralDeviceKindExt for PeripheralDeviceKind {
    fn get_icon(self) -> StaticIcon {
        match self {
            PeripheralDeviceKind::Keyboard => StaticIcon::Keyboard,
            PeripheralDeviceKind::Mouse => StaticIcon::Mouse,
            PeripheralDeviceKind::Headphones => StaticIcon::Headphones1,
            PeripheralDeviceKind::Gamepad => StaticIcon::Gamepad,
        }
    }
}

impl From<PowerProfile> for StaticIcon {
    fn from(profile: PowerProfile) -> Self {
        match profile {
            PowerProfile::Balanced => StaticIcon::Balanced,
            PowerProfile::Performance => StaticIcon::Performance,
            PowerProfile::PowerSaver => StaticIcon::PowerSaver,
            PowerProfile::Unknown => StaticIcon::None,
        }
    }
}

fn battery_icon(kind: PeripheralDeviceKind, level: BatLevel) -> StaticIcon {
    let icons = match kind {
        PeripheralDeviceKind::Keyboard => &KEYBOARD_BATTERY_ICONS,
        PeripheralDeviceKind::Mouse => &MOUSE_BATTERY_ICONS,
        PeripheralDeviceKind::Headphones => &HEADPHONE_BATTERY_ICONS,
        PeripheralDeviceKind::Gamepad => &GAMEPAD_BATTERY_ICONS,
    };
    icons[level as usize]
}

fn battery_level_icon(capacity: i64) -> StaticIcon {
    if capacity < 20 {
        StaticIcon::Battery0
    } else if capacity < 40 {
        StaticIcon::Battery1
    } else if capacity < 60 {
        StaticIcon::Battery2
    } else if capacity < 80 {
        StaticIcon::Battery3
    } else {
        StaticIcon::Battery4
    }
}

#[derive(Debug, Clone)]
pub struct KbdBacklight {
    pub max: u32,
    pub current: Remote<u32>,
    pub retained: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct UPowerService {
    handle: UPower,
    data: UPowerData,
    pub kbd_backlight: Option<KbdBacklight>,
    kbd_commander: UnboundedSender<u32>,
}

impl Deref for UPowerService {
    type Target = UPowerData;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl UPowerService {
    fn sync_kbd_backlight(&mut self) {
        self.kbd_backlight = match (self.kbd_backlight.take(), self.data.kbd_backlight) {
            (Some(mut state), Some(data)) => {
                state.max = data.max;
                state.current.receive(data.current);
                Some(state)
            }
            (None, Some(data)) => Some(KbdBacklight {
                max: data.max,
                current: Remote::new(data.current),
                retained: None,
            }),
            (_, None) => None,
        };
    }

    // Slider drags send a value per frame: forward them to UPower at most every 100ms
    fn start_kbd_commander(handle: UPower, commands: UnboundedReceiver<u32>) {
        tokio::spawn(async move {
            let mut commands =
                UnboundedReceiverStream::new(commands).throttle(Duration::from_millis(100));
            while let Some(value) = commands.next().await {
                if let Err(err) = handle.execute(UPowerCommand::SetKbdBacklight(value)).await {
                    error!("Failed to set keyboard backlight: {err}");
                }
            }
        });
    }
}

impl ReadOnlyService for UPowerService {
    type UpdateEvent = UPowerData;
    type Error = ();

    fn update(&mut self, event: Self::UpdateEvent) {
        self.data = event;
        self.sync_kbd_backlight();
    }

    fn subscribe() -> Subscription<ServiceEvent<Self>> {
        Subscription::run_with(TypeId::of::<Self>(), |_| {
            channel(100, async |mut output| {
                let handle = match UPower::connect().await {
                    Ok(handle) => handle,
                    Err(err) => {
                        error!("Failed to connect to system bus for upower: {err}");
                        return;
                    }
                };

                let mut updates = pin!(handle.updates());
                let Some(data) = updates.next().await else {
                    return;
                };
                info!("UPower service initialized");

                let (kbd_commander, commands) = tokio::sync::mpsc::unbounded_channel();
                Self::start_kbd_commander(handle.clone(), commands);

                let mut service = UPowerService {
                    handle,
                    data,
                    kbd_backlight: None,
                    kbd_commander,
                };
                service.sync_kbd_backlight();
                let _ = output.send(ServiceEvent::Init(service)).await;

                while let Some(data) = updates.next().await {
                    let _ = output.send(ServiceEvent::Update(data)).await;
                }
            })
        })
    }
}

impl Service for UPowerService {
    type Command = UPowerCommand;

    fn command(&mut self, command: Self::Command) -> Task<ServiceEvent<Self>> {
        if let UPowerCommand::SetKbdBacklight(value) = command {
            let _ = self.kbd_commander.send(value);
            return Task::none();
        }

        let handle = self.handle.clone();

        Task::future(async move {
            if let Err(err) = handle.execute(command).await {
                warn!("UPower command failed: {err}");
            }
        })
        .discard()
    }
}

#[cfg(test)]
mod tests {
    use super::{BatteryData, BatteryDataExt, BatteryStatus, StaticIcon};

    #[test]
    fn unknown_system_battery_uses_empty_fallback_icon() {
        let battery = BatteryData {
            capacity: 50,
            status: BatteryStatus::Unknown,
            is_discharging: false,
        };

        assert!(matches!(battery.get_icon(), StaticIcon::Battery0));
    }

    #[test]
    fn full_system_battery_uses_full_icon_even_low_capacity() {
        let battery = BatteryData {
            capacity: 10,
            status: BatteryStatus::Full,
            is_discharging: false,
        };

        assert!(matches!(battery.get_icon(), StaticIcon::Battery4));
    }

    #[test]
    fn not_charging_system_battery_keeps_capacity_icon() {
        let battery = BatteryData {
            capacity: 10,
            status: BatteryStatus::NotCharging,
            is_discharging: false,
        };

        assert!(matches!(battery.get_icon(), StaticIcon::Battery0));
    }
}
