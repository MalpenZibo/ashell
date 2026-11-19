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
    sync::Arc,
};

pub use ashell_services::notifications::{
    DEFAULT_ACTION_KEY, Notification, NotificationIcon, NotificationsCommand, NotificationsConfig,
    NotificationsData, Urgency,
};

/// The daemon's handle once it runs, then each snapshot, the first included.
#[derive(Debug, Clone)]
pub enum Event {
    Connected(Notifications),
    Snapshot(NotificationsData),
}

/// The configuration the daemon starts with. It is not part of the
/// subscription's identity: a new one must not restart the daemon, and
/// reaches it as `Configure` instead.
struct Start(Arc<NotificationsConfig>);

impl Hash for Start {
    fn hash<H: Hasher>(&self, _state: &mut H) {}
}

pub fn subscribe(config: Arc<NotificationsConfig>) -> Subscription<Event> {
    Subscription::run_with(Start(config), |Start(config)| {
        let config = NotificationsConfig::clone(config);
        channel(100, async move |mut output| {
            let handle = match Notifications::connect(config).await {
                Ok(handle) => handle,
                Err(err) => {
                    error!("Failed to initialize notifications service: {err}");
                    return;
                }
            };

            let mut updates = pin!(handle.updates());
            let _ = output.send(Event::Connected(handle.clone())).await;

            while let Some(data) = updates.next().await {
                if output.send(Event::Snapshot(data)).await.is_err() {
                    return;
                }
            }
        })
    })
}
