use std::ops::Deref;
use zbus::{
    Result, proxy,
    zvariant::{ObjectPath, OwnedObjectPath},
};

pub struct UPowerDbus<'a>(UPowerProxy<'a>);

impl<'a> Deref for UPowerDbus<'a> {
    type Target = UPowerProxy<'a>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl UPowerDbus<'_> {
    pub async fn new(conn: &zbus::Connection) -> anyhow::Result<Self> {
        let nm = UPowerProxy::new(conn).await?;

        Ok(Self(nm))
    }

    pub async fn get_battery_device(&self) -> anyhow::Result<Option<DeviceProxy>> {
        let devices = self.enumerate_devices().await?;

        for device in devices {
            let device = DeviceProxy::builder(self.inner().connection())
                .path(device)?
                .build()
                .await?;

            let device_type = device.device_type().await?;
            let power_supply = device.power_supply().await?;

            if device_type == 2 && power_supply {
                return Ok(Some(device));
            }
        }

        Ok(None)
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
}

#[proxy(
    interface = "org.freedesktop.UPower",
    default_service = "org.freedesktop.UPower",
    default_path = "/org/freedesktop/UPower"
)]
pub trait UPower {
    fn enumerate_devices(&self) -> Result<Vec<OwnedObjectPath>>;

    #[zbus(signal)]
    fn device_added(&self) -> Result<OwnedObjectPath>;
}

#[proxy(
    default_service = "org.freedesktop.UPower",
    default_path = "/org/freedesktop/UPower/Device",
    interface = "org.freedesktop.UPower.Device"
)]
pub trait Device {
    #[zbus(property, name = "Type")]
    fn device_type(&self) -> Result<u32>;

    #[zbus(property)]
    fn power_supply(&self) -> Result<bool>;

    #[zbus(property)]
    fn time_to_empty(&self) -> Result<i64>;

    #[zbus(property)]
    fn time_to_full(&self) -> Result<i64>;

    #[zbus(property)]
    fn percentage(&self) -> Result<f64>;

    #[zbus(property)]
    fn state(&self) -> Result<u32>;
}

#[proxy(
    default_service = "org.freedesktop.UPower.PowerProfiles",
    default_path = "/org/freedesktop/UPower/PowerProfiles",
    interface = "org.freedesktop.UPower.PowerProfiles"
)]
pub trait PowerProfiles {
    #[zbus(property)]
    fn active_profile(&self) -> Result<String>;

    #[zbus(property)]
    fn set_active_profile(&self, profile: &str) -> Result<()>;
}
