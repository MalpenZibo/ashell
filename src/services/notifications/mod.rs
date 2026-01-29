use crate::services::notifications::dbus::NotificationDaemon;
use crate::services::{ReadOnlyService, ServiceEvent};
use iced::Subscription;
use std::collections::HashMap;
use tokio::sync::mpsc;
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
    sender: Option<mpsc::UnboundedSender<ServiceEvent<Self>>>,
}

impl NotificationsService {
    pub fn new() -> Self {
        Self {
            notifications: HashMap::new(),
            connection: None,
            sender: None,
        }
    }

    pub async fn start(&mut self) -> Result<(), Error> {
        let (tx, _rx) = mpsc::unbounded_channel();

        self.sender = Some(tx.clone());

        let connection = NotificationDaemon::start_server()
            .await
            .map_err(|e| Error::ConnectionError(e.to_string()))?;

        self.connection = Some(connection.clone());

        // Spawn a task to handle notifications
        tokio::spawn(async move {
            let daemon = NotificationDaemon::default();
            let _ = connection
                .object_server()
                .at(dbus::OBJECT_PATH, daemon)
                .await;

            // Listen for signals or something
            // For now, just keep running
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            }
        });

        Ok(())
    }

    pub fn get_notifications(&self) -> &HashMap<u32, Notification> {
        &self.notifications
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
        Subscription::none() // For now, no subscription
    }
}
