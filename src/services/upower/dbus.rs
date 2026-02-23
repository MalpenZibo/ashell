use log::debug;
use std::ops::Deref;
use zbus::{
    proxy,
    zvariant::{ObjectPath, OwnedObjectPath},
};

pub struct UPowerDbus<'a>(UPowerProxy<'a>);

impl<'a> Deref for UPowerDbus<'a> {
    type Target = UPowerProxy<'a>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Default)]
pub struct SystemBattery(Vec<DeviceProxy<'static>>);

impl SystemBattery {
    pub async fn state(&self) -> DeviceState {
        let mut charging = false;
        let mut discharging = false;
        let mut fully_charged_count = 0;
        let mut total_devices = 0;

        for device in &self.0 {
            if let Ok(energy_full) = device.energy_full().await
                && energy_full == 0.0
            {
                continue;
            }

            if let Ok(state_raw) = device.state().await
                && let Ok(state) = DeviceState::try_from(state_raw)
            {
                total_devices += 1;
                match state {
                    DeviceState::Charging => charging = true,
                    DeviceState::Discharging => discharging = true,
                    DeviceState::FullyCharged => fully_charged_count += 1,
                    _ => {}
                }
            }
        }

        if total_devices == 0 {
            return DeviceState::Unknown;
        }

        if fully_charged_count == total_devices {
            DeviceState::FullyCharged
        } else if charging {
            DeviceState::Charging
        } else if discharging {
            DeviceState::Discharging
        } else {
            DeviceState::Unknown
        }
    }

    pub async fn percentage(&self) -> anyhow::Result<f64> {
        let mut energy = 0.0;
        let mut energy_full = 0.0;

        for device in &self.0 {
            energy += device.energy().await.unwrap_or(0.0);
            energy_full += device.energy_full().await.unwrap_or(0.0);
        }

        if energy_full == 0.0 {
            anyhow::bail!("No battery capacity data available");
        }

        Ok(energy / energy_full * 100.0)
    }

    pub async fn time_to_empty(&self) -> i64 {
        let mut time = 0;
        for device in &self.0 {
            if let Ok(t) = device.time_to_empty().await {
                time += t;
            }
        }
        time
    }

    pub async fn time_to_full(&self) -> i64 {
        let mut time = 0;
        for device in &self.0 {
            if let Ok(t) = device.time_to_full().await {
                time += t;
            }
        }
        time
    }

    pub fn get_devices_path(self) -> Vec<String> {
        self.0
            .into_iter()
            .map(|device| device.inner().path().to_string())
            .collect()
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum DeviceState {
    #[default]
    Unknown = 0,
    Charging = 1,
    Discharging = 2,
    Empty = 3,
    FullyCharged = 4,
    PendingCharge = 5,
    PendingDischarge = 6,
}

impl TryFrom<u32> for DeviceState {
    type Error = ();
    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Unknown),
            1 => Ok(Self::Charging),
            2 => Ok(Self::Discharging),
            3 => Ok(Self::Empty),
            4 => Ok(Self::FullyCharged),
            5 => Ok(Self::PendingCharge),
            6 => Ok(Self::PendingDischarge),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum UpDeviceKind {
    #[default]
    Unknown = 0,
    LinePower = 1,
    Battery = 2,
    Ups = 3,
    Monitor = 4,
    Mouse = 5,
    Keyboard = 6,
    Pda = 7,
    Phone = 8,
    MediaPlayer = 9,
    Tablet = 10,
    Computer = 11,
    GamingInput = 12,
    Pen = 13,
    Touchpad = 14,
    Headset = 17,
    Speakers = 18,
    Headphones = 19,
}

impl UpDeviceKind {
    pub fn from_u32(value: u32) -> Option<Self> {
        match value {
            0 => Some(Self::Unknown),
            1 => Some(Self::LinePower),
            2 => Some(Self::Battery),
            5 => Some(Self::Mouse),
            6 => Some(Self::Keyboard),
            12 => Some(Self::GamingInput),
            17 => Some(Self::Headset),
            19 => Some(Self::Headphones),
            _ => None,
        }
    }

    pub fn is_peripheral(&self) -> bool {
        matches!(
            self,
            Self::Mouse | Self::Keyboard | Self::GamingInput | Self::Headset | Self::Headphones
        )
    }

    pub fn is_power_source(&self) -> bool {
        matches!(self, Self::Battery)
    }
}

impl UPowerDbus<'_> {
    pub async fn new(conn: &zbus::Connection) -> anyhow::Result<Self> {
        let nm = UPowerProxy::new(conn).await?;
        Ok(Self(nm))
    }

    pub async fn get_system_batteries(&self) -> anyhow::Result<Option<SystemBattery>> {
        self.get_battery_devices(|device_type, power_supply| {
            device_type.is_power_source() && power_supply
        })
        .await
        .map(|devices| {
            if !devices.is_empty() {
                Some(SystemBattery(devices))
            } else {
                None
            }
        })
    }

    pub async fn get_peripheral_batteries(&self) -> anyhow::Result<Vec<DeviceProxy<'static>>> {
        self.get_battery_devices(|device_type, power_supply| {
            device_type.is_peripheral() && !power_supply
        })
        .await
    }

    pub async fn get_device(
        &self,
        path: &ObjectPath<'static>,
    ) -> anyhow::Result<DeviceProxy<'static>> {
        let device = DeviceProxy::builder(self.inner().connection())
            .path(path)?
            .build()
            .await?;
        Ok(device)
    }

    async fn get_battery_devices(
        &self,
        f: fn(UpDeviceKind, bool) -> bool,
    ) -> anyhow::Result<Vec<DeviceProxy<'static>>> {
        let devices = self.enumerate_devices().await?;
        debug!("Found {} devices", devices.len());

        let mut res = Vec::new();
        for device in devices {
            let device = DeviceProxy::builder(self.inner().connection())
                .path(device)?
                .build()
                .await?;

            let device_type = UpDeviceKind::from_u32(device.device_type().await?)
                .unwrap_or(UpDeviceKind::Unknown);

            let power_supply = device.power_supply().await?;

            debug!(
                "Device: {}, Type: {:?}, Power Supply: {}",
                device.inner().path(),
                device_type,
                power_supply
            );

            if f(device_type, power_supply) {
                res.push(device);
            }
        }
        Ok(res)
    }
}

#[proxy(
    interface = "org.freedesktop.UPower",
    default_service = "org.freedesktop.UPower",
    default_path = "/org/freedesktop/UPower"
)]
pub trait UPower {
    fn enumerate_devices(&self) -> zbus::Result<Vec<OwnedObjectPath>>;

    #[zbus(signal)]
    fn device_added(&self) -> zbus::Result<OwnedObjectPath>;

    #[zbus(signal)]
    fn device_removed(&self) -> zbus::Result<OwnedObjectPath>;
}

#[proxy(
    default_service = "org.freedesktop.UPower",
    default_path = "/org/freedesktop/UPower/Device",
    interface = "org.freedesktop.UPower.Device"
)]
pub trait Device {
    #[zbus(property, name = "Type")]
    fn device_type(&self) -> zbus::Result<u32>;

    #[zbus(property)]
    fn power_supply(&self) -> zbus::Result<bool>;

    #[zbus(property)]
    fn time_to_empty(&self) -> zbus::Result<i64>;

    #[zbus(property)]
    fn time_to_full(&self) -> zbus::Result<i64>;

    #[zbus(property)]
    fn percentage(&self) -> zbus::Result<f64>;

    #[zbus(property)]
    fn energy(&self) -> zbus::Result<f64>;

    #[zbus(property)]
    fn energy_full(&self) -> zbus::Result<f64>;

    #[zbus(property)]
    fn state(&self) -> zbus::Result<u32>;

    #[zbus(property, name = "Model")]
    fn model(&self) -> zbus::Result<String>;
}

#[proxy(
    default_service = "org.freedesktop.UPower.PowerProfiles",
    default_path = "/org/freedesktop/UPower/PowerProfiles",
    interface = "org.freedesktop.UPower.PowerProfiles"
)]
pub trait PowerProfiles {
    #[zbus(property)]
    fn active_profile(&self) -> zbus::Result<String>;

    #[zbus(property)]
    fn set_active_profile(&self, profile: &str) -> zbus::Result<()>;
}
