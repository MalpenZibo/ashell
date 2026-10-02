use super::{ReadOnlyService, Service, ServiceEvent};
use ashell_services::bluetooth::{Bluetooth, BluetoothData};
use iced::{
    Subscription, Task,
    futures::{SinkExt, StreamExt},
    stream::channel,
};
use log::{error, info, warn};
use std::{any::TypeId, ops::Deref, pin::pin};

pub use ashell_services::bluetooth::{BluetoothCommand, BluetoothDevice, BluetoothState};

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

                let mut updates = pin!(handle.updates());
                let Some(data) = updates.next().await else {
                    return;
                };
                info!("Bluetooth service initialized");
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

        Task::future(async move {
            if let Err(err) = handle.execute(command).await {
                warn!("Bluetooth command failed: {err}");
            }
        })
        .discard()
    }
}
