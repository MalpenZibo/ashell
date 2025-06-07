use std::collections::HashMap;

use zbus::{
    proxy,
    zvariant::{OwnedObjectPath, OwnedValue},
};

use super::{BluetoothDevice, BluetoothState};

type ManagedObjects = HashMap<OwnedObjectPath, HashMap<String, HashMap<String, OwnedValue>>>;

pub struct BluetoothDbus<'a> {
    pub bluez: BluezObjectManagerProxy<'a>,
    pub adapter: Option<AdapterProxy<'a>>,
}

impl BluetoothDbus<'_> {
    pub async fn new(conn: &zbus::Connection) -> anyhow::Result<Self> {
        let bluez = BluezObjectManagerProxy::new(conn).await?;
        let adapter = bluez
            .get_managed_objects()
            .await?
            .into_iter()
            .filter_map(|(key, item)| {
                if item.contains_key("org.bluez.Adapter1") {
                    Some(key)
                } else {
                    None
                }
            })
            .next();

        let adapter = if let Some(adapter) = adapter {
            Some(AdapterProxy::builder(conn).path(adapter)?.build().await?)
        } else {
            None
        };

        Ok(Self { bluez, adapter })
    }

    pub async fn set_powered(&self, value: bool) -> zbus::Result<()> {
        if let Some(adapter) = &self.adapter {
            adapter.set_powered(value).await?;
        }

        Ok(())
    }

    pub async fn state(&self) -> zbus::Result<BluetoothState> {
        match &self.adapter {
            Some(adapter) => {
                if adapter.powered().await? {
                    Ok(BluetoothState::Active)
                } else {
                    Ok(BluetoothState::Inactive)
                }
            }
            _ => Ok(BluetoothState::Unavailable),
        }
    }

    pub async fn devices(&self) -> anyhow::Result<Vec<BluetoothDevice>> {
        let devices_proxy = self
            .bluez
            .get_managed_objects()
            .await?
            .into_iter()
            .filter_map(|(key, item)| {
                if item.contains_key("org.bluez.Device1") {
                    Some((key.clone(), item.contains_key("org.bluez.Battery1")))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        let mut devices = Vec::new();
        for (device_path, has_battery) in devices_proxy {
            let device = DeviceProxy::builder(self.bluez.inner().connection())
                .path(device_path.clone())?
                .build()
                .await?;

            let name = device.alias().await?;
            let connected = device.connected().await?;

            if connected {
                let battery = if has_battery {
                    let battery_proxy = BatteryProxy::builder(self.bluez.inner().connection())
                        .path(&device_path)?
                        .build()
                        .await?;

                    Some(battery_proxy.percentage().await?)
                } else {
                    None
                };

                devices.push(BluetoothDevice {
                    name,
                    battery,
                    path: device_path,
                });
            }
        }

        Ok(devices)
    }
}

#[proxy(
    default_service = "org.bluez",
    default_path = "/",
    interface = "org.freedesktop.DBus.ObjectManager"
)]
pub trait BluezObjectManager {
    fn get_managed_objects(&self) -> zbus::Result<ManagedObjects>;

    #[zbus(signal)]
    fn interfaces_added(&self) -> Result<()>;

    #[zbus(signal)]
    fn interfaces_removed(&self) -> Result<()>;
}

#[proxy(
    default_service = "org.bluez",
    default_path = "/org/bluez/hci0",
    interface = "org.bluez.Adapter1"
)]
pub trait Adapter {
    #[zbus(property)]
    fn powered(&self) -> zbus::Result<bool>;

    #[zbus(property)]
    fn set_powered(&self, value: bool) -> zbus::Result<()>;
}

#[proxy(default_service = "org.bluez", interface = "org.bluez.Device1")]
trait Device {
    #[zbus(property)]
    fn alias(&self) -> zbus::Result<String>;

    #[zbus(property)]
    fn connected(&self) -> zbus::Result<bool>;
}

#[proxy(default_service = "org.bluez", interface = "org.bluez.Battery1")]
pub trait Battery {
    #[zbus(property)]
    fn percentage(&self) -> zbus::Result<u8>;
}
