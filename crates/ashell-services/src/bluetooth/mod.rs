use crate::{bus, rfkill, stream::channel};
use dbus::{BatteryProxy, BluetoothDbus, DeviceProxy};
use futures::{
    SinkExt, Stream, StreamExt,
    stream::{BoxStream, select_all},
    stream_select,
};
use log::{debug, error, warn};
use std::time::Duration;
use zbus::zvariant::OwnedObjectPath;

mod dbus;

const BLUEZ: &str = "org.bluez";
const DISCOVERY_DURATION: Duration = Duration::from_secs(15);
const RETRY_DELAY: Duration = Duration::from_secs(5);

#[derive(PartialEq, Eq, Debug, Clone)]
pub enum BluetoothState {
    Unavailable,
    Active,
    Inactive,
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct BluetoothDevice {
    pub name: String,
    pub battery: Option<u8>,
    pub path: OwnedObjectPath,
    pub connected: bool,
    pub paired: bool,
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct BluetoothData {
    pub state: BluetoothState,
    pub devices: Vec<BluetoothDevice>,
    pub discovering: bool,
}

impl Default for BluetoothData {
    fn default() -> Self {
        Self {
            state: BluetoothState::Unavailable,
            devices: Vec::new(),
            discovering: false,
        }
    }
}

#[derive(Debug, Clone)]
pub enum BluetoothCommand {
    Toggle,
    StartDiscovery,
    PairDevice(OwnedObjectPath),
    ConnectDevice(OwnedObjectPath),
    DisconnectDevice(OwnedObjectPath),
    RemoveDevice(OwnedObjectPath),
}

#[derive(PartialEq, Eq)]
enum Change {
    Topology,
    Property,
}

#[derive(Debug, Clone)]
pub struct Bluetooth {
    conn: zbus::Connection,
}

impl Bluetooth {
    pub async fn connect() -> anyhow::Result<Self> {
        let conn = zbus::Connection::system().await?;

        Ok(Self { conn })
    }

    /// Yields the current state once subscribed, then a fresh snapshot on every change.
    pub fn updates(&self) -> impl Stream<Item = BluetoothData> + Send + 'static {
        let conn = self.conn.clone();

        channel(100, |mut output| async move {
            loop {
                match bus::has_owner(&conn, BLUEZ).await {
                    Ok(true) => {}
                    Ok(false) => {
                        if output.send(BluetoothData::default()).await.is_err() {
                            return;
                        }
                        if let Err(err) = bus::owner_acquired(&conn, BLUEZ).await {
                            error!("Failed to watch for bluez: {err}");
                            return;
                        }
                        continue;
                    }
                    Err(err) => {
                        error!("Failed to look up bluez on the system bus: {err}");
                        return;
                    }
                }

                debug!("Listening for bluetooth events");

                let mut changes = match Self::changes(&conn).await {
                    Ok(changes) => changes,
                    Err(err) => {
                        warn!("Failed to listen for bluetooth events: {err}");
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
                        Err(err) => warn!("Failed to read bluetooth data: {err}"),
                    }

                    match changes.next().await {
                        Some(Change::Property) => {}
                        Some(Change::Topology) | None => break,
                    }
                }
            }
        })
    }

    pub async fn execute(&self, command: BluetoothCommand) -> anyhow::Result<()> {
        debug!("Bluetooth command: {command:?}");

        let bluetooth = BluetoothDbus::new(&self.conn).await?;

        match command {
            BluetoothCommand::Toggle => match Self::read_state(&bluetooth).await? {
                BluetoothState::Unavailable => {}
                BluetoothState::Active => bluetooth.set_powered(false).await?,
                BluetoothState::Inactive => bluetooth.set_powered(true).await?,
            },
            BluetoothCommand::StartDiscovery => {
                bluetooth.start_discovery().await?;
                tokio::time::sleep(DISCOVERY_DURATION).await;
                bluetooth.stop_discovery().await?;
            }
            BluetoothCommand::PairDevice(device) => bluetooth.pair_device(&device).await?,
            BluetoothCommand::ConnectDevice(device) => bluetooth.connect_device(&device).await?,
            BluetoothCommand::DisconnectDevice(device) => {
                bluetooth.disconnect_device(&device).await?
            }
            BluetoothCommand::RemoveDevice(device) => bluetooth.remove_device(&device).await?,
        }

        Ok(())
    }

    async fn read_state(bluetooth: &BluetoothDbus<'_>) -> anyhow::Result<BluetoothState> {
        let state = bluetooth.state().await?;

        Ok(match state {
            BluetoothState::Active if rfkill::bluetooth_soft_blocked().await? => {
                BluetoothState::Inactive
            }
            state => state,
        })
    }

    async fn read_data(conn: &zbus::Connection) -> anyhow::Result<BluetoothData> {
        let bluetooth = BluetoothDbus::new(conn).await?;

        let state = Self::read_state(&bluetooth).await?;
        let devices = bluetooth.devices().await?;
        let discovering = bluetooth.discovering().await.unwrap_or(false);

        Ok(BluetoothData {
            state,
            devices,
            discovering,
        })
    }

    async fn changes(conn: &zbus::Connection) -> anyhow::Result<BoxStream<'static, Change>> {
        let bluetooth = BluetoothDbus::new(conn).await?;

        let topology = stream_select!(
            bluetooth
                .bluez
                .receive_interfaces_added()
                .await?
                .map(|_| {}),
            bluetooth
                .bluez
                .receive_interfaces_removed()
                .await?
                .map(|_| {}),
            bus::owner_changes(conn, BLUEZ).await?,
        )
        .map(|_| Change::Topology);

        let Some(adapter) = bluetooth.adapter.as_ref() else {
            return Ok(topology.boxed());
        };

        // Property streams yield their current value first: skip it, updates() reads a snapshot
        let mut properties: Vec<BoxStream<'static, ()>> = vec![
            adapter
                .receive_powered_changed()
                .await
                .skip(1)
                .map(|_| {})
                .boxed(),
            adapter
                .receive_discovering_changed()
                .await
                .skip(1)
                .map(|_| {})
                .boxed(),
            rfkill::soft_block_changes().await?.boxed(),
        ];

        for (path, has_battery) in bluetooth.managed_devices().await? {
            // A device can vanish while the stream is built: skip it, its
            // InterfacesRemoved triggers a rebuild anyway
            match Self::device_changes(conn, &path, has_battery).await {
                Ok(changes) => properties.extend(changes),
                Err(err) => debug!("Not watching bluetooth device {path}: {err}"),
            }
        }

        let properties = select_all(properties).map(|_| Change::Property);

        Ok(stream_select!(topology, properties).boxed())
    }

    async fn device_changes(
        conn: &zbus::Connection,
        path: &OwnedObjectPath,
        has_battery: bool,
    ) -> zbus::Result<Vec<BoxStream<'static, ()>>> {
        let device = DeviceProxy::builder(conn)
            .path(path.clone())?
            .build()
            .await?;

        let mut changes = vec![
            device
                .receive_connected_changed()
                .await
                .skip(1)
                .map(|_| {})
                .boxed(),
            device
                .receive_paired_changed()
                .await
                .skip(1)
                .map(|_| {})
                .boxed(),
            device
                .receive_alias_changed()
                .await
                .skip(1)
                .map(|_| {})
                .boxed(),
        ];

        if has_battery {
            let battery = BatteryProxy::builder(conn)
                .path(path.clone())?
                .build()
                .await?;
            changes.push(
                battery
                    .receive_percentage_changed()
                    .await
                    .skip(1)
                    .map(|_| {})
                    .boxed(),
            );
        }

        Ok(changes)
    }
}
