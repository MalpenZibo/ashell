use super::{ReadOnlyService, Service, ServiceEvent};
use crate::{
    components::icons::StaticIcon, services::throttle::ThrottleExt, utils::IndicatorState,
};
use dbus::{DeviceProxy, PowerProfilesProxy, SystemBattery, UPowerDbus, UPowerProxy, UpDeviceKind};
use iced::{
    Subscription,
    futures::{
        SinkExt, Stream, StreamExt,
        channel::mpsc::Sender,
        stream::{once, pending, select_all},
        stream_select,
    },
    stream::channel,
};
use log::{error, warn};
use serde::Deserialize;
use std::{any::TypeId, fmt, time::Duration};
use zbus::zvariant::ObjectPath;

mod dbus;

#[derive(Clone, Copy, Debug)]
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

#[derive(Debug, Clone)]
pub struct Peripheral {
    pub name: String,
    pub kind: PeripheralDeviceKind,
    pub data: BatteryData,
    pub device: DeviceProxy<'static>,
}

impl Peripheral {
    pub fn get_icon_state(&self) -> StaticIcon {
        enum BatLevel {
            Charging,
            Full,
            Medium,
            Low,
            Alert,
        }

        let get_type_icon = |bat_level: BatLevel| -> StaticIcon {
            match (self.kind, bat_level) {
                (PeripheralDeviceKind::Keyboard, BatLevel::Charging) => {
                    StaticIcon::KeyboardBatteryCharging
                }
                (PeripheralDeviceKind::Keyboard, BatLevel::Full) => StaticIcon::KeyboardBatteryFull,
                (PeripheralDeviceKind::Keyboard, BatLevel::Medium) => {
                    StaticIcon::KeyboardBatteryMedium
                }
                (PeripheralDeviceKind::Keyboard, BatLevel::Low) => StaticIcon::KeyboardBatteryLow,
                (PeripheralDeviceKind::Keyboard, BatLevel::Alert) => {
                    StaticIcon::KeyboardBatteryAlert
                }
                (PeripheralDeviceKind::Mouse, BatLevel::Charging) => {
                    StaticIcon::MouseBatteryCharging
                }
                (PeripheralDeviceKind::Mouse, BatLevel::Full) => StaticIcon::MouseBatteryFull,
                (PeripheralDeviceKind::Mouse, BatLevel::Medium) => StaticIcon::MouseBatteryMedium,
                (PeripheralDeviceKind::Mouse, BatLevel::Low) => StaticIcon::MouseBatteryLow,
                (PeripheralDeviceKind::Mouse, BatLevel::Alert) => StaticIcon::MouseBatteryAlert,
                (PeripheralDeviceKind::Headphones, BatLevel::Charging) => {
                    StaticIcon::HeadphoneBatteryCharging
                }
                (PeripheralDeviceKind::Headphones, BatLevel::Full) => {
                    StaticIcon::HeadphoneBatteryFull
                }
                (PeripheralDeviceKind::Headphones, BatLevel::Medium) => {
                    StaticIcon::HeadphoneBatteryMedium
                }
                (PeripheralDeviceKind::Headphones, BatLevel::Low) => {
                    StaticIcon::HeadphoneBatteryLow
                }
                (PeripheralDeviceKind::Headphones, BatLevel::Alert) => {
                    StaticIcon::HeadphoneBatteryAlert
                }
                (PeripheralDeviceKind::Gamepad, BatLevel::Charging) => {
                    StaticIcon::GamepadBatteryCharging
                }
                (PeripheralDeviceKind::Gamepad, BatLevel::Full) => StaticIcon::GamepadBatteryFull,
                (PeripheralDeviceKind::Gamepad, BatLevel::Medium) => {
                    StaticIcon::GamepadBatteryMedium
                }
                (PeripheralDeviceKind::Gamepad, BatLevel::Low) => StaticIcon::GamepadBatteryLow,
                (PeripheralDeviceKind::Gamepad, BatLevel::Alert) => StaticIcon::GamepadBatteryAlert,
            }
        };

        match self.data {
            BatteryData {
                status: BatteryStatus::Charging(_),
                ..
            } => get_type_icon(BatLevel::Charging),
            BatteryData {
                status: BatteryStatus::Discharging(_),
                capacity,
            } if capacity < 10 => get_type_icon(BatLevel::Alert),
            BatteryData {
                status: BatteryStatus::Discharging(_),
                capacity,
            } if capacity < 40 => get_type_icon(BatLevel::Low),
            BatteryData {
                status: BatteryStatus::Discharging(_),
                capacity,
            } if capacity < 70 => get_type_icon(BatLevel::Medium),
            BatteryData {
                status: BatteryStatus::Discharging(_),
                ..
            }
            | BatteryData {
                status: BatteryStatus::Full,
                ..
            } => get_type_icon(BatLevel::Full),
        }
    }
}

#[derive(Deserialize, Copy, Clone, PartialEq, Eq, Debug)]
pub enum PeripheralDeviceKind {
    Keyboard,
    Mouse,
    Headphones,
    Gamepad,
}

impl fmt::Display for PeripheralDeviceKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PeripheralDeviceKind::Keyboard => write!(f, "Keyboard"),
            PeripheralDeviceKind::Mouse => write!(f, "Mouse"),
            PeripheralDeviceKind::Headphones => write!(f, "Headphones"),
            PeripheralDeviceKind::Gamepad => write!(f, "Gamepad"),
        }
    }
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

#[derive(Debug, Clone)]
pub enum UPowerEvent {
    UpdateSystemBattery(BatteryData),
    UpdatePeripherals(Vec<Peripheral>),
    NoBattery,
    UpdatePowerProfile(PowerProfile),
}

#[derive(Copy, Clone, Debug)]
pub enum BatteryStatus {
    Charging(Duration),
    Discharging(Duration),
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
    fn from(power_profile: String) -> PowerProfile {
        match power_profile.as_str() {
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

#[derive(Debug, Clone)]
pub struct UPowerService {
    pub system_battery: Option<BatteryData>,
    pub peripherals: Vec<Peripheral>,
    pub power_profile: PowerProfile,
    conn: zbus::Connection,
}

enum State {
    Init,
    Active(
        zbus::Connection,
        Option<Vec<ObjectPath<'static>>>,
        Vec<ObjectPath<'static>>,
    ),
    Error,
}

impl ReadOnlyService for UPowerService {
    type UpdateEvent = UPowerEvent;
    type Error = ();

    fn update(&mut self, event: Self::UpdateEvent) {
        match event {
            UPowerEvent::UpdateSystemBattery(data) => {
                self.system_battery.replace(data);
            }
            UPowerEvent::UpdatePeripherals(data) => {
                self.peripherals = data;
            }
            UPowerEvent::NoBattery => {
                self.system_battery = None;
            }
            UPowerEvent::UpdatePowerProfile(profile) => {
                self.power_profile = profile;
            }
        }
    }

    fn subscribe() -> Subscription<ServiceEvent<Self>> {
        let id = TypeId::of::<Self>();

        Subscription::run_with_id(
            id,
            channel(100, async |mut output| {
                let mut state = State::Init;

                loop {
                    state = UPowerService::start_listening(state, &mut output).await;
                }
            }),
        )
    }
}

impl UPowerService {
    async fn initialize_data(
        conn: &zbus::Connection,
    ) -> anyhow::Result<(
        Option<(BatteryData, Vec<ObjectPath<'static>>)>,
        Vec<Peripheral>,
        PowerProfile,
    )> {
        let system_battery = UPowerService::initialize_system_battery_data(conn).await?;
        let peripherals = UPowerService::initialize_peripheral_data(conn).await?;

        let power_profile = UPowerService::initialize_power_profile_data(conn).await;

        match (system_battery, power_profile) {
            (Some(battery), Ok(power_profile)) => Ok((
                Some((battery.0, battery.1.get_devices_path())),
                peripherals,
                power_profile,
            )),
            (Some(battery), Err(err)) => {
                warn!("Failed to get power profile: {err}");

                Ok((
                    Some((battery.0, battery.1.get_devices_path())),
                    peripherals,
                    PowerProfile::Unknown,
                ))
            }
            (None, Ok(power_profile)) => Ok((None, peripherals, power_profile)),
            (None, Err(err)) => {
                warn!("Failed to get power profile: {err}");

                Ok((None, peripherals, PowerProfile::Unknown))
            }
        }
    }

    async fn initialize_power_profile_data(
        conn: &zbus::Connection,
    ) -> anyhow::Result<PowerProfile> {
        let powerprofiles = PowerProfilesProxy::new(conn).await?;

        let profile = powerprofiles
            .active_profile()
            .await
            .map(PowerProfile::from)?;

        Ok(profile)
    }

    async fn initialize_system_battery_data(
        conn: &zbus::Connection,
    ) -> anyhow::Result<Option<(BatteryData, SystemBattery)>> {
        let upower = UPowerDbus::new(conn).await?;
        let battery = upower.get_system_batteries().await?;

        match battery {
            Some(battery) => {
                let state = battery.state().await;
                let state = match state {
                    1 => BatteryStatus::Charging(Duration::from_secs(
                        battery.time_to_full().await as u64,
                    )),
                    2 => BatteryStatus::Discharging(Duration::from_secs(
                        battery.time_to_empty().await as u64,
                    )),
                    4 => BatteryStatus::Full,
                    _ => BatteryStatus::Discharging(Duration::from_secs(0)),
                };
                let percentage = battery.percentage().await as i64;

                Ok(Some((
                    BatteryData {
                        capacity: percentage,
                        status: state,
                    },
                    battery,
                )))
            }
            _ => Ok(None),
        }
    }

    async fn initialize_peripheral_data(
        conn: &zbus::Connection,
    ) -> anyhow::Result<Vec<Peripheral>> {
        let upower = UPowerDbus::new(conn).await?;
        let devices = upower.get_peripheral_batteries().await?;

        let mut peripherals = Vec::with_capacity(devices.len());

        for device in devices {
            let Ok(device_type) = device.device_type().await else {
                warn!(
                    "Failed to read device's type for device '{}'",
                    device.inner().path().as_str()
                );
                continue;
            };
            let device_kind = match UpDeviceKind::from_u32(device_type).unwrap_or_default() {
                UpDeviceKind::Mouse => PeripheralDeviceKind::Mouse,
                UpDeviceKind::Keyboard => PeripheralDeviceKind::Keyboard,
                UpDeviceKind::Headphones => PeripheralDeviceKind::Headphones,
                UpDeviceKind::Headset => PeripheralDeviceKind::Headphones,
                UpDeviceKind::GamingInput => PeripheralDeviceKind::Gamepad,
                _ => continue,
            };

            let name = match device.model().await {
                Ok(model) => model,
                Err(_) => device_kind.to_string(),
            };

            let Ok(state) = device.state().await else {
                continue;
            };
            let state = match state {
                1 => {
                    let Ok(time_to_full) = device.time_to_full().await else {
                        warn!(
                            "Failed to read device's time_to_full for device '{}'",
                            device.inner().path().as_str()
                        );
                        continue;
                    };
                    BatteryStatus::Charging(Duration::from_secs(time_to_full as u64))
                }
                2 => {
                    let Ok(time_to_empty) = device.time_to_empty().await else {
                        warn!(
                            "Failed to read device's time_to_empty for device '{}'",
                            device.inner().path().as_str()
                        );
                        continue;
                    };
                    BatteryStatus::Discharging(Duration::from_secs(time_to_empty as u64))
                }
                4 => BatteryStatus::Full,
                _ => BatteryStatus::Discharging(Duration::from_secs(0)),
            };
            let Ok(percentage) = device.percentage().await else {
                warn!(
                    "Failed to read device's percentage for device '{}'",
                    device.inner().path().as_str()
                );
                continue;
            };

            peripherals.push(Peripheral {
                name,
                kind: device_kind,
                data: BatteryData {
                    capacity: percentage as i64,
                    status: state,
                },
                device,
            });
        }

        Ok(peripherals)
    }

    async fn events(
        conn: &zbus::Connection,
        system_battery_devices: &Option<Vec<ObjectPath<'static>>>,
        peripheral_paths: &[ObjectPath<'static>],
    ) -> anyhow::Result<impl Stream<Item = UPowerEvent> + use<>> {
        let system_battery_event = if let Some(battery_devices) = system_battery_devices {
            let upower = UPowerDbus::new(conn).await?;

            let mut events = Vec::new();

            for device_path in battery_devices {
                let device = upower.get_device(device_path).await?;

                events.push(
                    stream_select!(
                        device.receive_state_changed().await.map(|_| ()),
                        device.receive_percentage_changed().await.map(|_| ()),
                        device
                            .receive_time_to_full_changed()
                            .await
                            .throttle(Duration::from_secs(30))
                            .map(|_| ()),
                        device
                            .receive_time_to_empty_changed()
                            .await
                            .throttle(Duration::from_secs(30))
                            .map(|_| ()),
                    )
                    .filter_map({
                        let conn = conn.clone();
                        move |_| {
                            let conn = conn.clone();
                            async move {
                                if let Some((data, _)) = Self::initialize_system_battery_data(&conn)
                                    .await
                                    .ok()
                                    .flatten()
                                {
                                    Some(UPowerEvent::UpdateSystemBattery(data))
                                } else {
                                    None
                                }
                            }
                        }
                    })
                    .boxed(),
                );
            }

            select_all(events).boxed()
        } else {
            once(async {}).map(|_| UPowerEvent::NoBattery).boxed()
        };

        let peripheral_event = if !peripheral_paths.is_empty() {
            let upower = UPowerDbus::new(conn).await?;

            let mut events = Vec::new();

            for device_path in peripheral_paths {
                let device = upower.get_device(device_path).await?;

                events.push(
                    stream_select!(
                        device.receive_state_changed().await.map(|_| ()),
                        device.receive_percentage_changed().await.map(|_| ()),
                        device
                            .receive_time_to_full_changed()
                            .await
                            .throttle(Duration::from_secs(30))
                            .map(|_| ()),
                        device
                            .receive_time_to_empty_changed()
                            .await
                            .throttle(Duration::from_secs(30))
                            .map(|_| ()),
                    )
                    .filter_map({
                        let conn = conn.clone();
                        move |_| {
                            let conn = conn.clone();
                            async move {
                                Self::initialize_peripheral_data(&conn)
                                    .await
                                    .ok()
                                    .map(UPowerEvent::UpdatePeripherals)
                            }
                        }
                    })
                    .boxed(),
                );
            }

            select_all(events).boxed()
        } else {
            pending().boxed()
        };

        let upower_proxy = UPowerProxy::new(conn).await?;
        let device_added_event = upower_proxy
            .receive_device_added()
            .await?
            .filter_map({
                let conn = conn.clone();
                move |_added_device| {
                    let conn = conn.clone();
                    async move {
                        Self::initialize_peripheral_data(&conn)
                            .await
                            .ok()
                            .map(UPowerEvent::UpdatePeripherals)
                    }
                }
            })
            .boxed();

        let device_removed_event = upower_proxy
            .receive_device_removed()
            .await?
            .filter_map({
                let conn = conn.clone();
                move |_removed_device| {
                    let conn = conn.clone();
                    async move {
                        Self::initialize_peripheral_data(&conn)
                            .await
                            .ok()
                            .map(UPowerEvent::UpdatePeripherals)
                    }
                }
            })
            .boxed();

        let powerprofiles = PowerProfilesProxy::new(conn).await?;
        let power_profile_event =
            powerprofiles
                .receive_active_profile_changed()
                .await
                .map(move |_| {
                    UPowerEvent::UpdatePowerProfile(
                        powerprofiles
                            .cached_active_profile()
                            .map(|d| d.map(PowerProfile::from).unwrap_or_default())
                            .unwrap_or_default(),
                    )
                });

        Ok(stream_select!(
            system_battery_event,
            peripheral_event,
            device_added_event,
            device_removed_event,
            power_profile_event
        ))
    }

    async fn start_listening(state: State, output: &mut Sender<ServiceEvent<Self>>) -> State {
        match state {
            State::Init => match zbus::Connection::system().await {
                Ok(conn) => match UPowerService::initialize_data(&conn).await {
                    Ok((system_battery, peripherals, power_profile)) => {
                        let peripheral_paths = peripherals
                            .iter()
                            .map(|p| p.device.inner().path().clone())
                            .collect();

                        let service = UPowerService {
                            system_battery: system_battery.as_ref().map(|b| b.0),
                            peripherals,
                            power_profile,
                            conn: conn.clone(),
                        };
                        let _ = output.send(ServiceEvent::Init(service)).await;

                        State::Active(conn, system_battery.map(|b| b.1), peripheral_paths)
                    }
                    Err(err) => {
                        error!("Failed to initialize upower service: {err}");

                        State::Error
                    }
                },
                Err(err) => {
                    error!("Failed to connect to system bus for upower: {err}",);
                    State::Error
                }
            },
            State::Active(conn, system_battery_paths, peripheral_paths) => {
                match UPowerService::events(&conn, &system_battery_paths, &peripheral_paths).await {
                    Ok(mut events) => {
                        while let Some(event) = events.next().await {
                            let _ = output.send(ServiceEvent::Update(event)).await;
                        }

                        State::Active(conn, system_battery_paths, peripheral_paths)
                    }
                    Err(err) => {
                        error!("Failed to listen for upower events: {err}");

                        State::Error
                    }
                }
            }
            State::Error => {
                let _ = pending::<u8>().next().await;

                State::Error
            }
        }
    }
}

pub enum PowerProfileCommand {
    Toggle,
}

impl Service for UPowerService {
    type Command = PowerProfileCommand;

    fn command(&mut self, command: Self::Command) -> iced::Task<ServiceEvent<Self>> {
        iced::Task::perform(
            {
                let conn = self.conn.clone();
                let power_profile = self.power_profile;
                async move {
                    let powerprofiles = PowerProfilesProxy::new(&conn)
                        .await
                        .expect("Failed to create PowerProfilesProxy");

                    match command {
                        PowerProfileCommand::Toggle => {
                            let current_profile = power_profile;
                            match current_profile {
                                PowerProfile::Balanced => {
                                    let _ = powerprofiles.set_active_profile("performance").await;

                                    PowerProfile::Performance
                                }
                                PowerProfile::Performance => {
                                    let _ = powerprofiles.set_active_profile("power-saver").await;

                                    PowerProfile::PowerSaver
                                }
                                PowerProfile::PowerSaver => {
                                    let _ = powerprofiles.set_active_profile("balanced").await;

                                    PowerProfile::Balanced
                                }
                                PowerProfile::Unknown => PowerProfile::Unknown,
                            }
                        }
                    }
                }
            },
            |power_profile| ServiceEvent::Update(UPowerEvent::UpdatePowerProfile(power_profile)),
        )
    }
}
