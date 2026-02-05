use crate::services::notifications::dbus::NotificationDaemon;
use crate::services::{ReadOnlyService, ServiceEvent};
use iced::Subscription;
use iced::futures::{SinkExt, StreamExt, channel::mpsc::Sender, stream::pending};
use iced::stream::channel;
use log::{error, info};
use std::any::TypeId;
use std::collections::HashMap;
use std::ops::{Deref, DerefMut};
use zbus::Connection;

pub mod dbus;

pub use dbus::Notification;

#[derive(Debug, Clone)]
pub enum UpdateEvent {
    NotificationReceived(Notification),
    NotificationClosed(u32),
}

#[derive(Debug, Clone)]
pub struct NotificationsData {
    pub notifications: HashMap<u32, Notification>,
}

#[derive(Debug, Clone)]
pub struct NotificationsService {
    data: NotificationsData,
    connection: Connection,
}

impl Deref for NotificationsService {
    type Target = NotificationsData;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl DerefMut for NotificationsService {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}

impl NotificationsService {
    async fn init_service() -> anyhow::Result<Connection> {
        let connection = NotificationDaemon::start_server().await?;
        Ok(connection)
    }

    async fn start_listening(state: State, output: &mut Sender<ServiceEvent<Self>>) -> State {
        match state {
            State::Init => match Self::init_service().await {
                Ok(connection) => {
                    info!("Notifications service initialized");
                    let _ = output
                        .send(ServiceEvent::Init(NotificationsService {
                            data: NotificationsData {
                                notifications: HashMap::new(),
                            },
                            connection: connection.clone(),
                        }))
                        .await;
                    State::Active(connection)
                }
                Err(err) => {
                    error!("Failed to initialize notifications service: {err}");
                    State::Error
                }
            },
            State::Active(_connection) => {
                let mut last_notifications = HashMap::new();
                loop {
                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

                    if let Some(notifications) = dbus::NOTIFICATIONS.get() {
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
            }
            State::Error => {
                error!("Notifications service error");
                let _ = pending::<u8>().next().await;
                State::Error
            }
        }
    }
}

enum State {
    Init,
    Active(Connection),
    Error,
}

impl ReadOnlyService for NotificationsService {
    type UpdateEvent = UpdateEvent;
    type Error = ();

    fn update(&mut self, event: UpdateEvent) {
        match event {
            UpdateEvent::NotificationReceived(notification) => {
                self.data.notifications.insert(notification.id, notification);
            }
            UpdateEvent::NotificationClosed(id) => {
                self.data.notifications.remove(&id);
            }
        }
    }

    fn subscribe() -> Subscription<ServiceEvent<Self>> {
        let id = TypeId::of::<Self>();

        Subscription::run_with_id(
            id,
            channel(100, async |mut output| {
                let mut state = State::Init;

                loop {
                    state = NotificationsService::start_listening(state, &mut output).await;
                }
            }),
        )
    }
}
