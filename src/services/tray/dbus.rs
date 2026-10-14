use iced::futures::StreamExt;
use log::{info, warn};
use std::time::Duration;
use zbus::{
    Connection, Result,
    fdo::{DBusProxy, RequestNameFlags, RequestNameReply},
    interface,
    message::Header,
    names::{BusName, UniqueName, WellKnownName},
    object_server::SignalEmitter,
    proxy,
    zvariant::{self, OwnedObjectPath, OwnedValue, Type},
};

const NAME: WellKnownName =
    WellKnownName::from_static_str_unchecked("org.kde.StatusNotifierWatcher");
const OBJECT_PATH: &str = "/StatusNotifierWatcher";

/// Clients grab their well-known name before exporting `/StatusNotifierItem`,
/// so re-probe briefly to avoid registering an item we cannot read yet.
const READY_PROBE_ATTEMPTS: usize = 6;
const READY_PROBE_INTERVAL: Duration = Duration::from_secs(1);

#[derive(Debug, Default)]
pub struct StatusNotifierWatcher {
    items: Vec<(UniqueName<'static>, String)>,
    /// Tracked so `StatusNotifierHostRegistered` is emitted only once.
    host_name: Option<UniqueName<'static>>,
}

impl StatusNotifierWatcher {
    pub async fn start_server() -> anyhow::Result<Connection> {
        let connection = zbus::connection::Connection::session().await?;
        let watcher = StatusNotifierWatcher::default();
        connection.object_server().at(OBJECT_PATH, watcher).await?;
        let interface = connection
            .object_server()
            .interface::<_, StatusNotifierWatcher>(OBJECT_PATH)
            .await?;

        let dbus_proxy = DBusProxy::new(&connection).await?;
        let name_owner_changed_stream = dbus_proxy.receive_name_owner_changed().await?;

        let flags = RequestNameFlags::AllowReplacement | RequestNameFlags::ReplaceExisting;
        if dbus_proxy.request_name(NAME, flags).await? == RequestNameReply::InQueue {
            warn!("Bus name '{NAME}' already owned");
        }
        let emitter = SignalEmitter::new(&connection, OBJECT_PATH)?;
        Self::status_notifier_host_registered(&emitter).await?;

        let internal_connection = connection.clone();
        let internal_interface = interface.clone();
        tokio::spawn(async move {
            let mut have_bus_name = false;
            let unique_name = internal_connection.unique_name().map(|x| x.as_ref());

            let mut name_owner_changed_stream = name_owner_changed_stream.fuse();

            while let Some(evt) = name_owner_changed_stream.next().await {
                let args = match evt.args() {
                    Ok(args) => args,
                    Err(_) => continue,
                };
                if args.name.as_ref() == NAME {
                    if args.new_owner.as_ref() == unique_name.as_ref() {
                        info!("Acquired bus name: {NAME}");
                        have_bus_name = true;
                    } else if have_bus_name {
                        info!("Lost bus name: {NAME}");
                        have_bus_name = false;
                    }
                } else if args.new_owner.as_ref().is_none() {
                    if let BusName::Unique(name) = &args.name {
                        let mut interface = internal_interface.get_mut().await;
                        if let Some(idx) = interface
                            .items
                            .iter()
                            .position(|(unique_name, _)| unique_name == name)
                        {
                            let emitter =
                                match SignalEmitter::new(&internal_connection, OBJECT_PATH) {
                                    Ok(e) => e,
                                    Err(e) => {
                                        warn!("Failed to create signal emitter: {e}");
                                        continue;
                                    }
                                };
                            let service = interface.items.remove(idx).1;
                            if let Err(e) =
                                StatusNotifierWatcher::status_notifier_item_unregistered(
                                    &emitter, &service,
                                )
                                .await
                            {
                                warn!("Failed to emit item_unregistered signal: {e}");
                            }
                        }
                    }
                } else {
                    // Probe outside the loop so a slow owner cannot stall
                    // signal handling; dedup on sender makes this race-safe.
                    let conn = internal_connection.clone();
                    let interface = internal_interface.clone();
                    let name = args.name.to_string();
                    tokio::spawn(async move {
                        Self::probe_and_register_name(&conn, &interface, &name).await;
                    });
                }
            }
        });

        if let Err(e) = Self::discover_items(&connection, &interface).await {
            info!("Failed initial tray item discovery: {e}");
        }

        Ok(connection)
    }

    async fn discover_items(
        conn: &Connection,
        interface: &zbus::object_server::InterfaceRef<StatusNotifierWatcher>,
    ) -> anyhow::Result<()> {
        let dbus_proxy = DBusProxy::new(conn).await?;
        let names = dbus_proxy.list_names().await?;

        // Probe concurrently so a slow name cannot block the loop; unique
        // names are included because Qt/GTK/Chromium register under them.
        let futures = names
            .into_iter()
            .filter(|name| {
                let s = name.as_str();
                s != NAME.as_str() && s != "org.freedesktop.DBus"
            })
            .map(|name| {
                let conn = conn.clone();
                let interface = interface.clone();
                async move {
                    Self::probe_and_register_name(&conn, &interface, name.as_str()).await;
                }
            });
        iced::futures::future::join_all(futures).await;
        Ok(())
    }

    async fn probe_and_register_name(
        conn: &Connection,
        interface: &zbus::object_server::InterfaceRef<StatusNotifierWatcher>,
        name: &str,
    ) {
        let dbus_proxy = match DBusProxy::new(conn).await {
            Ok(p) => p,
            Err(_) => return,
        };
        let bus_name = match BusName::try_from(name) {
            Ok(n) => n,
            Err(_) => return,
        };
        let sender = match dbus_proxy.get_name_owner(bus_name).await {
            Ok(owner) => owner,
            Err(_) => return,
        };
        // Already tracked → do not re-emit `StatusNotifierItemRegistered`.
        let tracked: std::collections::HashSet<String> = interface
            .get()
            .await
            .items
            .iter()
            .map(|(s, _)| s.as_str().to_owned())
            .collect();
        if tracked.contains(sender.as_str()) {
            return;
        }
        // Suffixed names are almost certainly SNI, but the object may not be
        // exported yet; retry only those to avoid a re-probe storm.
        let attempts = if name.contains("StatusNotifierItem") {
            READY_PROBE_ATTEMPTS
        } else {
            1
        };
        for attempt in 0..attempts {
            if Self::is_status_notifier_item(conn, name).await {
                let emitter = match SignalEmitter::new(conn, OBJECT_PATH) {
                    Ok(e) => e,
                    Err(e) => {
                        warn!("Failed to create signal emitter: {e}");
                        return;
                    }
                };
                let mut watcher = interface.get_mut().await;
                watcher
                    .register_status_notifier_item_manual(
                        "/StatusNotifierItem",
                        sender.into_inner(),
                        &emitter,
                    )
                    .await;
                return;
            }
            if attempt + 1 < attempts {
                tokio::time::sleep(READY_PROBE_INTERVAL).await;
            }
        }
    }

    /// Verify the object instead of trusting the name suffix: a client may
    /// grab its name before exporting `/StatusNotifierItem`, and clients
    /// without the suffix (Qt/GTK/Chromium) must still be found.
    async fn is_status_notifier_item(conn: &Connection, name: &str) -> bool {
        // Disable property caching: `GetAll` at build time logs a WARN for
        // names that have not exported `/StatusNotifierItem` (yet).
        let builder = match StatusNotifierItemProxy::builder(conn)
            .destination(name.to_owned())
            .and_then(|b| b.path("/StatusNotifierItem"))
        {
            Ok(b) => b,
            Err(_) => return false,
        };
        let builder = builder.cache_properties(zbus::proxy::CacheProperties::No);
        match builder.build().await {
            Ok(proxy) => tokio::time::timeout(Duration::from_millis(500), proxy.icon_name())
                .await
                .map(|r| r.is_ok())
                .unwrap_or(false),
            Err(_) => false,
        }
    }
}

impl StatusNotifierWatcher {
    async fn register_status_notifier_item_manual(
        &mut self,
        service: &str,
        sender: UniqueName<'static>,
        emitter: &SignalEmitter<'_>,
    ) {
        if self.items.iter().any(|(s, _)| s == &sender) {
            return;
        }

        let service = if service.starts_with('/') {
            format!("{sender}{service}")
        } else {
            service.to_string()
        };

        Self::status_notifier_item_registered(emitter, &service)
            .await
            .unwrap_or_else(|e| warn!("Failed to emit item_registered signal: {e}"));

        self.items.push((sender, service));
    }
}

#[interface(
    name = "org.kde.StatusNotifierWatcher",
    proxy(
        gen_blocking = false,
        default_service = "org.kde.StatusNotifierWatcher",
        default_path = "/StatusNotifierWatcher",
    )
)]
impl StatusNotifierWatcher {
    async fn register_status_notifier_item(
        &mut self,
        service: &str,
        #[zbus(header)] header: Header<'_>,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) {
        let sender = match header.sender() {
            Some(s) => s.to_owned(),
            None => {
                warn!("D-Bus message has no sender");
                return;
            }
        };
        self.register_status_notifier_item_manual(service, sender, &emitter)
            .await;
    }

    async fn register_status_notifier_host(
        &mut self,
        _service: &str,
        #[zbus(header)] header: Header<'_>,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) {
        // Guard so `StatusNotifierHostRegistered` is emitted only once;
        // emitting on every (re-)registration would loop.
        let sender = match header.sender() {
            Some(s) => s.to_owned(),
            None => {
                warn!("D-Bus message has no sender");
                return;
            }
        };
        if self.host_name.replace(sender).is_some() {
            return;
        }
        let _ = Self::status_notifier_host_registered(&emitter).await;
    }

    #[zbus(property)]
    fn registered_status_notifier_items(&self) -> Vec<String> {
        self.items.iter().map(|(_, x)| x.clone()).collect()
    }

    #[zbus(property)]
    fn is_status_notifier_host_registered(&self) -> bool {
        true
    }

    #[zbus(property)]
    fn protocol_version(&self) -> i32 {
        0
    }

    #[zbus(signal)]
    async fn status_notifier_item_registered(
        emitter: &SignalEmitter<'_>,
        service: &str,
    ) -> Result<()>;

    #[zbus(signal)]
    async fn status_notifier_item_unregistered(
        emitter: &SignalEmitter<'_>,
        service: &str,
    ) -> Result<()>;

    #[zbus(signal)]
    async fn status_notifier_host_registered(emitter: &SignalEmitter<'_>) -> Result<()>;

    #[zbus(signal)]
    async fn status_notifier_host_unregistered(emitter: &SignalEmitter<'_>) -> Result<()>;
}

#[derive(Clone, Debug, zvariant::Value)]
pub struct Icon {
    pub width: i32,
    pub height: i32,
    pub bytes: Vec<u8>,
}

/// Service names are `<unique-sender>/<object-path>` for method-registered
/// items, or a bare path; split off the sender.
pub(crate) fn split_service_name(name: &str) -> (&str, &str) {
    match name.find('/') {
        Some(idx) => (&name[..idx], &name[idx..]),
        None => (name, "/StatusNotifierItem"),
    }
}

#[proxy(interface = "org.kde.StatusNotifierItem")]
pub trait StatusNotifierItem {
    #[zbus(property)]
    fn icon_name(&self) -> zbus::Result<String>;

    #[zbus(property)]
    fn icon_pixmap(&self) -> zbus::Result<Vec<Icon>>;

    #[zbus(property)]
    fn menu(&self) -> zbus::Result<OwnedObjectPath>;

    #[zbus(property)]
    fn status(&self) -> zbus::Result<String>;

    #[zbus(property)]
    fn attention_icon_name(&self) -> zbus::Result<String>;

    #[zbus(property)]
    fn attention_icon_pixmap(&self) -> zbus::Result<Vec<Icon>>;

    #[zbus(signal)]
    async fn new_icon(&self) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn new_attention_icon(&self) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn new_status(&self, status: &str) -> zbus::Result<()>;

    fn activate(&self, x: i32, y: i32) -> zbus::Result<()>;
}

#[derive(Clone, Debug, Type)]
#[zvariant(signature = "(ia{sv}av)")]
pub struct Layout(pub i32, pub LayoutProps, pub Vec<Layout>);

impl<'a> serde::Deserialize<'a> for Layout {
    fn deserialize<D: serde::Deserializer<'a>>(
        deserializer: D,
    ) -> std::result::Result<Self, D::Error> {
        let (id, props, children) =
            <(i32, LayoutProps, Vec<(zvariant::Signature, Self)>)>::deserialize(deserializer)?;
        Ok(Self(id, props, children.into_iter().map(|x| x.1).collect()))
    }
}

#[derive(Clone, Debug, Type, zvariant::DeserializeDict)]
#[zvariant(signature = "dict")]
pub struct LayoutProps {
    #[zvariant(rename = "children-display")]
    pub children_display: Option<String>,
    pub label: Option<String>,
    #[zvariant(rename = "type")]
    pub type_: Option<String>,
    #[zvariant(rename = "toggle-type")]
    pub toggle_type: Option<String>,
    #[zvariant(rename = "toggle-state")]
    pub toggle_state: Option<i32>,
    pub visible: Option<bool>,
}

#[proxy(interface = "com.canonical.dbusmenu")]
pub trait DBusMenu {
    fn get_layout(
        &self,
        parent_id: i32,
        recursion_depth: i32,
        property_names: &[&str],
    ) -> zbus::Result<(u32, Layout)>;

    fn event(&self, id: i32, event_id: &str, data: &OwnedValue, timestamp: u32)
    -> zbus::Result<()>;

    fn about_to_show(&self, id: i32) -> zbus::Result<bool>;

    #[zbus(signal)]
    fn layout_updated(&self, revision: u32, parent: i32) -> zbus::Result<()>;

    /// dbusmenu signal: the menu layout changed, so refetch it.
    #[zbus(signal)]
    fn items_removed(
        &self,
        revision: u32,
        path: String,
        parent: i32,
        item_ids: Vec<i32>,
    ) -> zbus::Result<()>;
}

#[cfg(test)]
mod tests {
    use super::{StatusNotifierWatcher, split_service_name};

    #[test]
    fn protocol_version_is_zero() {
        let watcher = StatusNotifierWatcher::default();
        assert_eq!(watcher.protocol_version(), 0);
    }

    #[test]
    fn host_is_reported_registered() {
        let watcher = StatusNotifierWatcher::default();
        assert!(watcher.is_status_notifier_host_registered());
    }

    #[test]
    fn split_service_name_splits_unique_sender_from_path() {
        let (sender, path) = split_service_name(":1.131/StatusNotifierItem");
        assert_eq!(sender, ":1.131");
        assert_eq!(path, "/StatusNotifierItem");
    }

    #[test]
    fn split_service_name_splits_at_first_slash() {
        let (sender, path) = split_service_name(":1.283/org/blueman/sni");
        assert_eq!(sender, ":1.283");
        assert_eq!(path, "/org/blueman/sni");
    }

    #[test]
    fn split_service_name_defaults_path_for_bare_well_known_name() {
        let (sender, path) = split_service_name("org.kde.StatusNotifierItem-123");
        assert_eq!(sender, "org.kde.StatusNotifierItem-123");
        assert_eq!(path, "/StatusNotifierItem");
    }

    #[test]
    fn split_service_name_handles_bare_path() {
        let (sender, path) = split_service_name("/StatusNotifierItem");
        assert_eq!(sender, "");
        assert_eq!(path, "/StatusNotifierItem");
    }
}
