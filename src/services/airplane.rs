use super::{ReadOnlyService, Service, ServiceEvent};
use crate::components::icons::StaticIcon;
use ashell_services::airplane::Airplane;
use iced::{
    Subscription, Task,
    futures::{SinkExt, StreamExt},
    stream::channel,
};
use log::{error, warn};
use std::{any::TypeId, ops::Deref, pin::pin};

pub use ashell_services::airplane::{AirplaneCommand, AirplaneData};

#[derive(Debug, Clone)]
pub struct AirplaneService {
    handle: Airplane,
    data: AirplaneData,
}

impl Deref for AirplaneService {
    type Target = AirplaneData;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl ReadOnlyService for AirplaneService {
    type UpdateEvent = AirplaneData;
    type Error = ();

    fn update(&mut self, event: Self::UpdateEvent) {
        self.data = event;
    }

    fn subscribe() -> Subscription<ServiceEvent<Self>> {
        Subscription::run_with(TypeId::of::<Self>(), |_| {
            channel(100, async |mut output| {
                let handle = match Airplane::connect().await {
                    Ok(handle) => handle,
                    Err(err) => {
                        error!("Failed to start the airplane mode service: {err}");
                        return;
                    }
                };

                let mut updates = pin!(handle.updates());
                let Some(data) = updates.next().await else {
                    return;
                };
                let _ = output
                    .send(ServiceEvent::Init(AirplaneService { handle, data }))
                    .await;

                while let Some(data) = updates.next().await {
                    let _ = output.send(ServiceEvent::Update(data)).await;
                }
            })
        })
    }
}

impl Service for AirplaneService {
    type Command = AirplaneCommand;

    fn command(&mut self, command: Self::Command) -> Task<ServiceEvent<Self>> {
        let handle = self.handle.clone();

        Task::future(async move {
            if let Err(err) = handle.execute(command).await {
                warn!("Airplane mode command failed: {err:#}");
            }
        })
        .discard()
    }
}

pub fn icon(active: bool) -> StaticIcon {
    if active {
        StaticIcon::AirplaneOn
    } else {
        StaticIcon::AirplaneOff
    }
}
