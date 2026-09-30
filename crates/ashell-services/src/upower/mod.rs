use crate::{bus, stream::channel, throttle::ThrottleExt};
use dbus::{
    DeviceProxy, DeviceState, KbdBacklightProxy, PowerProfilesProxy, SystemBattery, UPowerDbus,
    UpDeviceKind,
};
use futures::{
    SinkExt, Stream, StreamExt,
    stream::{BoxStream, select_all},
    stream_select,
};
use log::{debug, error, warn};
use std::{fmt, time::Duration};
use zbus::zvariant::OwnedObjectPath;

mod dbus;

const UPOWER: &str = "org.freedesktop.UPower";
const RETRY_DELAY: Duration = Duration::from_secs(5);
const TIME_ESTIMATE_THROTTLE: Duration = Duration::from_secs(30);

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct BatteryData {
    pub capacity: i64,
    pub status: BatteryStatus,
    pub is_discharging: bool,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum BatteryStatus {
    Charging(Duration),
    Discharging(Duration),
    NotCharging,
    Unknown,
    Full,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Peripheral {
    pub name: String,
    pub kind: PeripheralDeviceKind,
    pub data: BatteryData,
    pub path: OwnedObjectPath,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize))]
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

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
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

impl PowerProfile {
    fn id(self) -> Option<&'static str> {
        match self {
            PowerProfile::Balanced => Some("balanced"),
            PowerProfile::Performance => Some("performance"),
            PowerProfile::PowerSaver => Some("power-saver"),
            PowerProfile::Unknown => None,
        }
    }

    fn next(self) -> PowerProfile {
        match self {
            PowerProfile::Balanced => PowerProfile::Performance,
            PowerProfile::Performance => PowerProfile::PowerSaver,
            PowerProfile::PowerSaver => PowerProfile::Balanced,
            PowerProfile::Unknown => PowerProfile::Unknown,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct ChargeLimit {
    pub enabled: bool,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct KbdBacklight {
    pub max: u32,
    pub current: u32,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct UPowerData {
    pub system_battery: Option<BatteryData>,
    pub charge_limit: Option<ChargeLimit>,
    pub peripherals: Vec<Peripheral>,
    pub power_profile: PowerProfile,
    pub kbd_backlight: Option<KbdBacklight>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum UPowerCommand {
    TogglePowerProfile,
    ToggleChargeLimit,
    SetKbdBacklight(u32),
}

#[derive(PartialEq, Eq)]
enum Change {
    Topology,
    Property,
}

#[derive(Debug, Clone)]
pub struct UPower {
    conn: zbus::Connection,
}

impl UPower {
    pub async fn connect() -> anyhow::Result<Self> {
        let conn = zbus::Connection::system().await?;

        Ok(Self { conn })
    }

    /// Yields the current state once subscribed, then a fresh snapshot on every change.
    pub fn updates(&self) -> impl Stream<Item = UPowerData> + Send + 'static {
        let conn = self.conn.clone();

        channel(100, |mut output| async move {
            loop {
                match bus::has_owner(&conn, UPOWER).await {
                    Ok(true) => {}
                    Ok(false) => {
                        if output.send(UPowerData::default()).await.is_err() {
                            return;
                        }
                        if let Err(err) = bus::owner_acquired(&conn, UPOWER).await {
                            error!("Failed to watch for upower: {err}");
                            return;
                        }
                        continue;
                    }
                    Err(err) => {
                        error!("Failed to look up upower on the system bus: {err}");
                        return;
                    }
                }

                debug!("Listening for upower events");

                let mut changes = match Self::changes(&conn).await {
                    Ok(changes) => changes,
                    Err(err) => {
                        warn!("Failed to listen for upower events: {err}");
                        tokio::time::sleep(RETRY_DELAY).await;
                        continue;
                    }
                };

                loop {
                    match Self::read_data(&conn).await {
                        Ok(data) => {
                            if output.send(data).await.is_err() {
                                return;
                            }
                        }
                        Err(err) => warn!("Failed to read upower data: {err}"),
                    }

                    match changes.next().await {
                        Some(Change::Property) => {}
                        Some(Change::Topology) | None => break,
                    }
                }
            }
        })
    }

    pub async fn execute(&self, command: UPowerCommand) -> anyhow::Result<()> {
        debug!("UPower command: {command:?}");

        match command {
            UPowerCommand::TogglePowerProfile => {
                let current = Self::read_power_profile(&self.conn).await?;
                self.set_power_profile(current.next()).await
            }
            UPowerCommand::ToggleChargeLimit => {
                let upower = UPowerDbus::new(&self.conn).await?;
                let Some(battery) = upower.get_system_batteries().await? else {
                    return Ok(());
                };
                let Some(threshold) = battery.charge_threshold().await else {
                    return Ok(());
                };

                threshold
                    .device
                    .enable_charge_threshold(!threshold.enabled)
                    .await?;

                Ok(())
            }
            UPowerCommand::SetKbdBacklight(value) => self.set_kbd_backlight(value).await,
        }
    }

    async fn set_power_profile(&self, profile: PowerProfile) -> anyhow::Result<()> {
        let Some(id) = profile.id() else {
            return Ok(());
        };

        PowerProfilesProxy::new(&self.conn)
            .await?
            .set_active_profile(id)
            .await?;

        Ok(())
    }

    async fn set_kbd_backlight(&self, value: u32) -> anyhow::Result<()> {
        KbdBacklightProxy::new(&self.conn)
            .await?
            .set_brightness(i32::try_from(value).unwrap_or(i32::MAX))
            .await?;

        Ok(())
    }

    async fn read_data(conn: &zbus::Connection) -> anyhow::Result<UPowerData> {
        let upower = UPowerDbus::new(conn).await?;

        let (system_battery, charge_limit) = match upower.get_system_batteries().await? {
            Some(battery) => match Self::read_system_battery(&battery).await {
                Some(data) => {
                    let charge_limit =
                        battery
                            .charge_threshold()
                            .await
                            .map(|threshold| ChargeLimit {
                                enabled: threshold.enabled,
                            });

                    (Some(data), charge_limit)
                }
                None => (None, None),
            },
            None => (None, None),
        };

        let peripherals = Self::read_peripherals(&upower).await?;

        let power_profile = Self::read_power_profile(conn).await.unwrap_or_else(|err| {
            debug!("Failed to get power profile: {err}");
            PowerProfile::Unknown
        });

        let kbd_backlight = Self::read_kbd_backlight(conn).await;

        Ok(UPowerData {
            system_battery,
            charge_limit,
            peripherals,
            power_profile,
            kbd_backlight,
        })
    }

    async fn read_system_battery(battery: &SystemBattery) -> Option<BatteryData> {
        let state = battery.state().await;
        let status = match state {
            DeviceState::Charging => {
                battery_status_from_timed_system_state(state, battery.time_to_full().await)
            }
            DeviceState::Discharging => {
                battery_status_from_timed_system_state(state, battery.time_to_empty().await)
            }
            _ => battery_status_from_system_state(state),
        };

        // If we can't get percentage data, don't show battery at all
        let capacity = battery.percentage().await.ok()? as i64;

        Some(BatteryData {
            capacity,
            status,
            is_discharging: state == DeviceState::Discharging,
        })
    }

    async fn read_peripherals(upower: &UPowerDbus<'_>) -> anyhow::Result<Vec<Peripheral>> {
        let devices = upower.get_peripheral_batteries().await?;

        let mut peripherals = Vec::with_capacity(devices.len());

        for device in devices {
            let path = device.inner().path();

            let Ok(device_type) = device.device_type().await else {
                warn!("Failed to read device's type for device '{path}'");
                continue;
            };
            let kind = match UpDeviceKind::from_u32(device_type).unwrap_or_default() {
                UpDeviceKind::Mouse | UpDeviceKind::Touchpad => PeripheralDeviceKind::Mouse,
                UpDeviceKind::Keyboard => PeripheralDeviceKind::Keyboard,
                UpDeviceKind::Headphones | UpDeviceKind::Headset => {
                    PeripheralDeviceKind::Headphones
                }
                UpDeviceKind::GamingInput => PeripheralDeviceKind::Gamepad,
                _ => continue,
            };

            let name = match device.model().await {
                Ok(model) => model,
                Err(_) => kind.to_string(),
            };

            let Ok(state_raw) = device.state().await else {
                continue;
            };
            let status = match state_raw {
                1 => {
                    let Ok(time_to_full) = device.time_to_full().await else {
                        warn!("Failed to read device's time_to_full for device '{path}'");
                        continue;
                    };
                    BatteryStatus::Charging(Duration::from_secs(time_to_full as u64))
                }
                2 => {
                    let Ok(time_to_empty) = device.time_to_empty().await else {
                        warn!("Failed to read device's time_to_empty for device '{path}'");
                        continue;
                    };
                    BatteryStatus::Discharging(Duration::from_secs(time_to_empty as u64))
                }
                4 => BatteryStatus::Full,
                5 | 6 => BatteryStatus::NotCharging,
                _ => BatteryStatus::Unknown,
            };
            let Ok(percentage) = device.percentage().await else {
                warn!("Failed to read device's percentage for device '{path}'");
                continue;
            };

            peripherals.push(Peripheral {
                name,
                kind,
                data: BatteryData {
                    capacity: percentage as i64,
                    status,
                    is_discharging: state_raw == 2,
                },
                path: path.to_owned().into(),
            });
        }

        peripherals.sort_by(|a, b| a.path.as_str().cmp(b.path.as_str()));

        Ok(peripherals)
    }

    async fn read_power_profile(conn: &zbus::Connection) -> anyhow::Result<PowerProfile> {
        let profile = PowerProfilesProxy::new(conn)
            .await?
            .active_profile()
            .await
            .map(PowerProfile::from)?;

        Ok(profile)
    }

    async fn read_kbd_backlight(conn: &zbus::Connection) -> Option<KbdBacklight> {
        let proxy = KbdBacklightProxy::new(conn).await.ok()?;

        Some(KbdBacklight {
            max: safe_cast(proxy.get_max_brightness().await.ok()?),
            current: safe_cast(proxy.get_brightness().await.ok()?),
        })
    }

    async fn changes(conn: &zbus::Connection) -> anyhow::Result<BoxStream<'static, Change>> {
        let upower = UPowerDbus::new(conn).await?;

        let topology = stream_select!(
            upower.receive_device_added().await?.map(|_| {}),
            upower.receive_device_removed().await?.map(|_| {}),
            bus::owner_changes(conn, UPOWER).await?,
        )
        .map(|_| Change::Topology);

        // Property streams yield their current value first: skip it, updates() reads a snapshot
        let mut properties: Vec<BoxStream<'static, ()>> = Vec::new();

        if let Some(battery) = upower.get_system_batteries().await? {
            for device in battery.devices() {
                properties.extend(Self::battery_changes(device).await);
                properties.push(
                    device
                        .receive_charge_threshold_supported_changed()
                        .await
                        .skip(1)
                        .map(|_| {})
                        .boxed(),
                );
                properties.push(
                    device
                        .receive_charge_threshold_enabled_changed()
                        .await
                        .skip(1)
                        .map(|_| {})
                        .boxed(),
                );
            }
        }

        for device in upower.get_peripheral_batteries().await? {
            properties.extend(Self::battery_changes(&device).await);
        }

        match PowerProfilesProxy::new(conn).await {
            Ok(power_profiles) => properties.push(
                power_profiles
                    .receive_active_profile_changed()
                    .await
                    .skip(1)
                    .map(|_| {})
                    .boxed(),
            ),
            Err(err) => debug!("Power profiles not available: {err}"),
        }

        match KbdBacklightProxy::new(conn).await {
            Ok(kbd_backlight) => match kbd_backlight.receive_brightness_changed().await {
                Ok(changes) => properties.push(changes.map(|_| {}).boxed()),
                Err(err) => debug!("Keyboard backlight not found: {err}"),
            },
            Err(err) => debug!("Keyboard backlight not found: {err}"),
        }

        let properties = select_all(properties).map(|_| Change::Property);

        Ok(stream_select!(topology, properties).boxed())
    }

    async fn battery_changes(device: &DeviceProxy<'static>) -> [BoxStream<'static, ()>; 4] {
        [
            device
                .receive_state_changed()
                .await
                .skip(1)
                .map(|_| {})
                .boxed(),
            device
                .receive_percentage_changed()
                .await
                .skip(1)
                .map(|_| {})
                .boxed(),
            device
                .receive_time_to_full_changed()
                .await
                .skip(1)
                .throttle(TIME_ESTIMATE_THROTTLE)
                .map(|_| {})
                .boxed(),
            device
                .receive_time_to_empty_changed()
                .await
                .skip(1)
                .throttle(TIME_ESTIMATE_THROTTLE)
                .map(|_| {})
                .boxed(),
        ]
    }
}

// DBus exposes the keyboard backlight as a signed integer: clamp negative values to 0
fn safe_cast(value: i32) -> u32 {
    u32::try_from(value).unwrap_or_else(|error| {
        warn!("Received negative keyboard backlight value: {error}");
        0
    })
}

fn battery_status_from_system_state(state: DeviceState) -> BatteryStatus {
    match state {
        DeviceState::Charging => BatteryStatus::Charging(Duration::ZERO),
        DeviceState::Discharging => BatteryStatus::Discharging(Duration::ZERO),
        DeviceState::FullyCharged => BatteryStatus::Full,
        // PendingCharge/PendingDischarge are transitional, not active discharge.
        // Do not show time estimates for them.
        DeviceState::PendingCharge | DeviceState::PendingDischarge => BatteryStatus::NotCharging,
        DeviceState::Unknown | DeviceState::Empty => BatteryStatus::Unknown,
    }
}

fn battery_status_from_timed_system_state(state: DeviceState, time: i64) -> BatteryStatus {
    match state {
        DeviceState::Charging => {
            BatteryStatus::Charging(Duration::from_secs(time.try_into().unwrap_or_default()))
        }
        DeviceState::Discharging => {
            BatteryStatus::Discharging(Duration::from_secs(time.try_into().unwrap_or_default()))
        }
        _ => battery_status_from_system_state(state),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        BatteryStatus, DeviceState, battery_status_from_system_state,
        battery_status_from_timed_system_state,
    };
    use std::time::Duration;

    #[test]
    fn pending_charge_is_not_mapped_to_discharging() {
        assert_eq!(
            battery_status_from_system_state(DeviceState::PendingCharge),
            BatteryStatus::NotCharging
        );
    }

    #[test]
    fn pending_discharge_is_not_mapped_to_discharging() {
        assert_eq!(
            battery_status_from_system_state(DeviceState::PendingDischarge),
            BatteryStatus::NotCharging
        );
    }

    #[test]
    fn unknown_and_empty_map_to_unknown() {
        assert_eq!(
            battery_status_from_system_state(DeviceState::Unknown),
            BatteryStatus::Unknown
        );
        assert_eq!(
            battery_status_from_system_state(DeviceState::Empty),
            BatteryStatus::Unknown
        );
    }

    #[test]
    fn normal_system_battery_states_keep_time_semantics() {
        assert_eq!(
            battery_status_from_timed_system_state(DeviceState::Charging, 120),
            BatteryStatus::Charging(Duration::from_secs(120))
        );
        assert_eq!(
            battery_status_from_timed_system_state(DeviceState::Discharging, 240),
            BatteryStatus::Discharging(Duration::from_secs(240))
        );
        assert_eq!(
            battery_status_from_system_state(DeviceState::FullyCharged),
            BatteryStatus::Full
        );
    }
}
