use super::{ReadOnlyService, Service, ServiceEvent};
use ashell_services::bluetooth::Bluetooth;
use iced::{
    Subscription, Task,
    futures::{SinkExt, StreamExt},
    stream::channel,
};
use log::{debug, error, info};
use std::{any::TypeId, ops::Deref, pin::pin, time::Duration};
use zbus::zvariant::OwnedObjectPath;

pub use ashell_services::bluetooth::{BluetoothData, BluetoothDevice, BluetoothState};

#[derive(Debug, Clone)]
pub struct BluetoothService {
    handle: Bluetooth,
    data: BluetoothData,
}

impl Deref for BluetoothService {
    type Target = BluetoothData;

    fn deref(&self) -> &Self::Target {
        &self.data
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

async fn refresh(handle: &Bluetooth) -> BluetoothData {
    handle.data().await.unwrap_or_else(|_| BluetoothData {
        state: BluetoothState::Unavailable,
        devices: vec![],
        discovering: false,
    })
}

impl ReadOnlyService for BluetoothService {
    type UpdateEvent = BluetoothData;
    type Error = ();

    fn update(&mut self, event: Self::UpdateEvent) {
        self.data = event;
    }

    fn subscribe() -> Subscription<ServiceEvent<Self>> {
        Subscription::run_with(TypeId::of::<Self>(), |_| {
            channel(100, async |mut output| {
                let handle = match Bluetooth::connect().await {
                    Ok(handle) => handle,
                    Err(err) => {
                        error!("Failed to connect to system bus: {err}");
                        return;
                    }
                };
                let data = match handle.data().await {
                    Ok(data) => data,
                    Err(err) => {
                        error!("Failed to initialize bluetooth service: {err}");
                        return;
                    }
                };
                info!("Bluetooth service initialized");

                let mut updates = pin!(handle.updates());
                let _ = output
                    .send(ServiceEvent::Init(BluetoothService { handle, data }))
                    .await;

                while let Some(data) = updates.next().await {
                    let _ = output.send(ServiceEvent::Update(data)).await;
                }
            })
        })
    }
}

impl Service for BluetoothService {
    type Command = BluetoothCommand;

    fn command(&mut self, command: Self::Command) -> Task<ServiceEvent<Self>> {
        let handle = self.handle.clone();

        match command {
            BluetoothCommand::Toggle => {
                if self.data.state == BluetoothState::Unavailable {
                    return Task::none();
                }

                let mut data = self.data.clone();
                Task::perform(
                    async move {
                        let powered = data.state == BluetoothState::Active;
                        debug!("Toggling bluetooth power to: {}", !powered);

                        if handle.set_powered(!powered).await.is_ok() {
                            data.state = if powered {
                                BluetoothState::Inactive
                            } else {
                                BluetoothState::Active
                            };
                        }

                        data
                    },
                    ServiceEvent::Update,
                )
            }
            BluetoothCommand::StartDiscovery => Task::perform(
                async move {
                    if handle.start_discovery().await.is_ok() {
                        tokio::time::sleep(Duration::from_secs(15)).await;
                        let _ = handle.stop_discovery().await;
                    }
                    refresh(&handle).await
                },
                ServiceEvent::Update,
            ),
            BluetoothCommand::PairDevice(device) => Task::perform(
                async move {
                    debug!("Pairing device: {device:?}");
                    let _ = handle.pair_device(&device).await;
                    refresh(&handle).await
                },
                ServiceEvent::Update,
            ),
            BluetoothCommand::ConnectDevice(device) => Task::perform(
                async move {
                    debug!("Connecting device: {device:?}");
                    let _ = handle.connect_device(&device).await;
                    refresh(&handle).await
                },
                ServiceEvent::Update,
            ),
            BluetoothCommand::DisconnectDevice(device) => Task::perform(
                async move {
                    debug!("Disconnecting device: {device:?}");
                    let _ = handle.disconnect_device(&device).await;
                    refresh(&handle).await
                },
                ServiceEvent::Update,
            ),
            BluetoothCommand::RemoveDevice(device) => Task::perform(
                async move {
                    debug!("Removing device: {device:?}");
                    let _ = handle.remove_device(&device).await;
                    refresh(&handle).await
                },
                ServiceEvent::Update,
            ),
        }
    }
}
