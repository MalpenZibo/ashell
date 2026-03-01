use freedesktop_icons::lookup;
use linicon_theme::get_icon_theme;
use log::{debug, info, warn};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::SystemTime;
use tokio::sync::broadcast;
use zbus::{
    Connection,
    fdo::{DBusProxy, RequestNameFlags, RequestNameReply},
    interface,
    names::WellKnownName,
    zvariant::OwnedValue,
};

#[derive(Debug, Clone)]
pub enum NotificationEvent {
    Received(Notification),
    Closed(u32),
}

const NAME: WellKnownName =
    WellKnownName::from_static_str_unchecked("org.freedesktop.Notifications");
pub const OBJECT_PATH: &str = "/org/freedesktop/Notifications";

fn non_empty_owned_value_string(value: Option<&OwnedValue>) -> Option<String> {
    value
        .and_then(|v| v.clone().try_into().ok())
        .map(|s: String| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn parse_file_url(value: &str) -> Option<PathBuf> {
    if !value.starts_with("file://") {
        return None;
    }

    let decoded = url::Url::parse(value).ok()?.to_file_path().ok()?;
    decoded.exists().then_some(decoded)
}

fn find_icon_path(icon_name: &str) -> Option<PathBuf> {
    let base_lookup = lookup(icon_name).with_cache();

    match get_icon_theme() {
        Some(theme) => base_lookup.with_theme(&theme).find().or_else(|| {
            let fallback_lookup = lookup(icon_name).with_cache();
            fallback_lookup.find()
        }),
        None => base_lookup.find(),
    }
}

fn resolve_notification_icon_path(
    app_name: &str,
    app_icon: &str,
    hints: &HashMap<String, OwnedValue>,
) -> Option<String> {
    let mut candidates = Vec::new();

    if !app_icon.trim().is_empty() {
        candidates.push(app_icon.trim().to_string());
    }

    for key in [
        "image-path",
        "image_path",
        "icon-name",
        "icon_name",
        "desktop-entry",
    ] {
        if let Some(value) = non_empty_owned_value_string(hints.get(key)) {
            candidates.push(value);
        }
    }

    if !app_name.trim().is_empty() {
        candidates.push(app_name.trim().to_string());
    }

    for candidate in candidates {
        if let Some(path) = parse_file_url(&candidate) {
            return Some(path.to_string_lossy().into_owned());
        }

        let candidate_path = PathBuf::from(&candidate);
        if (candidate.contains('/') || candidate.starts_with('.')) && candidate_path.exists() {
            return Some(candidate_path.to_string_lossy().into_owned());
        }

        if let Some(path) = find_icon_path(&candidate) {
            return Some(path.to_string_lossy().into_owned());
        }

        if let Some(stripped) = candidate.strip_suffix(".desktop")
            && let Some(path) = find_icon_path(stripped)
        {
            return Some(path.to_string_lossy().into_owned());
        }
    }

    None
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Notification {
    pub id: u32,
    pub app_name: String,
    pub app_icon: String,
    pub resolved_icon_path: Option<String>,
    pub summary: String,
    pub body: String,
    pub actions: Vec<String>,
    pub hints: HashMap<String, OwnedValue>,
    pub expire_timeout: i32,
    pub timestamp: SystemTime,
}

pub struct NotificationDaemon {
    notifications: HashMap<u32, Notification>,
    next_id: u32,
    event_tx: broadcast::Sender<NotificationEvent>,
    connection: Connection,
}

impl NotificationDaemon {
    pub fn new(event_tx: broadcast::Sender<NotificationEvent>, connection: Connection) -> Self {
        Self {
            notifications: HashMap::new(),
            next_id: 0,
            event_tx,
            connection,
        }
    }
}

#[interface(name = "org.freedesktop.Notifications")]
impl NotificationDaemon {
    async fn get_capabilities(&self) -> Vec<String> {
        vec![
            "body".to_string(),
            "body-markup".to_string(),
            "icon-static".to_string(),
            "actions".to_string(),
        ]
    }

    #[allow(clippy::too_many_arguments)]
    async fn notify(
        &mut self,
        app_name: String,
        replaces_id: u32,
        app_icon: String,
        summary: String,
        body: String,
        actions: Vec<String>,
        hints: HashMap<String, OwnedValue>,
        expire_timeout: i32,
    ) -> u32 {
        let id = if replaces_id == 0 {
            self.next_id += 1;
            self.next_id
        } else {
            replaces_id
        };

        let resolved_icon_path = resolve_notification_icon_path(&app_name, &app_icon, &hints);

        let notification = Notification {
            id,
            app_name,
            app_icon,
            resolved_icon_path,
            summary,
            body,
            actions,
            hints,
            expire_timeout,
            timestamp: SystemTime::now(),
        };

        self.notifications.insert(id, notification.clone());
        debug!("New notification: {:?}", notification);

        // Send event through channel
        let _ = self
            .event_tx
            .send(NotificationEvent::Received(notification));

        id
    }

    async fn close_notification(&mut self, id: u32) {
        let removed = self.notifications.remove(&id).is_some();

        if removed {
            // Send event through channel
            let _ = self.event_tx.send(NotificationEvent::Closed(id));

            // Emit DBus signal for external applications
            let _ = self
                .connection
                .emit_signal(
                    None::<&str>,
                    OBJECT_PATH,
                    NAME.as_str(),
                    "NotificationClosed",
                    &(id, 3u32),
                )
                .await;
        }
    }

    async fn get_server_information(&self) -> (String, String, String, String) {
        (
            "ashell".to_string(),
            "MalpenZibo".to_string(),
            "0.1".to_string(),
            "1.2".to_string(),
        )
    }
}

impl NotificationDaemon {
    pub async fn start_server() -> anyhow::Result<(Connection, broadcast::Sender<NotificationEvent>)>
    {
        // Initialize the event channel (100 message buffer)
        let (event_tx, _rx) = broadcast::channel(100);

        let connection = zbus::connection::Connection::session().await?;
        let daemon = NotificationDaemon::new(event_tx.clone(), connection.clone());
        connection.object_server().at(OBJECT_PATH, daemon).await?;

        let dbus_proxy = DBusProxy::new(&connection).await?;
        let flags = RequestNameFlags::AllowReplacement.into();
        if dbus_proxy.request_name(NAME, flags).await? == RequestNameReply::InQueue {
            warn!("Bus name '{NAME}' already owned");
        } else {
            info!("Acquired notification daemon bus name");
        }

        Ok((connection, event_tx))
    }

    pub async fn invoke_action(
        connection: &Connection,
        id: u32,
        action_key: String,
    ) -> anyhow::Result<()> {
        connection
            .emit_signal(
                None::<&str>,
                OBJECT_PATH,
                NAME.as_str(),
                "ActionInvoked",
                &(id, action_key),
            )
            .await?;
        Ok(())
    }

    pub async fn close_notification_by_id(connection: &Connection, id: u32) -> anyhow::Result<()> {
        // Get the object server interface to call the method properly
        let iface_ref = connection
            .object_server()
            .interface::<_, NotificationDaemon>(OBJECT_PATH)
            .await?;

        // Call the close_notification method which properly cleans up and emits events
        iface_ref.get_mut().await.close_notification(id).await;
        Ok(())
    }
}
