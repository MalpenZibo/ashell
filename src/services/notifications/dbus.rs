use log::{debug, info, warn};
use std::collections::HashMap;
use std::time::SystemTime;
use zbus::{
    Connection,
    fdo::{DBusProxy, RequestNameFlags, RequestNameReply},
    interface,
    names::WellKnownName,
    zvariant::{self, OwnedValue},
};

// Import the static from the notifications module
use crate::modules::notifications::NOTIFICATIONS;

pub static NOTIFICATION_CONNECTION: std::sync::OnceLock<Connection> = std::sync::OnceLock::new();

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
            let mut global_notifications = crate::modules::notifications::NOTIFICATIONS
                .get_or_init(|| std::sync::Mutex::new(std::collections::HashMap::new()))
                .lock()
                .unwrap();
            global_notifications.insert(id, notification.clone());
        }
        debug!("New notification: {:?}", notification);

        // Emit signal
        // self.notification_received(id, notification).await.unwrap();

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
            // Emit signal for notification closed (reason 3 = closed by call to CloseNotification)
            // let _ = self.notification_closed(id, 3).await;
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

    // #[zbus(signal)]
    // async fn notification_closed(&self, id: u32, reason: u32) -> zbus::Result<()>;

    // #[zbus(signal)]
    // async fn action_invoked(&self, id: u32, action_key: String) -> zbus::Result<()>;
}

impl NotificationDaemon {
    pub async fn start_server() -> anyhow::Result<Connection> {
        let connection = zbus::connection::Connection::session().await?;
        let daemon = NotificationDaemon::default();
        connection.object_server().at(OBJECT_PATH, daemon).await?;

        NOTIFICATION_CONNECTION.set(connection.clone()).unwrap();

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
}
