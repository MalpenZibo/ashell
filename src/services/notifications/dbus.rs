use log::{debug, info, warn};
use std::collections::HashMap;
use std::time::SystemTime;
use tokio::sync::broadcast;
use zbus::{
    Connection,
    fdo::{DBusProxy, RequestNameFlags, RequestNameReply},
    interface,
    names::WellKnownName,
    zvariant::{self, OwnedValue},
};

// Global static for storing notifications - accessed by service layer
pub static NOTIFICATIONS: std::sync::OnceLock<std::sync::Mutex<HashMap<u32, Notification>>> =
    std::sync::OnceLock::new();

pub static NOTIFICATION_CONNECTION: std::sync::OnceLock<Connection> = std::sync::OnceLock::new();

// Channel for internal notification events
pub static NOTIFICATION_EVENTS: std::sync::OnceLock<broadcast::Sender<NotificationEvent>> =
    std::sync::OnceLock::new();

#[derive(Debug, Clone)]
pub enum NotificationEvent {
    Received(Notification),
    Closed(u32),
}

const NAME: WellKnownName =
    WellKnownName::from_static_str_unchecked("org.freedesktop.Notifications");
pub const OBJECT_PATH: &str = "/org/freedesktop/Notifications";

#[derive(Debug, Clone, zvariant::Type, serde::Serialize, serde::Deserialize)]
pub struct Notification {
    pub id: u32,
    pub app_name: String,
    pub app_icon: String,
    pub summary: String,
    pub body: String,
    pub actions: Vec<String>,
    pub hints: HashMap<String, OwnedValue>,
    pub expire_timeout: i32,
    pub timestamp: SystemTime,
}

#[derive(Debug, Default)]
pub struct NotificationDaemon {
    notifications: HashMap<u32, Notification>,
    next_id: u32,
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

        let notification = Notification {
            id,
            app_name,
            app_icon,
            summary,
            body,
            actions,
            hints,
            expire_timeout,
            timestamp: SystemTime::now(),
        };

        self.notifications.insert(id, notification.clone());
        {
            let mut global_notifications = NOTIFICATIONS
                .get_or_init(|| std::sync::Mutex::new(std::collections::HashMap::new()))
                .lock()
                .unwrap();
            global_notifications.insert(id, notification.clone());
        }
        debug!("New notification: {:?}", notification);

        // Send event through channel
        if let Some(tx) = NOTIFICATION_EVENTS.get() {
            let _ = tx.send(NotificationEvent::Received(notification));
        }

        id
    }

    async fn close_notification(&mut self, id: u32) {
        let removed = if self.notifications.remove(&id).is_some() {
            let mut global_notifications = NOTIFICATIONS
                .get_or_init(|| std::sync::Mutex::new(std::collections::HashMap::new()))
                .lock()
                .unwrap();
            global_notifications.remove(&id);
            true
        } else {
            false
        };

        if removed {
            // Send event through channel
            if let Some(tx) = NOTIFICATION_EVENTS.get() {
                let _ = tx.send(NotificationEvent::Closed(id));
            }
            // Emit DBus signal for external applications
            if let Some(connection) = NOTIFICATION_CONNECTION.get() {
                let _ = connection
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
    pub async fn start_server() -> anyhow::Result<Connection> {
        // Check if already initialized and return existing connection
        if let Some(existing_connection) = NOTIFICATION_CONNECTION.get() {
            info!("Notification daemon already running, reusing existing connection");
            return Ok(existing_connection.clone());
        }

        // Initialize the event channel (100 message buffer)
        let (tx, _rx) = broadcast::channel(100);
        if NOTIFICATION_EVENTS.set(tx).is_err() {
            // Already initialized, just continue
        }

        let connection = zbus::connection::Connection::session().await?;
        let daemon = NotificationDaemon::default();
        connection.object_server().at(OBJECT_PATH, daemon).await?;

        // Try to set the connection, but don't panic if already set
        if NOTIFICATION_CONNECTION.set(connection.clone()).is_err() {
            warn!("Notification connection already set");
        }

        let dbus_proxy = DBusProxy::new(&connection).await?;
        let flags = RequestNameFlags::AllowReplacement.into();
        if dbus_proxy.request_name(NAME, flags).await? == RequestNameReply::InQueue {
            warn!("Bus name '{NAME}' already owned");
        } else {
            info!("Acquired notification daemon bus name");
        }

        Ok(connection)
    }

    pub async fn invoke_action(id: u32, action_key: String) -> anyhow::Result<()> {
        if let Some(connection) = NOTIFICATION_CONNECTION.get() {
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
        Ok(())
    }

    pub async fn close_notification_by_id(id: u32) -> anyhow::Result<()> {
        if let Some(connection) = NOTIFICATION_CONNECTION.get() {
            // Get the object server interface to call the method properly
            let iface_ref = connection
                .object_server()
                .interface::<_, NotificationDaemon>(OBJECT_PATH)
                .await?;

            // Call the close_notification method which properly cleans up both stores
            iface_ref.get_mut().await.close_notification(id).await;
        }
        Ok(())
    }
}
