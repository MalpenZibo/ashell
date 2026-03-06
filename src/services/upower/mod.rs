use crate::{IndicatorState, components::icons::StaticIcon};
use dbus::{PowerProfilesProxy, UPowerDbus, UPowerProxy, UpDeviceKind};
use futures::StreamExt;
use guido::prelude::*;
use log::{error, warn};
use std::time::Duration;

pub mod dbus;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct BatteryData {
    pub capacity: i64,
    pub status: BatteryStatus,
}

impl BatteryData {
    pub fn get_indicator_state(&self) -> IndicatorState {
        match self {
            BatteryData {
                status: BatteryStatus::Charging(_),
                ..
            } => IndicatorState::Success,
            BatteryData {
                status: BatteryStatus::Discharging(_),
                capacity,
            } if *capacity < 20 => IndicatorState::Danger,
            _ => IndicatorState::Normal,
        }
    }

    pub fn get_icon(&self) -> StaticIcon {
        match self {
            BatteryData {
                status: BatteryStatus::Charging(_),
                ..
            } => StaticIcon::BatteryCharging,
            BatteryData {
                status: BatteryStatus::Discharging(_),
                capacity,
            } if *capacity < 20 => StaticIcon::Battery0,
            BatteryData {
                status: BatteryStatus::Discharging(_),
                capacity,
            } if *capacity < 40 => StaticIcon::Battery1,
            BatteryData {
                status: BatteryStatus::Discharging(_),
                capacity,
            } if *capacity < 60 => StaticIcon::Battery2,
            BatteryData {
                status: BatteryStatus::Discharging(_),
                capacity,
            } if *capacity < 80 => StaticIcon::Battery3,
            _ => StaticIcon::Battery4,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Peripheral {
    pub name: String,
    pub kind: PeripheralDeviceKind,
    pub data: BatteryData,
    pub device_path: String,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum PeripheralDeviceKind {
    Keyboard,
    Mouse,
    Headphones,
    Gamepad,
}

impl PeripheralDeviceKind {
    pub fn get_icon(&self) -> StaticIcon {
        match self {
            PeripheralDeviceKind::Keyboard => StaticIcon::Keyboard,
            PeripheralDeviceKind::Mouse => StaticIcon::Mouse,
            PeripheralDeviceKind::Headphones => StaticIcon::Headphones1,
            PeripheralDeviceKind::Gamepad => StaticIcon::Gamepad,
        }
    }
}

#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub enum BatteryStatus {
    Charging(Duration),
    Discharging(Duration),
    #[default]
    Full,
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum PowerProfile {
    Balanced,
    Performance,
    PowerSaver,
    #[default]
    Unknown,
}

impl From<String> for PowerProfile {
    fn from(s: String) -> PowerProfile {
        match s.as_str() {
            "balanced" => PowerProfile::Balanced,
            "performance" => PowerProfile::Performance,
            "power-saver" => PowerProfile::PowerSaver,
            _ => PowerProfile::Unknown,
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

#[derive(Clone, PartialEq, guido::SignalFields)]
pub struct UPowerData {
    pub system_battery: Option<BatteryData>,
    pub peripherals: Vec<Peripheral>,
    pub power_profile: PowerProfile,
}

impl Default for UPowerData {
    fn default() -> Self {
        Self {
            system_battery: None,
            peripherals: Vec::new(),
            power_profile: PowerProfile::Unknown,
        }
    }
}

#[derive(Clone)]
pub enum UPowerCmd {
    TogglePowerProfile,
}

pub fn create() -> (UPowerDataSignals, Service<UPowerCmd>) {
    let data = UPowerDataSignals::new(UPowerData::default());
    let svc = start_upower_service(data.writers());
    (data, svc)
}

async fn initialize_system_battery(
    conn: &zbus::Connection,
) -> anyhow::Result<Option<(BatteryData, Vec<String>)>> {
    let upower = UPowerDbus::new(conn).await?;
    let battery = upower.get_system_batteries().await?;

    match battery {
        Some(battery) => {
            let state = battery.state().await;
            let status = match state {
                dbus::DeviceState::Charging => BatteryStatus::Charging(Duration::from_secs(
                    battery.time_to_full().await as u64,
                )),
                dbus::DeviceState::Discharging => BatteryStatus::Discharging(Duration::from_secs(
                    battery.time_to_empty().await as u64,
                )),
                dbus::DeviceState::FullyCharged => BatteryStatus::Full,
                _ => BatteryStatus::Discharging(Duration::from_secs(0)),
            };
            let percentage = match battery.percentage().await {
                Ok(pct) => pct as i64,
                Err(_) => return Ok(None),
            };
            let paths = battery.get_devices_path();
            Ok(Some((
                BatteryData {
                    capacity: percentage,
                    status,
                },
                paths,
            )))
        }
        None => Ok(None),
    }
}

async fn initialize_peripherals(conn: &zbus::Connection) -> anyhow::Result<Vec<Peripheral>> {
    let upower = UPowerDbus::new(conn).await?;
    let devices = upower.get_peripheral_batteries().await?;

    let mut peripherals = Vec::with_capacity(devices.len());
    for device in devices {
        let Ok(device_type) = device.device_type().await else {
            continue;
        };
        let device_kind = match UpDeviceKind::from_u32(device_type).unwrap_or_default() {
            UpDeviceKind::Mouse => PeripheralDeviceKind::Mouse,
            UpDeviceKind::Keyboard => PeripheralDeviceKind::Keyboard,
            UpDeviceKind::Headphones | UpDeviceKind::Headset => PeripheralDeviceKind::Headphones,
            UpDeviceKind::GamingInput => PeripheralDeviceKind::Gamepad,
            _ => continue,
        };

        let name = device
            .model()
            .await
            .unwrap_or_else(|_| format!("{device_kind:?}"));

        let Ok(state) = device.state().await else {
            continue;
        };
        let status = match state {
            1 => BatteryStatus::Charging(Duration::from_secs(
                device.time_to_full().await.unwrap_or(0) as u64,
            )),
            2 => BatteryStatus::Discharging(Duration::from_secs(
                device.time_to_empty().await.unwrap_or(0) as u64,
            )),
            4 => BatteryStatus::Full,
            _ => BatteryStatus::Discharging(Duration::from_secs(0)),
        };
        let Ok(percentage) = device.percentage().await else {
            continue;
        };

        let device_path = device.inner().path().to_string();

        peripherals.push(Peripheral {
            name,
            kind: device_kind,
            data: BatteryData {
                capacity: percentage as i64,
                status,
            },
            device_path,
        });
    }
    Ok(peripherals)
}

fn start_upower_service(writers: UPowerDataWriters) -> Service<UPowerCmd> {
    create_service::<UPowerCmd, _, _>(move |mut rx, ctx| async move {
        let conn = match zbus::Connection::system().await {
            Ok(c) => c,
            Err(e) => {
                error!("Failed to connect to system bus for upower: {e}");
                return;
            }
        };

        // Initialize
        let battery_result = initialize_system_battery(&conn).await;
        let peripherals = initialize_peripherals(&conn).await.unwrap_or_default();
        let power_profile = match PowerProfilesProxy::new(&conn).await {
            Ok(pp) => pp
                .active_profile()
                .await
                .map(PowerProfile::from)
                .unwrap_or_default(),
            Err(e) => {
                warn!("Failed to get power profile: {e}");
                PowerProfile::Unknown
            }
        };

        let _battery_paths = match &battery_result {
            Ok(Some((data, paths))) => {
                writers.system_battery.set(Some(*data));
                Some(paths.clone())
            }
            _ => {
                writers.system_battery.set(None);
                None
            }
        };
        writers.peripherals.set(peripherals.clone());
        writers.power_profile.set(power_profile);

        // Set up event streams for battery changes
        let upower_proxy = match UPowerProxy::new(&conn).await {
            Ok(p) => p,
            Err(e) => {
                error!("Failed to create UPower proxy: {e}");
                // Still handle commands
                while ctx.is_running() {
                    if let Some(cmd) = rx.recv().await {
                        handle_upower_cmd(&conn, &writers, cmd).await;
                    } else {
                        break;
                    }
                }
                return;
            }
        };

        // Listen for device add/remove (triggers peripheral refresh)
        let mut device_added = match upower_proxy.receive_device_added().await {
            Ok(s) => s.map(|_| UPowerEvent::DeviceChanged).boxed(),
            Err(_) => futures::stream::pending().boxed(),
        };
        let mut device_removed = match upower_proxy.receive_device_removed().await {
            Ok(s) => s.map(|_| UPowerEvent::DeviceChanged).boxed(),
            Err(_) => futures::stream::pending().boxed(),
        };

        // Listen for power profile changes
        let pp_proxy = PowerProfilesProxy::new(&conn).await.ok();
        let mut pp_stream = match &pp_proxy {
            Some(pp) => pp
                .receive_active_profile_changed()
                .await
                .map(|_| UPowerEvent::PowerProfileChanged)
                .boxed(),
            None => futures::stream::pending().boxed(),
        };

        // Poll interval for battery refresh (30s)
        let mut battery_tick = tokio::time::interval(Duration::from_secs(30));
        battery_tick.tick().await; // Skip first immediate tick

        while ctx.is_running() {
            tokio::select! {
                cmd = rx.recv() => {
                    match cmd {
                        Some(cmd) => handle_upower_cmd(&conn, &writers, cmd).await,
                        None => break,
                    }
                }
                _ = device_added.next() => {
                    if let Ok(p) = initialize_peripherals(&conn).await {
                        writers.peripherals.set(p);
                    }
                }
                _ = device_removed.next() => {
                    if let Ok(p) = initialize_peripherals(&conn).await {
                        writers.peripherals.set(p);
                    }
                }
                _ = pp_stream.next() => {
                    if let Some(pp) = &pp_proxy {
                        let profile = pp.cached_active_profile()
                            .map(|d| d.map(PowerProfile::from).unwrap_or_default())
                            .unwrap_or_default();
                        writers.power_profile.set(profile);
                    }
                }
                _ = battery_tick.tick() => {
                    // Refresh battery data
                    if let Ok(Some((data, _))) = initialize_system_battery(&conn).await {
                        writers.system_battery.set(Some(data));
                    }
                    if let Ok(p) = initialize_peripherals(&conn).await {
                        writers.peripherals.set(p);
                    }
                }
            }
        }
    })
}

enum UPowerEvent {
    DeviceChanged,
    PowerProfileChanged,
}

async fn handle_upower_cmd(conn: &zbus::Connection, writers: &UPowerDataWriters, cmd: UPowerCmd) {
    match cmd {
        UPowerCmd::TogglePowerProfile => {
            if let Ok(pp) = PowerProfilesProxy::new(conn).await {
                let current = pp
                    .active_profile()
                    .await
                    .map(PowerProfile::from)
                    .unwrap_or_default();
                let next = match current {
                    PowerProfile::Balanced => "performance",
                    PowerProfile::Performance => "power-saver",
                    PowerProfile::PowerSaver => "balanced",
                    PowerProfile::Unknown => return,
                };
                let _ = pp.set_active_profile(next).await;
                writers
                    .power_profile
                    .set(PowerProfile::from(next.to_string()));
            }
        }
    }
}
