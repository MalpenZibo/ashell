use crate::services::notifications::dbus::NotificationDaemon;
use crate::services::{ReadOnlyService, ServiceEvent};
use iced::Subscription;
use iced::futures::SinkExt;
use std::collections::HashMap;
use zbus::Connection;

pub mod dbus;

pub use dbus::Notification;

#[derive(Debug, Clone)]
pub enum UpdateEvent {
    NotificationReceived(Notification),
    NotificationClosed(u32),
}

#[derive(Debug, Clone)]
pub enum Error {
    ConnectionError(String),
}

#[derive(Debug)]
pub struct NotificationsService {
    notifications: HashMap<u32, Notification>,
    connection: Option<Connection>,
}

impl NotificationsService {
    pub fn new() -> Self {
        Self {
            notifications: HashMap::new(),
            connection: None,
        }
    }

    pub async fn start(&mut self) -> Result<(), Error> {
        let connection = NotificationDaemon::start_server()
            .await
            .map_err(|e| Error::ConnectionError(e.to_string()))?;

        self.connection = Some(connection);

        Ok(())
    }
}

impl ReadOnlyService for NotificationsService {
    type UpdateEvent = UpdateEvent;
    type Error = Error;

    fn update(&mut self, event: UpdateEvent) {
        match event {
            UpdateEvent::NotificationReceived(notification) => {
                self.notifications.insert(notification.id, notification);
            }
            UpdateEvent::NotificationClosed(id) => {
                self.notifications.remove(&id);
            }
        }
    }

    fn subscribe() -> Subscription<ServiceEvent<Self>> {
        use std::any::TypeId;

        let id = TypeId::of::<Self>();

        Subscription::run_with_id(
            id,
            iced::stream::channel(100, |mut output| async move {
                let mut last_notifications = HashMap::new();
                loop {
                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

                    if let Some(notifications) = crate::modules::notifications::NOTIFICATIONS.get()
                    {
                        let current_notifications = if let Ok(guard) = notifications.lock() {
                            (*guard).clone()
                        } else {
                            HashMap::new()
                        };
                        // Check for new notifications
                        for (id, notification) in current_notifications.iter() {
                            if !last_notifications.contains_key(id) {
                                let _ = output
                                    .send(ServiceEvent::Update(UpdateEvent::NotificationReceived(
                                        notification.clone(),
                                    )))
                                    .await;
                            }
                        }

                        // Check for closed notifications
                        for id in last_notifications.keys() {
                            if !current_notifications.contains_key(id) {
                                let _ = output
                                    .send(ServiceEvent::Update(UpdateEvent::NotificationClosed(
                                        *id,
                                    )))
                                    .await;
                            }
                        }

                        last_notifications = current_notifications;
                    }
                }
            }),
        )
    }
}
