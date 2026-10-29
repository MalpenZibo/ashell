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
    CloseGuard, DEFAULT_ACTION_KEY, Notification, NotificationEvent, NotificationIcon, Urgency,
};

#[derive(Debug, Clone)]
pub struct NotificationsService {
    pub handle: Notifications,
}

impl ReadOnlyService for NotificationsService {
    type UpdateEvent = NotificationEvent;
    type Error = ();

    fn update(&mut self, _event: Self::UpdateEvent) {}

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

                let _ = output
                    .send(ServiceEvent::Init(NotificationsService {
                        handle: handle.clone(),
                    }))
                    .await;

                let mut events = pin!(handle.events());
                while let Some(event) = events.next().await {
                    if output.send(ServiceEvent::Update(event)).await.is_err() {
                        return;
                    }
                }

                error!("Notification event stream ended unexpectedly");
            })
        })
    }
}
