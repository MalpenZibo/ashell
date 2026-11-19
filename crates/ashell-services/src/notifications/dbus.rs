use super::{NotificationIcon, NotificationsData};
use log::{debug, info, warn};
use regex::Regex;
use std::{
    collections::{HashMap, VecDeque},
    sync::Arc,
    time::SystemTime,
};
use tokio::sync::watch;
use zbus::{
    Connection, OwnedVariant,
    fdo::{DBusProxy, RequestNameFlags, RequestNameReply},
    interface,
    message::Header,
    names::WellKnownName,
    object_server::InterfaceRef,
};

const NAME: WellKnownName =
    WellKnownName::from_static_str_unchecked("org.freedesktop.Notifications");
const OBJECT_PATH: &str = "/org/freedesktop/Notifications";

/// Close reasons from the freedesktop Notifications specification.
const CLOSE_REASON_EXPIRED: u32 = 1;
const CLOSE_REASON_DISMISSED: u32 = 2;
const CLOSE_REASON_CLOSED: u32 = 3;

/// Notification urgency as defined by the freedesktop Notifications
/// specification (sent as the `urgency` byte hint).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Urgency {
    Low,
    #[default]
    Normal,
    Critical,
}

impl Urgency {
    fn from_hints(hints: &HashMap<String, OwnedVariant>) -> Self {
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

#[derive(Debug, Clone, PartialEq)]
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
    pub desktop_entry: Option<String>,
    pub expire_timeout: i32,
    pub timestamp: SystemTime,
    pub urgency: Urgency,
    /// Unique name of the D-Bus connection that sent the notification, whose
    /// pid finds the sender's window on click. For sandboxed apps (Flatpak's
    /// D-Bus proxy) or portal notifications that is not the app's own pid.
    pub sender: Option<String>,
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

/// The notifications the daemon holds, newest first, and the rules that keep
/// them: the history limit and the blocklist.
#[derive(Debug)]
pub(super) struct Store {
    notifications: VecDeque<Arc<Notification>>,
    history_limit: usize,
    blocklist: Vec<Regex>,
}

impl Default for Store {
    fn default() -> Self {
        Self {
            notifications: VecDeque::new(),
            history_limit: DEFAULT_HISTORY_LIMIT,
            blocklist: Vec::new(),
        }
    }
}

const DEFAULT_HISTORY_LIMIT: usize = 200;

impl Store {
    pub(super) fn data(&self) -> NotificationsData {
        NotificationsData {
            notifications: self.notifications.iter().cloned().collect(),
        }
    }

    fn is_blocklisted(&self, app_name: &str) -> bool {
        self.blocklist
            .iter()
            .any(|pattern| pattern.is_match(app_name))
    }

    fn revision(&self, id: u32) -> Option<u64> {
        self.notifications
            .iter()
            .find(|n| n.id == id)
            .map(|n| n.revision)
    }

    /// Puts `notification` first, in place of the one it replaces, and returns
    /// the ids evicted to stay within the history limit.
    fn insert(&mut self, notification: Notification) -> Vec<u32> {
        self.notifications.retain(|n| n.id != notification.id);
        self.notifications.push_front(Arc::new(notification));
        self.evict()
    }

    fn remove(&mut self, id: u32) -> bool {
        let len = self.notifications.len();
        self.notifications.retain(|n| n.id != id);
        self.notifications.len() != len
    }

    /// Drops the oldest notifications over the limit. Critical ones stay until
    /// the user acknowledges them, as the specification asks.
    fn evict(&mut self) -> Vec<u32> {
        let mut evicted = Vec::new();
        while self.notifications.len() > self.history_limit
            && let Some(pos) = self
                .notifications
                .iter()
                .rposition(|n| n.urgency != Urgency::Critical)
            && let Some(notification) = self.notifications.remove(pos)
        {
            evicted.push(notification.id);
        }
        evicted
    }

    fn configure(&mut self, history_limit: usize, blocklist: Vec<Regex>) -> Vec<u32> {
        self.history_limit = history_limit;
        self.blocklist = blocklist;
        self.evict()
    }

    /// Removes every notification, or every one from `app_name`, and returns
    /// their ids.
    fn remove_all(&mut self, app_name: Option<&str>) -> Vec<u32> {
        let mut removed = Vec::new();
        self.notifications.retain(|n| {
            let keep = app_name.is_some_and(|app| n.app_name != app);
            if !keep {
                removed.push(n.id);
            }
            keep
        });
        removed
    }
}

pub(super) struct NotificationDaemon {
    next_id: u32,
    next_revision: u64,
    store: Store,
    data: watch::Sender<NotificationsData>,
    connection: Connection,
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
        hints: HashMap<String, OwnedVariant>,
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

        if self.store.is_blocklisted(&app_name) {
            debug!("Dropping notification {id} from blocklisted {app_name}");
            // Closed from a task so the reply, which tells the sender the id,
            // most likely goes out first.
            let connection = self.connection.clone();
            tokio::spawn(async move {
                emit_closed(&connection, id, CLOSE_REASON_EXPIRED).await;
            });
            return id;
        }

        self.next_revision += 1;
        let notification = Notification {
            id,
            revision: self.next_revision,
            icon: NotificationIcon::resolve(&app_name, &app_icon, &hints),
            urgency: Urgency::from_hints(&hints),
            desktop_entry: hints
                .get("desktop-entry")
                .and_then(|v| String::try_from(v.clone()).ok()),
            app_name,
            app_icon,
            summary,
            body,
            actions,
            expire_timeout,
            timestamp: SystemTime::now(),
            sender: header.sender().map(|sender| sender.to_string()),
        };
        debug!(
            "New notification {id} from {}: {}",
            notification.app_name, notification.summary
        );

        let evicted = self.store.insert(notification);
        self.publish();
        self.emit_all_closed(&evicted, CLOSE_REASON_EXPIRED).await;

        id
    }

    async fn close_notification(&mut self, id: u32) {
        if self.store.remove(id) {
            self.publish();
        }
        emit_closed(&self.connection, id, CLOSE_REASON_CLOSED).await;
    }

    async fn get_server_information(&self) -> (String, String, String, String) {
        (
            "ashell".to_string(),
            "MalpenZibo".to_string(),
            env!("CARGO_PKG_VERSION").to_string(),
            "1.2".to_string(),
        )
    }
}

impl NotificationDaemon {
    pub(super) async fn start(
        data: watch::Sender<NotificationsData>,
    ) -> anyhow::Result<Connection> {
        let connection = Connection::session().await?;
        let daemon = NotificationDaemon {
            next_id: 0,
            next_revision: 0,
            store: Store::default(),
            data,
            connection: connection.clone(),
        };
        connection.object_server().at(OBJECT_PATH, daemon).await?;

        let dbus_proxy = DBusProxy::new(&connection).await?;
        let flags = RequestNameFlags::ALLOW_REPLACEMENT | RequestNameFlags::REPLACE_EXISTING;
        match dbus_proxy.request_name(NAME, flags).await? {
            RequestNameReply::PrimaryOwner | RequestNameReply::AlreadyOwner => {
                info!("Acquired notification daemon bus name");
            }
            RequestNameReply::InQueue | RequestNameReply::Exists => {
                warn!("Bus name '{NAME}' already owned");
            }
        }

        Ok(connection)
    }

    pub(super) async fn interface(
        connection: &Connection,
    ) -> anyhow::Result<InterfaceRef<NotificationDaemon>> {
        Ok(connection
            .object_server()
            .interface::<_, NotificationDaemon>(OBJECT_PATH)
            .await?)
    }

    fn publish(&self) {
        self.data.send_replace(self.store.data());
    }

    async fn emit_all_closed(&self, ids: &[u32], reason: u32) {
        for &id in ids {
            emit_closed(&self.connection, id, reason).await;
        }
    }

    pub(super) async fn configure(&mut self, history_limit: usize, blocklist: Vec<Regex>) {
        let evicted = self.store.configure(history_limit, blocklist);
        if !evicted.is_empty() {
            self.publish();
            self.emit_all_closed(&evicted, CLOSE_REASON_EXPIRED).await;
        }
    }

    pub(super) async fn invoke(
        &mut self,
        id: u32,
        revision: u64,
        action_key: Option<String>,
    ) -> anyhow::Result<bool> {
        if self.store.revision(id) != Some(revision) {
            debug!("Skipping notification {id}: replaced or closed since");
            return Ok(false);
        }

        if let Some(action_key) = action_key {
            self.connection
                .emit_signal(
                    None::<&str>,
                    OBJECT_PATH,
                    NAME.as_str(),
                    "ActionInvoked",
                    &(id, action_key),
                )
                .await?;
        }

        self.store.remove(id);
        self.publish();
        emit_closed(&self.connection, id, CLOSE_REASON_DISMISSED).await;
        Ok(true)
    }

    /// Dismisses every notification, or every one from `app_name`.
    pub(super) async fn dismiss_all(&mut self, app_name: Option<&str>) {
        let ids = self.store.remove_all(app_name);
        if ids.is_empty() {
            return;
        }
        self.publish();
        self.emit_all_closed(&ids, CLOSE_REASON_DISMISSED).await;
    }
}

async fn emit_closed(connection: &Connection, id: u32, reason: u32) {
    if let Err(e) = connection
        .emit_signal(
            None::<&str>,
            OBJECT_PATH,
            NAME.as_str(),
            "NotificationClosed",
            &(id, reason),
        )
        .await
    {
        warn!("Failed to emit NotificationClosed for {id}: {e}");
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

    #[test]
    fn action_pairs_pair_keys_with_labels() {
        let mut n = notification("app", None);
        n.actions = vec![
            "default".to_string(),
            "Open".to_string(),
            "reply".to_string(),
            "Reply".to_string(),
        ];
        assert_eq!(
            n.action_pairs().collect::<Vec<_>>(),
            [("default", "Open"), ("reply", "Reply")]
        );
    }

    #[test]
    fn urgency_reads_the_byte_hint() {
        let mut hints = HashMap::new();
        assert_eq!(Urgency::from_hints(&hints), Urgency::Normal);

        hints.insert("urgency".to_string(), OwnedVariant::from(0u8));
        assert_eq!(Urgency::from_hints(&hints), Urgency::Low);

        hints.insert("urgency".to_string(), OwnedVariant::from(2u8));
        assert_eq!(Urgency::from_hints(&hints), Urgency::Critical);

        hints.insert("urgency".to_string(), OwnedVariant::from(1u8));
        assert_eq!(Urgency::from_hints(&hints), Urgency::Normal);
    }

    fn stored(id: u32, urgency: Urgency) -> Notification {
        Notification {
            id,
            revision: u64::from(id),
            urgency,
            ..notification("app", None)
        }
    }

    fn ids(store: &Store) -> Vec<u32> {
        store.notifications.iter().map(|n| n.id).collect()
    }

    #[test]
    fn a_replacement_moves_to_the_front() {
        let mut store = Store::default();
        store.insert(stored(1, Urgency::Normal));
        store.insert(stored(2, Urgency::Normal));
        store.insert(Notification {
            revision: 9,
            ..stored(1, Urgency::Normal)
        });

        assert_eq!(ids(&store), [1, 2]);
        assert_eq!(store.revision(1), Some(9));
    }

    #[test]
    fn eviction_drops_the_oldest_but_keeps_critical_ones() {
        let mut store = Store::default();
        store.configure(2, Vec::new());
        store.insert(stored(1, Urgency::Critical));
        store.insert(stored(2, Urgency::Normal));
        let evicted = store.insert(stored(3, Urgency::Normal));

        assert_eq!(evicted, [2]);
        assert_eq!(ids(&store), [3, 1]);
    }

    #[test]
    fn a_lower_limit_evicts_at_once() {
        let mut store = Store::default();
        for id in 1..=4 {
            store.insert(stored(id, Urgency::Normal));
        }

        assert_eq!(store.configure(2, Vec::new()), [1, 2]);
        assert_eq!(ids(&store), [4, 3]);
    }

    #[test]
    fn the_blocklist_matches_the_app_name() {
        let mut store = Store::default();
        store.configure(10, vec![Regex::new("^spotify$").unwrap()]);

        assert!(store.is_blocklisted("spotify"));
        assert!(!store.is_blocklisted("spotify-launcher"));
    }

    #[test]
    fn notifications_can_be_removed_by_app() {
        let mut store = Store::default();
        store.insert(stored(1, Urgency::Normal));
        store.insert(Notification {
            app_name: "other".to_string(),
            ..stored(2, Urgency::Normal)
        });
        store.insert(stored(3, Urgency::Normal));

        assert_eq!(store.remove_all(Some("app")), [3, 1]);
        assert_eq!(ids(&store), [2]);
        assert_eq!(store.remove_all(None), [2]);
    }
}
