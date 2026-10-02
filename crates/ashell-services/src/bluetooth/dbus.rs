use std::collections::HashMap;
use zbus::{
    proxy,
    proxy::CacheProperties,
    zvariant::{OwnedObjectPath, OwnedValue, Value},
};

use super::BluetoothDevice;

type Properties = HashMap<String, OwnedValue>;
type ManagedObjects = HashMap<OwnedObjectPath, HashMap<String, Properties>>;

pub(super) const ADAPTER: &str = "org.bluez.Adapter1";
pub(super) const DEVICE: &str = "org.bluez.Device1";
pub(super) const BATTERY: &str = "org.bluez.Battery1";

#[derive(Clone, Copy)]
pub(super) struct AdapterSnapshot {
    pub powered: bool,
    pub discovering: bool,
}

/// The adapter and devices bluez exposes, all from one `GetManagedObjects`
/// reply, so the snapshot is consistent and a device can't vanish halfway.
pub(super) struct Snapshot {
    pub adapter: Option<AdapterSnapshot>,
    pub devices: Vec<BluetoothDevice>,
}

pub(super) async fn snapshot(conn: &zbus::Connection) -> zbus::Result<Snapshot> {
    let objects = object_manager(conn).await?.get_managed_objects().await?;

    Ok(Snapshot::from_objects(&objects))
}

impl Snapshot {
    fn from_objects(objects: &ManagedObjects) -> Self {
        let adapter = objects
            .values()
            .find_map(|interfaces| interfaces.get(ADAPTER))
            .map(|adapter| AdapterSnapshot {
                powered: property(adapter, "Powered").unwrap_or(false),
                discovering: property(adapter, "Discovering").unwrap_or(false),
            });

        let mut devices: Vec<_> = objects
            .iter()
            .filter_map(|(path, interfaces)| {
                let device = interfaces.get(DEVICE)?;
                let connected = property(device, "Connected").unwrap_or(false);

                Some(BluetoothDevice {
                    name: property::<&str>(device, "Alias")
                        .unwrap_or_default()
                        .to_owned(),
                    battery: interfaces
                        .get(BATTERY)
                        .filter(|_| connected)
                        .and_then(|battery| property(battery, "Percentage")),
                    path: path.clone(),
                    connected,
                    paired: property(device, "Paired").unwrap_or(false),
                })
            })
            .collect();

        devices.sort_by(|a, b| a.name.cmp(&b.name));

        Self { adapter, devices }
    }
}

fn property<'a, T>(properties: &'a Properties, name: &str) -> Option<T>
where
    T: TryFrom<&'a Value<'a>>,
    <T as TryFrom<&'a Value<'a>>>::Error: Into<zbus::zvariant::Error>,
{
    properties.get(name)?.downcast_ref().ok()
}

/// bluez's object manager. Properties aren't cached: building it must not
/// call bluez, which would activate the service.
pub(super) async fn object_manager(
    conn: &zbus::Connection,
) -> zbus::Result<BluezObjectManagerProxy<'static>> {
    BluezObjectManagerProxy::builder(conn)
        .cache_properties(CacheProperties::No)
        .build()
        .await
}

/// The adapter and devices, for running commands on them.
pub struct BluetoothDbus<'a> {
    pub adapter: Option<AdapterProxy<'a>>,
    conn: zbus::Connection,
}

impl BluetoothDbus<'_> {
    pub async fn new(conn: &zbus::Connection) -> anyhow::Result<Self> {
        let adapter = object_manager(conn)
            .await?
            .get_managed_objects()
            .await?
            .into_iter()
            .find_map(|(path, interfaces)| interfaces.contains_key(ADAPTER).then_some(path));

        let adapter = match adapter {
            Some(path) => Some(AdapterProxy::builder(conn).path(path)?.build().await?),
            None => None,
        };

        Ok(Self {
            adapter,
            conn: conn.clone(),
        })
    }

    pub async fn set_powered(&self, value: bool) -> zbus::Result<()> {
        if let Some(adapter) = &self.adapter {
            adapter.set_powered(value).await?;
        }

        Ok(())
    }

    pub async fn start_discovery(&self) -> zbus::Result<()> {
        if let Some(adapter) = &self.adapter {
            adapter.start_discovery().await?;
        }
        Ok(())
    }

    pub async fn stop_discovery(&self) -> zbus::Result<()> {
        if let Some(adapter) = &self.adapter {
            adapter.stop_discovery().await?;
        }
        Ok(())
    }

    pub async fn pair_device(&self, device_path: &OwnedObjectPath) -> zbus::Result<()> {
        self.device(device_path).await?.pair().await
    }

    pub async fn connect_device(&self, device_path: &OwnedObjectPath) -> zbus::Result<()> {
        self.device(device_path).await?.connect().await
    }

    pub async fn disconnect_device(&self, device_path: &OwnedObjectPath) -> zbus::Result<()> {
        self.device(device_path).await?.disconnect().await
    }

    pub async fn remove_device(&self, device_path: &OwnedObjectPath) -> zbus::Result<()> {
        if let Some(adapter) = &self.adapter {
            adapter.remove_device(device_path.as_ref()).await?;
        }
        Ok(())
    }

    async fn device(&self, path: &OwnedObjectPath) -> zbus::Result<DeviceProxy<'static>> {
        DeviceProxy::builder(&self.conn)
            .path(path.clone())?
            .build()
            .await
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

    fn start_discovery(&self) -> zbus::Result<()>;

    fn stop_discovery(&self) -> zbus::Result<()>;

    #[zbus(property)]
    fn discovering(&self) -> zbus::Result<bool>;

    fn remove_device(&self, device: zbus::zvariant::ObjectPath<'_>) -> zbus::Result<()>;
}

#[proxy(default_service = "org.bluez", interface = "org.bluez.Device1")]
pub trait Device {
    #[zbus(property)]
    fn alias(&self) -> zbus::Result<String>;

    #[zbus(property)]
    fn connected(&self) -> zbus::Result<bool>;

    #[zbus(property)]
    fn paired(&self) -> zbus::Result<bool>;

    fn pair(&self) -> zbus::Result<()>;

    fn connect(&self) -> zbus::Result<()>;

    fn disconnect(&self) -> zbus::Result<()>;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn properties(values: Vec<(&str, Value<'static>)>) -> Properties {
        values
            .into_iter()
            .map(|(name, value)| (name.to_owned(), value.try_into().unwrap()))
            .collect()
    }

    fn path(path: &str) -> OwnedObjectPath {
        OwnedObjectPath::try_from(path).unwrap()
    }

    fn device(alias: &str, connected: bool, battery: Option<u8>) -> HashMap<String, Properties> {
        let mut interfaces = HashMap::from([(
            DEVICE.to_owned(),
            properties(vec![
                ("Alias", Value::from(alias.to_owned())),
                ("Connected", Value::from(connected)),
                ("Paired", Value::from(true)),
            ]),
        )]);
        if let Some(percentage) = battery {
            interfaces.insert(
                BATTERY.to_owned(),
                properties(vec![("Percentage", Value::from(percentage))]),
            );
        }
        interfaces
    }

    #[test]
    fn reads_adapter_and_sorts_devices() {
        let objects = ManagedObjects::from([
            (
                path("/org/bluez/hci0"),
                HashMap::from([(
                    ADAPTER.to_owned(),
                    properties(vec![
                        ("Powered", Value::from(true)),
                        ("Discovering", Value::from(false)),
                    ]),
                )]),
            ),
            (
                path("/org/bluez/hci0/dev_B"),
                device("Mouse", true, Some(80)),
            ),
            (
                path("/org/bluez/hci0/dev_A"),
                device("Headphones", false, Some(50)),
            ),
        ]);

        let snapshot = Snapshot::from_objects(&objects);

        let adapter = snapshot.adapter.unwrap();
        assert!(adapter.powered);
        assert!(!adapter.discovering);
        let names: Vec<_> = snapshot.devices.iter().map(|d| d.name.as_str()).collect();
        assert_eq!(names, ["Headphones", "Mouse"]);
        assert_eq!(snapshot.devices[1].battery, Some(80));
    }

    #[test]
    fn battery_only_for_connected_devices() {
        let objects = ManagedObjects::from([(
            path("/org/bluez/hci0/dev_A"),
            device("Headphones", false, Some(50)),
        )]);

        assert_eq!(Snapshot::from_objects(&objects).devices[0].battery, None);
    }

    #[test]
    fn missing_adapter_and_properties() {
        let objects = ManagedObjects::from([(
            path("/org/bluez/hci0/dev_A"),
            HashMap::from([(DEVICE.to_owned(), Properties::new())]),
        )]);

        let snapshot = Snapshot::from_objects(&objects);

        assert!(snapshot.adapter.is_none());
        let device = &snapshot.devices[0];
        assert_eq!(device.name, "");
        assert!(!device.connected && !device.paired);
    }
}
