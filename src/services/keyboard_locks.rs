use super::{ReadOnlyService, ServiceEvent};
use ashell_services::keyboard_locks::KeyboardLocks;
use iced::{
    Subscription,
    futures::{SinkExt, StreamExt},
    stream::channel,
};
use log::error;
use std::{any::TypeId, ops::Deref, pin::pin};

pub use ashell_services::keyboard_locks::KeyboardLocksData;

#[derive(Debug, Clone)]
pub struct KeyboardLocksService {
    data: KeyboardLocksData,
}

impl Deref for KeyboardLocksService {
    type Target = KeyboardLocksData;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl ReadOnlyService for KeyboardLocksService {
    type UpdateEvent = KeyboardLocksData;
    type Error = ();

    fn update(&mut self, event: Self::UpdateEvent) {
        self.data = event;
    }

    fn subscribe() -> Subscription<ServiceEvent<Self>> {
        Subscription::run_with(TypeId::of::<Self>(), |_| {
            channel(16, async |mut output| {
                let handle = match KeyboardLocks::connect().await {
                    Ok(handle) => handle,
                    Err(err) => {
                        error!("Failed to start the keyboard locks service: {err}");
                        return;
                    }
                };

                let mut updates = pin!(handle.updates());
                let Some(data) = updates.next().await else {
                    return;
                };
                let _ = output
                    .send(ServiceEvent::Init(KeyboardLocksService { data }))
                    .await;

                while let Some(data) = updates.next().await {
                    let _ = output.send(ServiceEvent::Update(data)).await;
                }
            })
        })
    }
}
