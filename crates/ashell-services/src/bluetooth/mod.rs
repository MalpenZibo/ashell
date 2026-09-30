use crate::{rfkill, stream::channel};
use dbus::{BatteryProxy, BluetoothDbus, DeviceProxy};
use futures::{
    SinkExt, Stream, StreamExt,
    stream::{BoxStream, select_all},
    stream_select,
};
use log::{debug, error, info, warn};
use std::time::Duration;
use zbus::zvariant::OwnedObjectPath;

mod dbus;

const DISCOVERY_DURATION: Duration = Duration::from_secs(15);

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

    pub async fn data(&self) -> anyhow::Result<BluetoothData> {
        Self::read_data(&self.conn).await
    }

    /// Yields the current state once subscribed, then a fresh snapshot on every change.
    pub fn updates(&self) -> impl Stream<Item = BluetoothData> + Send + 'static {
        let conn = self.conn.clone();

        channel(100, |mut output| async move {
            loop {
                info!("Listening for bluetooth events");

                let mut changes = match Self::changes(&conn).await {
                    Ok(changes) => changes,
                    Err(err) => {
                        error!("Failed to listen for bluetooth events: {err}");
                        return;
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

        match command {
            BluetoothCommand::Toggle => {
                match Self::read_state(&BluetoothDbus::new(&self.conn).await?).await? {
                    BluetoothState::Unavailable => Ok(()),
                    BluetoothState::Active => self.set_powered(false).await,
                    BluetoothState::Inactive => self.set_powered(true).await,
                }
            }
            BluetoothCommand::StartDiscovery => {
                self.start_discovery().await?;
                tokio::time::sleep(DISCOVERY_DURATION).await;
                self.stop_discovery().await
            }
            BluetoothCommand::PairDevice(device) => self.pair_device(&device).await,
            BluetoothCommand::ConnectDevice(device) => self.connect_device(&device).await,
            BluetoothCommand::DisconnectDevice(device) => self.disconnect_device(&device).await,
            BluetoothCommand::RemoveDevice(device) => self.remove_device(&device).await,
        }
    }

    pub async fn set_powered(&self, powered: bool) -> anyhow::Result<()> {
        BluetoothDbus::new(&self.conn)
            .await?
            .set_powered(powered)
            .await?;

        Ok(())
    }

    pub async fn start_discovery(&self) -> anyhow::Result<()> {
        BluetoothDbus::new(&self.conn)
            .await?
            .start_discovery()
            .await?;

        Ok(())
    }

    pub async fn stop_discovery(&self) -> anyhow::Result<()> {
        BluetoothDbus::new(&self.conn)
            .await?
            .stop_discovery()
            .await?;

        Ok(())
    }

    pub async fn pair_device(&self, device: &OwnedObjectPath) -> anyhow::Result<()> {
        BluetoothDbus::new(&self.conn)
            .await?
            .pair_device(device)
            .await?;

        Ok(())
    }

    pub async fn connect_device(&self, device: &OwnedObjectPath) -> anyhow::Result<()> {
        BluetoothDbus::new(&self.conn)
            .await?
            .connect_device(device)
            .await?;

        Ok(())
    }

    pub async fn disconnect_device(&self, device: &OwnedObjectPath) -> anyhow::Result<()> {
        BluetoothDbus::new(&self.conn)
            .await?
            .disconnect_device(device)
            .await?;

        Ok(())
    }

    pub async fn remove_device(&self, device: &OwnedObjectPath) -> anyhow::Result<()> {
        BluetoothDbus::new(&self.conn)
            .await?
            .remove_device(device)
            .await?;

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

        let conn = bluetooth.bluez.inner().connection();
        for device in bluetooth.devices().await? {
            let device_proxy = DeviceProxy::builder(conn)
                .path(device.path.clone())?
                .build()
                .await?;
            properties.push(
                device_proxy
                    .receive_connected_changed()
                    .await
                    .skip(1)
                    .map(|_| {})
                    .boxed(),
            );
            properties.push(
                device_proxy
                    .receive_paired_changed()
                    .await
                    .skip(1)
                    .map(|_| {})
                    .boxed(),
            );
            properties.push(
                device_proxy
                    .receive_alias_changed()
                    .await
                    .skip(1)
                    .map(|_| {})
                    .boxed(),
            );

            let battery = BatteryProxy::builder(conn)
                .path(device.path)?
                .build()
                .await?;
            properties.push(
                battery
                    .receive_percentage_changed()
                    .await
                    .skip(1)
                    .map(|_| {})
                    .boxed(),
            );
        }

        let properties = select_all(properties).map(|_| Change::Property);

        Ok(stream_select!(topology, properties).boxed())
    }
}
