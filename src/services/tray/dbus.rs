use iced::futures::StreamExt;
use log::{info, warn};
use std::time::Duration;
use zbus::{
    Connection, OwnedObjectPath, OwnedVariant, Result, Type,
    fdo::{DBusProxy, RequestNameFlags, RequestNameReply},
    interface,
    message::Header,
    names::{BusName, UniqueName, WellKnownName},
    object_server::{InterfaceRef, SignalEmitter},
    proxy,
};

const NAME: WellKnownName =
    WellKnownName::from_static_str_unchecked("org.kde.StatusNotifierWatcher");
const OBJECT_PATH: &str = "/StatusNotifierWatcher";
const ITEM_OBJECT_PATH: &str = "/StatusNotifierItem";

/// Clients grab their well-known name before exporting `/StatusNotifierItem`,
/// so re-probe briefly to avoid registering an item we cannot read yet.
const READY_PROBE_ATTEMPTS: usize = 6;
const READY_PROBE_INTERVAL: Duration = Duration::from_secs(1);

/// The spec's `org.kde.StatusNotifierItem-<pid>-<id>` form.
fn is_item_name(name: &str) -> bool {
    name.contains("StatusNotifierItem")
}

/// Only item names count: the app's own names reach the same object too.
fn probe_alias<'a>(name: &'a BusName<'_>) -> Option<&'a str> {
    match name {
        BusName::WellKnown(name) if is_item_name(name) => Some(name.as_str()),
        _ => None,
    }
}

#[derive(Debug)]
struct RegisteredItem {
    sender: UniqueName<'static>,
    path: String,
    /// As first announced: hosts match `StatusNotifierItemUnregistered` on it.
    service: String,
    /// Well-known names whose release removes the item.
    names: Vec<String>,
}

#[derive(Debug, Default)]
pub struct StatusNotifierWatcher {
    /// Keyed by (sender, path).
    items: Vec<RegisteredItem>,
    hosts: Vec<UniqueName<'static>>,
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

        // Never take the name from a running watcher; queue for it instead.
        if dbus_proxy
            .request_name(NAME, RequestNameFlags::ALLOW_REPLACEMENT)
            .await?
            == RequestNameReply::InQueue
        {
            info!("Another {NAME} is running, using it and waiting in line");
        }

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
                        // Our own host lives on this connection; count it now.
                        if let Some(unique_name) = internal_connection.unique_name() {
                            let unique_name = unique_name.to_owned().into_inner();
                            let mut interface = internal_interface.get_mut().await;
                            if !interface.hosts.contains(&unique_name) {
                                interface.hosts.push(unique_name);
                            }
                        }
                        if let Ok(emitter) = SignalEmitter::new(&internal_connection, OBJECT_PATH) {
                            let _ = Self::status_notifier_host_registered(&emitter).await;
                        }
                        // Finds items that do not re-register on their own.
                        let conn = internal_connection.clone();
                        let interface = internal_interface.clone();
                        tokio::spawn(async move {
                            if let Err(e) = Self::discover_items(&conn, &interface).await {
                                info!("Failed tray item discovery: {e}");
                            }
                        });
                    } else if have_bus_name {
                        info!("Lost bus name: {NAME}");
                        have_bus_name = false;
                        let mut interface = internal_interface.get_mut().await;
                        interface.items.clear();
                        interface.hosts.clear();
                    }
                    continue;
                }
                if !have_bus_name {
                    continue;
                }

                if args.old_owner.is_some() {
                    Self::remove_name(&internal_connection, &internal_interface, &args.name).await;
                }
                if args.new_owner.is_some() {
                    // Probe outside the loop so a slow owner cannot stall it.
                    let conn = internal_connection.clone();
                    let interface = internal_interface.clone();
                    let name = args.name.to_string();
                    tokio::spawn(async move {
                        Self::probe_and_register_name(&conn, &interface, &name).await;
                    });
                }
            }
        });

        Ok(connection)
    }

    /// Removes what `name` owned (see `remove_owner`) and announces it.
    async fn remove_name(
        conn: &Connection,
        interface: &InterfaceRef<StatusNotifierWatcher>,
        name: &BusName<'_>,
    ) {
        // Emit after releasing the guard.
        let removed = interface.get_mut().await.remove_owner(name);
        if removed.is_empty() {
            return;
        }

        let emitter = match SignalEmitter::new(conn, OBJECT_PATH) {
            Ok(e) => e,
            Err(e) => {
                warn!("Failed to create signal emitter: {e}");
                return;
            }
        };
        for service in removed {
            if let Err(e) = Self::status_notifier_item_unregistered(&emitter, &service).await {
                warn!("Failed to emit item_unregistered signal: {e}");
            }
        }
    }

    async fn discover_items(
        conn: &Connection,
        interface: &InterfaceRef<StatusNotifierWatcher>,
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
        interface: &InterfaceRef<StatusNotifierWatcher>,
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
        let sender = match dbus_proxy.get_name_owner(bus_name.clone()).await {
            Ok(owner) => owner,
            Err(_) => return,
        };
        let alias = probe_alias(&bus_name);
        // Already tracked: only remember the name.
        {
            let mut watcher = interface.get_mut().await;
            if watcher.find(&sender, ITEM_OBJECT_PATH).is_some() {
                watcher.add_item(ITEM_OBJECT_PATH, sender.into_inner(), alias);
                return;
            }
        }
        // Suffixed names are almost certainly SNI, but the object may not be
        // exported yet; retry only those to avoid a re-probe storm.
        let attempts = if is_item_name(name) {
            READY_PROBE_ATTEMPTS
        } else {
            1
        };
        for attempt in 0..attempts {
            if Self::is_status_notifier_item(conn, name).await {
                // Changed hands during the retries: the new owner gets its own probe.
                match dbus_proxy.get_name_owner(bus_name.clone()).await {
                    Ok(owner) if owner == sender => {}
                    _ => return,
                }
                let added = interface.get_mut().await.add_item(
                    ITEM_OBJECT_PATH,
                    sender.into_inner(),
                    alias,
                );
                if let Some(service) = added {
                    match SignalEmitter::new(conn, OBJECT_PATH) {
                        Ok(emitter) => Self::status_notifier_item_registered(&emitter, &service)
                            .await
                            .unwrap_or_else(|e| {
                                warn!("Failed to emit item_registered signal: {e}")
                            }),
                        Err(e) => warn!("Failed to create signal emitter: {e}"),
                    }
                }
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
        let builder = StatusNotifierItemProxy::builder(conn)
            .destination(name.to_owned())
            .path(ITEM_OBJECT_PATH)
            .cache_properties(zbus::proxy::CacheProperties::No);
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
    fn find(&self, sender: &UniqueName<'_>, path: &str) -> Option<&RegisteredItem> {
        self.items
            .iter()
            .find(|i| &i.sender == sender && i.path == path)
    }

    /// Returns the service to announce, or `None` if already tracked.
    fn add_item(
        &mut self,
        service: &str,
        sender: UniqueName<'static>,
        alias: Option<&str>,
    ) -> Option<String> {
        let (path, announced) = if service.starts_with('/') {
            (service.to_owned(), format!("{sender}{service}"))
        } else {
            (ITEM_OBJECT_PATH.to_owned(), service.to_owned())
        };
        let well_known = (!service.starts_with('/') && !service.starts_with(':'))
            .then_some(service)
            .into_iter()
            .chain(alias);

        if let Some(item) = self
            .items
            .iter_mut()
            .find(|i| i.sender == sender && i.path == path)
        {
            for name in well_known {
                if !item.names.iter().any(|n| n == name) {
                    item.names.push(name.to_owned());
                }
            }
            return None;
        }

        self.items.push(RegisteredItem {
            sender,
            path,
            service: announced.clone(),
            names: well_known.map(str::to_owned).collect(),
        });
        Some(announced)
    }

    /// Drops what `name` owned; returns the services to announce.
    fn remove_owner(&mut self, name: &BusName<'_>) -> Vec<String> {
        let removed: Vec<RegisteredItem> = match name {
            BusName::Unique(name) => {
                self.hosts.retain(|host| host != name);
                self.items.extract_if(.., |i| &i.sender == name).collect()
            }
            BusName::WellKnown(name) => self
                .items
                .extract_if(.., |i| i.names.iter().any(|n| n == name.as_str()))
                .collect(),
        };
        removed.into_iter().map(|i| i.service).collect()
    }
}

#[interface(
    name = "org.kde.StatusNotifierWatcher",
    proxy(
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
        if let Some(service) = self.add_item(service, sender, None) {
            Self::status_notifier_item_registered(&emitter, &service)
                .await
                .unwrap_or_else(|e| warn!("Failed to emit item_registered signal: {e}"));
        }
    }

    async fn register_status_notifier_host(
        &mut self,
        _service: &str,
        #[zbus(header)] header: Header<'_>,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) {
        // Once per host: emitting on every re-registration would loop.
        let sender = match header.sender() {
            Some(s) => s.to_owned(),
            None => {
                warn!("D-Bus message has no sender");
                return;
            }
        };
        if self.hosts.contains(&sender) {
            return;
        }
        self.hosts.push(sender);
        let _ = Self::status_notifier_host_registered(&emitter).await;
    }

    #[zbus(property)]
    fn registered_status_notifier_items(&self) -> Vec<String> {
        self.items.iter().map(|i| i.service.clone()).collect()
    }

    #[zbus(property)]
    fn is_status_notifier_host_registered(&self) -> bool {
        !self.hosts.is_empty()
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

#[derive(Clone, Debug, serde::Deserialize, Type)]
pub struct Icon {
    pub width: i32,
    pub height: i32,
    pub bytes: Vec<u8>,
}

// The generated getters decode an `ay` into one `Variant` per byte, 64 bytes
// each: a 256x256 pixmap peaked at ~42 MB. These read it straight into `Icon`.
impl StatusNotifierItemProxy<'_> {
    pub async fn icon_pixmap(&self) -> Result<Vec<Icon>> {
        self.pixmap("IconPixmap").await
    }

    pub async fn attention_icon_pixmap(&self) -> Result<Vec<Icon>> {
        self.pixmap("AttentionIconPixmap").await
    }

    async fn pixmap(&self, property: &str) -> Result<Vec<Icon>> {
        let proxy = self.inner();
        let reply = proxy
            .connection()
            .call_method(
                Some(proxy.destination()),
                proxy.path(),
                Some("org.freedesktop.DBus.Properties"),
                "Get",
                &(proxy.interface(), property),
            )
            .await?;
        let body = reply.body();
        let icons: zbus::as_variant::Deserialize<Vec<Icon>> = body.deserialize()?;
        Ok(icons.0)
    }
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
    fn menu(&self) -> zbus::Result<OwnedObjectPath>;

    #[zbus(property)]
    fn status(&self) -> zbus::Result<String>;

    #[zbus(property)]
    fn attention_icon_name(&self) -> zbus::Result<String>;

    #[zbus(signal)]
    async fn new_icon(&self) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn new_attention_icon(&self) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn new_status(&self, status: &str) -> zbus::Result<()>;

    fn activate(&self, x: i32, y: i32) -> zbus::Result<()>;
}

#[derive(Clone, Debug, Type)]
#[zbus(signature = "(ia{sv}av)")]
pub struct Layout(pub i32, pub LayoutProps, pub Vec<Layout>);

impl<'a> serde::Deserialize<'a> for Layout {
    fn deserialize<D: serde::Deserializer<'a>>(
        deserializer: D,
    ) -> std::result::Result<Self, D::Error> {
        let (id, props, children) =
            <(i32, LayoutProps, Vec<(zbus::Signature, Self)>)>::deserialize(deserializer)?;
        Ok(Self(id, props, children.into_iter().map(|x| x.1).collect()))
    }
}

#[derive(Clone, Debug, Type, zbus::DeserializeDict)]
#[zbus(signature = "dict")]
pub struct LayoutProps {
    #[zbus(rename = "children-display")]
    pub children_display: Option<String>,
    pub label: Option<String>,
    #[zbus(rename = "type")]
    pub type_: Option<String>,
    #[zbus(rename = "toggle-type")]
    pub toggle_type: Option<String>,
    #[zbus(rename = "toggle-state")]
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

    fn event(
        &self,
        id: i32,
        event_id: &str,
        data: &OwnedVariant,
        timestamp: u32,
    ) -> zbus::Result<()>;

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
    use super::{StatusNotifierWatcher, probe_alias, split_service_name};
    use zbus::names::{BusName, UniqueName, WellKnownName};

    #[test]
    fn protocol_version_is_zero() {
        let watcher = StatusNotifierWatcher::default();
        assert_eq!(watcher.protocol_version(), 0);
    }

    #[test]
    fn no_host_is_registered_initially() {
        let watcher = StatusNotifierWatcher::default();
        assert!(!watcher.is_status_notifier_host_registered());
    }

    #[test]
    fn host_is_registered_once_tracked() {
        let mut watcher = StatusNotifierWatcher::default();
        watcher
            .hosts
            .push(UniqueName::from_static_str_unchecked(":1.7"));
        assert!(watcher.is_status_notifier_host_registered());
    }

    fn unique(name: &'static str) -> UniqueName<'static> {
        UniqueName::from_static_str_unchecked(name)
    }

    fn well_known(name: &'static str) -> BusName<'static> {
        BusName::WellKnown(WellKnownName::from_static_str_unchecked(name))
    }

    #[test]
    fn items_are_keyed_by_sender_and_path() {
        let mut watcher = StatusNotifierWatcher::default();
        assert_eq!(
            watcher.add_item("/StatusNotifierItem", unique(":1.42"), None),
            Some(":1.42/StatusNotifierItem".to_owned())
        );
        // Same connection, second object: a separate item.
        assert_eq!(
            watcher.add_item("/org/blueman/sni", unique(":1.42"), None),
            Some(":1.42/org/blueman/sni".to_owned())
        );
        // Same item again by its well-known name: not announced twice.
        assert_eq!(
            watcher.add_item("org.kde.StatusNotifierItem-42-1", unique(":1.42"), None),
            None
        );
        assert_eq!(
            watcher.registered_status_notifier_items(),
            vec![
                ":1.42/StatusNotifierItem".to_owned(),
                ":1.42/org/blueman/sni".to_owned(),
            ]
        );
    }

    #[test]
    fn closed_connection_removes_all_its_items_and_hosts() {
        let mut watcher = StatusNotifierWatcher::default();
        watcher.add_item("/StatusNotifierItem", unique(":1.42"), None);
        watcher.add_item("/org/blueman/sni", unique(":1.42"), None);
        watcher.add_item("/StatusNotifierItem", unique(":1.43"), None);
        watcher.hosts.push(unique(":1.42"));

        let removed = watcher.remove_owner(&BusName::Unique(unique(":1.42")));

        assert_eq!(
            removed,
            vec![
                ":1.42/StatusNotifierItem".to_owned(),
                ":1.42/org/blueman/sni".to_owned(),
            ]
        );
        assert_eq!(
            watcher.registered_status_notifier_items(),
            vec![":1.43/StatusNotifierItem".to_owned()]
        );
        assert!(!watcher.is_status_notifier_host_registered());
    }

    #[test]
    fn released_well_known_name_removes_its_item() {
        let mut watcher = StatusNotifierWatcher::default();
        watcher.add_item("org.kde.StatusNotifierItem-42-1", unique(":1.42"), None);
        watcher.add_item("/StatusNotifierItem", unique(":1.43"), None);

        let removed = watcher.remove_owner(&well_known("org.kde.StatusNotifierItem-42-1"));

        assert_eq!(removed, vec!["org.kde.StatusNotifierItem-42-1".to_owned()]);
        assert_eq!(
            watcher.registered_status_notifier_items(),
            vec![":1.43/StatusNotifierItem".to_owned()]
        );
    }

    #[test]
    fn later_well_known_name_removes_item_but_keeps_announced_service() {
        // Found by path, then registered by name.
        let mut watcher = StatusNotifierWatcher::default();
        watcher.add_item("/StatusNotifierItem", unique(":1.42"), None);
        watcher.add_item("org.kde.StatusNotifierItem-42-1", unique(":1.42"), None);

        let removed = watcher.remove_owner(&well_known("org.kde.StatusNotifierItem-42-1"));

        assert_eq!(removed, vec![":1.42/StatusNotifierItem".to_owned()]);
        assert!(watcher.registered_status_notifier_items().is_empty());
    }

    #[test]
    fn probe_alias_removes_item_on_release() {
        let mut watcher = StatusNotifierWatcher::default();
        watcher.add_item(
            "/StatusNotifierItem",
            unique(":1.42"),
            Some("org.kde.StatusNotifierItem-42-1"),
        );

        let removed = watcher.remove_owner(&well_known("org.kde.StatusNotifierItem-42-1"));

        assert_eq!(removed, vec![":1.42/StatusNotifierItem".to_owned()]);
    }

    #[test]
    fn only_item_names_become_probe_aliases() {
        let item = well_known("org.kde.StatusNotifierItem-42-1");
        assert_eq!(probe_alias(&item), Some("org.kde.StatusNotifierItem-42-1"));
        assert_eq!(probe_alias(&well_known("org.example.App")), None);
        assert_eq!(probe_alias(&BusName::Unique(unique(":1.42"))), None);
    }

    #[test]
    fn unrelated_name_removes_nothing() {
        let mut watcher = StatusNotifierWatcher::default();
        watcher.add_item("/StatusNotifierItem", unique(":1.42"), None);

        assert!(
            watcher
                .remove_owner(&well_known("org.example.Other"))
                .is_empty()
        );
        assert!(
            watcher
                .remove_owner(&BusName::Unique(unique(":1.99")))
                .is_empty()
        );
        assert_eq!(watcher.registered_status_notifier_items().len(), 1);
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
