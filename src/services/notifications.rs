use super::{ReadOnlyService, ServiceEvent};
use ashell_services::notifications::Notifications;
use iced::{
    Subscription,
    futures::{SinkExt, StreamExt},
    stream::channel,
};
use log::error;
use std::{
    hash::{Hash, Hasher},
    pin::pin,
};

pub use ashell_services::notifications::{
    DEFAULT_ACTION_KEY, Notification, NotificationIcon, NotificationsCommand, NotificationsConfig,
    NotificationsData, Urgency,
};

/// Sent once connected; every snapshot, the first one included, follows as an
/// `Update`.
#[derive(Debug, Clone)]
pub struct NotificationsService {
    pub handle: Notifications,
}

impl ReadOnlyService for NotificationsService {
    type UpdateEvent = NotificationsData;
    type Error = ();

    fn update(&mut self, _event: Self::UpdateEvent) {}

    fn subscribe() -> Subscription<ServiceEvent<Self>> {
        Self::subscribe_with(NotificationsConfig::default())
    }
}

/// The configuration the daemon starts with. It is not part of the
/// subscription's identity: a new one must not restart the daemon, and
/// reaches it as `Configure` instead.
struct Start(NotificationsConfig);

impl Hash for Start {
    fn hash<H: Hasher>(&self, _state: &mut H) {}
}

impl NotificationsService {
    pub fn subscribe_with(config: NotificationsConfig) -> Subscription<ServiceEvent<Self>> {
        Subscription::run_with(Start(config), |Start(config)| {
            let config = config.clone();
            channel(100, async move |mut output| {
                let handle = match Notifications::connect(config).await {
                    Ok(handle) => handle,
                    Err(err) => {
                        error!("Failed to initialize notifications service: {err}");
                        return;
                    }
                };

                let mut updates = pin!(handle.updates());
                let _ = output
                    .send(ServiceEvent::Init(NotificationsService {
                        handle: handle.clone(),
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
