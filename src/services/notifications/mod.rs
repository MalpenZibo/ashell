use crate::services::{ReadOnlyService, ServiceEvent};
use iced::Subscription;
use iced::futures::{SinkExt, StreamExt, channel::mpsc::Sender, stream::pending};
use iced::stream::channel;
use iced::widget::{image, svg};
use log::{error, info};
use std::any::TypeId;
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;
use zbus::Connection;
use zbus::zvariant::OwnedValue;

pub mod dbus;

pub use dbus::Notification;
use dbus::NotificationEvent;

#[derive(Debug, Clone)]
pub enum NotificationIcon {
    Image(image::Handle),
    Svg(svg::Handle),
}

impl NotificationIcon {
    pub fn resolve(
        app_name: &str,
        app_icon: &str,
        hints: &HashMap<String, OwnedValue>,
    ) -> Option<Self> {
        icon_candidates(app_name, app_icon, hints)
            .find_map(resolve_candidate)
            .map(Self::from_path)
    }

    fn from_path(path: PathBuf) -> Self {
        let is_svg = path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| e.eq_ignore_ascii_case("svg"));
        if is_svg {
            Self::Svg(svg::Handle::from_path(path))
        } else {
            Self::Image(image::Handle::from_path(path))
        }
    }
}

const HINT_KEYS: &[&str] = &[
    "image-path",
    "image_path",
    "icon-name",
    "icon_name",
    "desktop-entry",
];

fn icon_candidates<'a>(
    app_name: &'a str,
    app_icon: &'a str,
    hints: &'a HashMap<String, OwnedValue>,
) -> impl Iterator<Item = String> + 'a {
    std::iter::once(app_icon.to_string())
        .chain(
            HINT_KEYS
                .iter()
                .filter_map(|k| hints.get(*k).and_then(|v| v.clone().try_into().ok())),
        )
        .chain(std::iter::once(app_name.to_string()))
        .map(|s: String| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn resolve_candidate(candidate: String) -> Option<PathBuf> {
    if let Ok(url) = url::Url::parse(&candidate)
        && url.scheme() == "file"
    {
        return url.to_file_path().ok().filter(|p| p.exists());
    }

    if candidate.contains('/') || candidate.starts_with('.') {
        let path = PathBuf::from(&candidate);
        if path.exists() {
            return Some(path);
        }
    }

    let name = candidate.strip_suffix(".desktop").unwrap_or(&candidate);
    freedesktop_lookup(name)
}

fn freedesktop_lookup(name: &str) -> Option<PathBuf> {
    let base = freedesktop_icons::lookup(name).with_cache();
    match linicon_theme::get_icon_theme() {
        Some(theme) => base
            .with_theme(&theme)
            .find()
            .or_else(|| freedesktop_icons::lookup(name).with_cache().find()),
        None => base.find(),
    }
}

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
        Subscription::run_with(TypeId::of::<Self>(), |_| {
            channel(100, async |mut output| {
                let mut state = State::Init;

                loop {
                    state = NotificationsService::start_listening(state, &mut output).await;
                }
            })
        })
    }
}
