use crate::services::notifications::dbus::{NotificationDaemon, NotificationEvent};
use crate::services::{ReadOnlyService, ServiceEvent};
use iced::Subscription;
use iced::futures::{SinkExt, StreamExt, channel::mpsc::Sender, stream::pending};
use iced::stream::channel;
use log::{error, info};
use std::any::TypeId;
use std::collections::HashMap;
use std::ops::{Deref, DerefMut};
use tokio_stream::wrappers::BroadcastStream;
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
                // Subscribe to notification events from the broadcast channel
                if let Some(tx) = dbus::NOTIFICATION_EVENTS.get() {
                    let rx = tx.subscribe();
                    let mut stream = BroadcastStream::new(rx);

                    while let Some(result) = stream.next().await {
                        match result {
                            Ok(event) => {
                                let update = match event {
                                    NotificationEvent::Received(notification) => {
                                        UpdateEvent::NotificationReceived(notification)
                                    }
                                    NotificationEvent::Closed(id) => {
                                        UpdateEvent::NotificationClosed(id)
                                    }
                                };
                                let _ = output.send(ServiceEvent::Update(update)).await;
                            }
                            Err(e) => {
                                error!("Error receiving notification event: {e}");
                            }
                        }
                    }
                }
                // If we exit the loop, something went wrong
                error!("Notification event stream ended unexpectedly");
                State::Error
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
