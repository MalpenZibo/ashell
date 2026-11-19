use super::{ReadOnlyService, ServiceEvent};
use ashell_services::notifications::Notifications;
use iced::{
    Subscription,
    futures::{SinkExt, StreamExt},
    stream::channel,
};
use log::error;
use std::{any::TypeId, pin::pin};

pub use ashell_services::notifications::{
    DEFAULT_ACTION_KEY, Notification, NotificationIcon, NotificationsCommand, NotificationsData,
    Urgency,
};

#[derive(Debug, Clone)]
pub struct NotificationsService {
    pub handle: Notifications,
    pub data: NotificationsData,
}

impl ReadOnlyService for NotificationsService {
    type UpdateEvent = NotificationsData;
    type Error = ();

    fn update(&mut self, event: Self::UpdateEvent) {
        self.data = event;
    }

    fn subscribe() -> Subscription<ServiceEvent<Self>> {
        Subscription::run_with(TypeId::of::<Self>(), |_| {
            channel(100, async |mut output| {
                let handle = match Notifications::connect().await {
                    Ok(handle) => handle,
                    Err(err) => {
                        error!("Failed to initialize notifications service: {err}");
                        return;
                    }
                };

                let mut updates = pin!(handle.updates());
                let Some(data) = updates.next().await else {
                    return;
                };
                let _ = output
                    .send(ServiceEvent::Init(NotificationsService {
                        handle: handle.clone(),
                        data,
                    }))
                    .await;

                while let Some(data) = updates.next().await {
                    if output.send(ServiceEvent::Update(data)).await.is_err() {
                        return;
                    }
                }
            })
        })
    }
}
