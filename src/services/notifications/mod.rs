use crate::services::{ReadOnlyService, ServiceEvent};
use iced::Subscription;
use iced::futures::{SinkExt, StreamExt, channel::mpsc::Sender, stream::pending};
use iced::stream::channel;
use log::{error, info};
use std::any::TypeId;
use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;
use zbus::Connection;

pub mod dbus;

pub use dbus::Notification;
use dbus::NotificationEvent;

#[derive(Debug, Clone)]
pub struct NotificationsService {
    pub connection: Connection,
}

impl NotificationsService {
    async fn init_service() -> anyhow::Result<(Connection, broadcast::Sender<NotificationEvent>)> {
        let (connection, event_tx) = dbus::NotificationDaemon::start_server().await?;
        Ok((connection, event_tx))
    }

    async fn start_listening(state: State, output: &mut Sender<ServiceEvent<Self>>) -> State {
        match state {
            State::Init => match Self::init_service().await {
                Ok((connection, event_tx)) => {
                    info!("Notifications service initialized");
                    let _ = output
                        .send(ServiceEvent::Init(NotificationsService {
                            connection: connection.clone(),
                        }))
                        .await;
                    State::Active(connection, event_tx)
                }
                Err(err) => {
                    error!("Failed to initialize notifications service: {err}");
                    State::Error
                }
            },
            State::Active(_connection, event_tx) => {
                let rx = event_tx.subscribe();
                let mut stream = BroadcastStream::new(rx);

                while let Some(result) = stream.next().await {
                    match result {
                        Ok(event) => {
                            let _ = output.send(ServiceEvent::Update(event)).await;
                        }
                        Err(e) => {
                            error!("Error receiving notification event: {e}");
                        }
                    }
                }
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
    Active(Connection, broadcast::Sender<NotificationEvent>),
    Error,
}

impl ReadOnlyService for NotificationsService {
    type UpdateEvent = NotificationEvent;
    type Error = ();

    fn update(&mut self, _event: NotificationEvent) {}

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
