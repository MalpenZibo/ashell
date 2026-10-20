use super::{ReadOnlyService, ServiceEvent};
use ashell_services::privacy::Privacy;
use iced::{
    Subscription,
    futures::{SinkExt, StreamExt},
    stream::channel,
};
use log::error;
use std::{any::TypeId, ops::Deref, pin::pin};

pub use ashell_services::privacy::PrivacyData;

#[derive(Debug, Clone)]
pub struct PrivacyService {
    data: PrivacyData,
}

impl Deref for PrivacyService {
    type Target = PrivacyData;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl ReadOnlyService for PrivacyService {
    type UpdateEvent = PrivacyData;
    type Error = ();

    fn update(&mut self, event: Self::UpdateEvent) {
        self.data = event;
    }

    fn subscribe() -> Subscription<ServiceEvent<Self>> {
        Subscription::run_with(TypeId::of::<Self>(), |_| {
            channel(100, async |mut output| {
                let handle = match Privacy::connect().await {
                    Ok(handle) => handle,
                    Err(err) => {
                        error!("Failed to start the privacy service: {err}");
                        return;
                    }
                };

                let mut updates = pin!(handle.updates());
                let Some(data) = updates.next().await else {
                    return;
                };
                let _ = output
                    .send(ServiceEvent::Init(PrivacyService { data }))
                    .await;

                while let Some(data) = updates.next().await {
                    let _ = output.send(ServiceEvent::Update(data)).await;
                }
            })
        })
    }
}
