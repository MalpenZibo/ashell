use super::NotificationIcon;
use log::{debug, info, warn};
use std::collections::HashMap;
use std::time::SystemTime;
use tokio::sync::broadcast;
use zbus::{
    Connection,
    fdo::{DBusProxy, RequestNameFlags, RequestNameReply},
    interface,
    message::Header,
    names::WellKnownName,
    zvariant::OwnedValue,
};

#[derive(Debug, Clone)]
pub enum NotificationEvent {
    Received(Box<Notification>),
    Closed(u32),
}

/// Which instance of an id a close applies to.
#[derive(Debug, Clone, Copy)]
pub enum CloseGuard {
    /// Whatever holds the id now.
    Any,
    /// Only while the id still carries this revision.
    Revision(u64),
}

const NAME: WellKnownName =
    WellKnownName::from_static_str_unchecked("org.freedesktop.Notifications");
pub const OBJECT_PATH: &str = "/org/freedesktop/Notifications";

/// Notification urgency as defined by the freedesktop Notifications
/// specification (sent as the `urgency` byte hint).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum Urgency {
    Low,
    #[default]
    Normal,
    Critical,
}

impl Urgency {
    fn from_hints(hints: &HashMap<String, OwnedValue>) -> Self {
        hints
            .get("urgency")
            .and_then(|v| u8::try_from(v).ok())
            .map(|b| match b {
                0 => Self::Low,
                2 => Self::Critical,
                _ => Self::Normal,
            })
            .unwrap_or_default()
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Notification {
    pub id: u32,
    /// Bumped on every `Notify`, so a consumer can tell one instance of an id
    /// from the next one a `replaces_id` put in its place.
    pub revision: u64,
    pub app_name: String,
    pub app_icon: String,
    pub summary: String,
    pub body: String,
    pub actions: Vec<String>,
    /// Kept instead of all hints, which can carry raw `image-data`.
    #[serde(default)]
    pub desktop_entry: Option<String>,
    pub expire_timeout: i32,
    pub timestamp: SystemTime,
    pub urgency: Urgency,
    /// Unique name of the D-Bus connection that sent the notification, whose
    /// pid finds the sender's window on click. For sandboxed apps (Flatpak's
    /// D-Bus proxy) or portal notifications that is not the app's own pid.
    #[serde(default)]
    pub sender: Option<String>,
    #[serde(skip)]
    pub icon: Option<NotificationIcon>,
}

pub const DEFAULT_ACTION_KEY: &str = "default";

impl Notification {
    /// `(key, label)` pairs from the flat `[key, label, ...]` action list.
    pub fn action_pairs(&self) -> impl Iterator<Item = (&str, &str)> {
        self.actions
            .as_chunks::<2>()
            .0
            .iter()
            .map(|[key, label]| (key.as_str(), label.as_str()))
    }

    /// App ids or window classes the sender's windows likely carry: the
    /// `desktop-entry` hint (without `.desktop`), then the app name.
    pub fn app_ids(&self) -> Vec<String> {
        self.desktop_entry
            .iter()
            .cloned()
            .chain(std::iter::once(self.app_name.clone()))
            .map(|id| {
                let id = id.trim();
                id.strip_suffix(".desktop").unwrap_or(id).to_string()
            })
            .filter(|id| !id.is_empty())
            .collect()
    }
}

pub struct NotificationDaemon {
    next_id: u32,
    next_revision: u64,
    revisions: HashMap<u32, u64>,
    event_tx: broadcast::Sender<NotificationEvent>,
    connection: Connection,
}

impl NotificationDaemon {
    pub fn new(event_tx: broadcast::Sender<NotificationEvent>, connection: Connection) -> Self {
        Self {
            next_id: 0,
            next_revision: 0,
            revisions: HashMap::new(),
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
        #[zbus(header)] header: Header<'_>,
    ) -> u32 {
        let id = if replaces_id == 0 {
            self.next_id = self.next_id.checked_add(1).unwrap_or(1);
            self.next_id
        } else {
            self.next_id = self.next_id.max(replaces_id);
            replaces_id
        };

        self.next_revision += 1;
        let revision = self.next_revision;
        self.revisions.insert(id, revision);

        let icon = NotificationIcon::resolve(&app_name, &app_icon, &hints);
        let urgency = Urgency::from_hints(&hints);
        let desktop_entry = hints
            .get("desktop-entry")
            .and_then(|v| String::try_from(v.clone()).ok());
        let notification = Notification {
            id,
            revision,
            app_name,
            app_icon,
            summary,
            body,
            actions,
            desktop_entry,
            expire_timeout,
            timestamp: SystemTime::now(),
            urgency,
            sender: header.sender().map(|sender| sender.to_string()),
            icon,
        };

        debug!("New notification: {:?}", notification);

        // Send event through channel
        let _ = self
            .event_tx
            .send(NotificationEvent::Received(Box::new(notification)));

        id
    }

    async fn close_notification(&mut self, id: u32) {
        self.close_with_reason(id, 3).await;
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
        Self::init_server().await
    }

    async fn init_server() -> anyhow::Result<(Connection, broadcast::Sender<NotificationEvent>)> {
        let (event_tx, _rx) = broadcast::channel(100);

        let connection = zbus::connection::Connection::session().await?;
        let daemon = NotificationDaemon::new(event_tx.clone(), connection.clone());
        connection.object_server().at(OBJECT_PATH, daemon).await?;

        let dbus_proxy = DBusProxy::new(&connection).await?;
        let flags = RequestNameFlags::AllowReplacement | RequestNameFlags::ReplaceExisting;
        match dbus_proxy.request_name(NAME, flags).await? {
            RequestNameReply::PrimaryOwner | RequestNameReply::AlreadyOwner => {
                info!("Acquired notification daemon bus name");
            }
            RequestNameReply::InQueue | RequestNameReply::Exists => {
                warn!("Bus name '{NAME}' already owned");
            }
        }

        Ok((connection, event_tx))
    }

    /// `guard` pins the instance the caller acted on: both steps are skipped
    /// when `id` no longer carries it, because a `Notify` won the race against
    /// this call and the notification the user acted on is gone. Closing anyway
    /// would take the replacement down with it. Holding the interface lock over
    /// the check and both steps is what keeps that ordering decidable.
    ///
    /// Returns whether `id` was closed, false when it was replaced since.
    pub async fn invoke_and_close(
        connection: &Connection,
        id: u32,
        action_key: Option<String>,
        guard: CloseGuard,
    ) -> anyhow::Result<bool> {
        let iface_ref = connection
            .object_server()
            .interface::<_, NotificationDaemon>(OBJECT_PATH)
            .await?;
        let mut daemon = iface_ref.get_mut().await;

        if let CloseGuard::Revision(revision) = guard
            && daemon.revisions.get(&id) != Some(&revision)
        {
            debug!("Skipping close of notification {}: replaced since", id);
            return Ok(false);
        }

        if let Some(action_key) = action_key {
            connection
                .emit_signal(
                    None::<&str>,
                    OBJECT_PATH,
                    NAME.as_str(),
                    "ActionInvoked",
                    &(id, action_key),
                )
                .await?;
        }

        daemon.close_with_reason(id, 2).await;
        Ok(true)
    }

    /// `reason` per the freedesktop Notifications spec: 1 expired,
    /// 2 dismissed by the user, 3 closed by `CloseNotification`.
    async fn close_with_reason(&mut self, id: u32, reason: u32) {
        self.revisions.remove(&id);

        let _ = self.event_tx.send(NotificationEvent::Closed(id));

        let _ = self
            .connection
            .emit_signal(
                None::<&str>,
                OBJECT_PATH,
                NAME.as_str(),
                "NotificationClosed",
                &(id, reason),
            )
            .await;
    }

    /// Skips ids replaced since, like [`Self::invoke_and_close`].
    pub async fn evict_notifications(
        connection: &Connection,
        evicted: &[(u32, u64)],
    ) -> anyhow::Result<()> {
        let iface_ref = connection
            .object_server()
            .interface::<_, NotificationDaemon>(OBJECT_PATH)
            .await?;
        let mut daemon = iface_ref.get_mut().await;

        for (id, revision) in evicted {
            if daemon.revisions.get(id) != Some(revision) {
                debug!("Skipping eviction of notification {id}: replaced since");
                continue;
            }
            daemon.close_with_reason(*id, 1).await;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn notification(app_name: &str, desktop_entry: Option<&str>) -> Notification {
        Notification {
            id: 1,
            revision: 1,
            app_name: app_name.to_string(),
            app_icon: String::new(),
            summary: String::new(),
            body: String::new(),
            actions: vec![],
            desktop_entry: desktop_entry.map(str::to_string),
            expire_timeout: -1,
            timestamp: SystemTime::now(),
            urgency: Urgency::Normal,
            sender: None,
            icon: None,
        }
    }

    #[test]
    fn app_ids_prefer_the_desktop_entry() {
        let n = notification("Telegram Desktop", Some("org.telegram.desktop.desktop"));
        assert_eq!(n.app_ids(), ["org.telegram.desktop", "Telegram Desktop"]);
    }

    #[test]
    fn app_ids_fall_back_to_the_app_name() {
        assert_eq!(notification("firefox", None).app_ids(), ["firefox"]);
        assert!(notification(" ", None).app_ids().is_empty());
    }
}
