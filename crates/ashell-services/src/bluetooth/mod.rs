use crate::{rfkill, stream::channel};
use dbus::{BatteryProxy, BluetoothDbus, DeviceProxy};
use futures::{
    SinkExt, Stream, StreamExt,
    stream::{pending, select_all},
    stream_select,
};
use log::{error, info};
use std::pin::Pin;
use zbus::zvariant::OwnedObjectPath;

mod dbus;

type EventStream = Pin<Box<dyn Stream<Item = ()> + Send>>;

#[derive(PartialEq, Eq, Debug, Clone)]
pub enum BluetoothState {
    Unavailable,
    Active,
    Inactive,
}

#[derive(Debug, Clone)]
pub struct BluetoothDevice {
    pub name: String,
    pub battery: Option<u8>,
    pub path: OwnedObjectPath,
    pub connected: bool,
    pub paired: bool,
}

#[derive(Debug, Clone)]
pub struct BluetoothData {
    pub state: BluetoothState,
    pub devices: Vec<BluetoothDevice>,
    pub discovering: bool,
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

    pub fn updates(&self) -> impl Stream<Item = BluetoothData> + Send + 'static {
        let conn = self.conn.clone();

        channel(100, |mut output| async move {
            loop {
                info!("Listening for bluetooth events");

                let mut events = match Self::events(&conn).await {
                    Ok(events) => events,
                    Err(err) => {
                        error!("Failed to listen for bluetooth events: {err}");
                        return;
                    }
                };

                while events.next().await.is_some() {
                    if let Ok(data) = Self::read_data(&conn).await
                        && output.send(data).await.is_err()
                    {
                        return;
                    }
                }
            }
        })
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

    async fn read_data(conn: &zbus::Connection) -> anyhow::Result<BluetoothData> {
        let bluetooth = BluetoothDbus::new(conn).await?;

        let state = bluetooth.state().await?;
        let rfkill_soft_block = rfkill::bluetooth_soft_blocked().await?;

        let state = match state {
            BluetoothState::Unavailable => BluetoothState::Unavailable,
            BluetoothState::Active if rfkill_soft_block => BluetoothState::Inactive,
            state => state,
        };
        let devices = bluetooth.devices().await?;
        let discovering = bluetooth.discovering().await.unwrap_or(false);

        Ok(BluetoothData {
            state,
            devices,
            discovering,
        })
    }

    async fn events(conn: &zbus::Connection) -> anyhow::Result<impl Stream<Item = ()> + use<>> {
        let bluetooth = BluetoothDbus::new(conn).await?;

        let interface_changed = stream_select!(
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
        .boxed();

        let combined = match bluetooth.adapter.as_ref() {
            Some(adapter) => {
                let powered = adapter.receive_powered_changed().await.map(|_| {});
                let discovering = adapter.receive_discovering_changed().await.map(|_| {});
                let rfkill = rfkill::soft_block_changes().await?;
                let devices = bluetooth.devices().await?;

                let mut batteries: Vec<EventStream> = Vec::with_capacity(devices.len());
                let mut device_properties: Vec<EventStream> = Vec::with_capacity(devices.len());
                for device in devices {
                    let conn = bluetooth.bluez.inner().connection();

                    let battery = BatteryProxy::builder(conn)
                        .path(device.path.clone())?
                        .build()
                        .await?;
                    batteries.push(
                        battery
                            .receive_percentage_changed()
                            .await
                            .map(|_| {})
                            .boxed(),
                    );

                    let device_proxy = DeviceProxy::builder(conn)
                        .path(device.path)?
                        .build()
                        .await?;
                    let connected_changed: EventStream = device_proxy
                        .receive_connected_changed()
                        .await
                        .map(|_| {})
                        .boxed();
                    device_properties.push(connected_changed);
                }

                let battery_events = if batteries.is_empty() {
                    pending().boxed()
                } else {
                    select_all(batteries).boxed()
                };

                let device_property_events = if device_properties.is_empty() {
                    pending().boxed()
                } else {
                    select_all(device_properties).boxed()
                };

                Box::pin(stream_select!(
                    interface_changed,
                    powered,
                    discovering,
                    rfkill,
                    battery_events,
                    device_property_events,
                ))
            }
            _ => interface_changed,
        };

        Ok(combined)
    }
}
