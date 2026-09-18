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

#[derive(Debug, Default)]
pub struct StatusNotifierWatcher {
    items: Vec<(UniqueName<'static>, String)>,
    /// Name of the host service that registered itself via
    /// `RegisterStatusNotifierHost`. Tracked so we can emit
    /// `StatusNotifierHostRegistered` exactly once (not in a loop).
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
            // Continuous drain loop: probe well-known names every 3s (not 30s).
            // 30s is too slow for late-start — a client started after ashell
            // would take up to 30s to appear.
            let mut interval = tokio::time::interval(Duration::from_secs(3));

            loop {
                tokio::select! {
                    Some(evt) = name_owner_changed_stream.next() => {
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
                        } else if let BusName::Unique(name) = &args.name {
                            let mut interface = internal_interface.get_mut().await;
                            if let Some(idx) = interface
                                .items
                                .iter()
                                .position(|(unique_name, _)| unique_name == name)
                            {
                                let emitter = match
                                    SignalEmitter::new(&internal_connection, OBJECT_PATH)
                                {
                                    Ok(e) => e,
                                    Err(e) => {
                                        warn!("Failed to create signal emitter: {e}");
                                        continue;
                                    }
                                };
                                let service = interface.items.remove(idx).1;
                                if let Err(e) = StatusNotifierWatcher::status_notifier_item_unregistered(
                                    &emitter, &service,
                                )
                                .await {
                                    warn!("Failed to emit item_unregistered signal: {e}");
                                }
                            }
                        }
                    }
                    _ = interval.tick() => {
                        if let Err(e) =
                            Self::discover_items(&internal_connection, &internal_interface)
                                .await
                        {
                            info!("Failed to discover tray items: {e}");
                        }
                    }
                }
            }
        });

        // Initial discovery
        if let Err(e) = Self::discover_items(&connection, &interface).await {
            info!("Failed initial tray item discovery: {e}");
        }

        Ok(connection)
    }

    /// Re-sync the watcher's tracked items from the watcher's own
    /// `RegisteredItems` property. This is the key fix for late-start:
    /// a client that registered via the `RegisterStatusNotifierItem`
    /// method *before* this watcher existed (or before it re-armed)
    /// emits `StatusNotifierItemRegistered` only once, so a
    /// signal-only consumer misses it. Re-reading `RegisteredItems`
    /// on each discovery tick makes the watcher authoritative and
    /// self-healing, mirroring what a restart does — without requiring
    /// the user to restart the application.
    ///
    /// Each service in `RegisteredItems` is of the form
    /// `<unique-sender>/<object-path>` (for method-registered items) or
    /// a bare path (for well-known-name registrations). We split on the
    /// first `/` to recover the unique sender and dedup against the
    /// items already tracked in `self.items`.
    async fn sync_registered_items(
        conn: &Connection,
        interface: &zbus::object_server::InterfaceRef<StatusNotifierWatcher>,
    ) {
        let mut watcher = interface.get_mut().await;
        let registered = watcher.registered_status_notifier_items();

        let emitter = match SignalEmitter::new(conn, OBJECT_PATH) {
            Ok(e) => e,
            Err(e) => {
                warn!("Failed to create signal emitter: {e}");
                return;
            }
        };

        for service in &registered {
            let (sender, _path) = split_service_name(service);
            // Skip items already tracked by `self.items` (registered via
            // the method or the well-known-name probe path) so we don't
            // re-emit `StatusNotifierItemRegistered` for the same item.
            if watcher.items.iter().any(|(s, _)| s.as_ref() == sender) {
                continue;
            }
            let _ = StatusNotifierWatcher::status_notifier_item_registered(&emitter, service).await;
            // The sender is a unique name like `:1.131`; build a
            // `UniqueName<'static>` from the borrowed string.
            let sender = match UniqueName::try_from(sender.to_owned()) {
                Ok(s) => s,
                Err(_) => continue,
            };
            watcher.items.push((sender, service.clone()));
        }
    }

    async fn discover_items(
        conn: &Connection,
        interface: &zbus::object_server::InterfaceRef<StatusNotifierWatcher>,
    ) -> anyhow::Result<()> {
        // Re-sync from the watcher's `RegisteredItems` first: catches
        // clients that registered via the `RegisterStatusNotifierItem`
        // method but whose signal we missed (e.g. they registered
        // before this watcher existed). Without this, such items only
        // appear after a restart.
        Self::sync_registered_items(conn, interface).await;

        let dbus_proxy = DBusProxy::new(conn).await?;
        let names = dbus_proxy.list_names().await?;

        // Snapshot of tracked senders (owned) so we can dedup both
        // well-known and unique-name probes below without borrowing
        // `interface` across the concurrent probes.
        let tracked: std::collections::HashSet<String> = interface
            .get()
            .await
            .items
            .iter()
            .map(|(s, _)| s.as_str().to_owned())
            .collect();

        // Probe every name (well-known AND unique) concurrently so a slow
        // or unresponsive name does not block the watcher's event loop.
        // Probing unique names is essential: Qt/GTK/Chromium SNI clients
        // (telegram, nextcloud, ...) register via the
        // `RegisterStatusNotifierItem` method and live under a unique
        // name; their item may not be reachable from the well-known
        // name, and a missed registration signal means the item only
        // appeared after a restart. Concurrent probes keep the 3s tick
        // bounded regardless of how many names are on the bus.
        let futures = names
            .into_iter()
            .filter(|name| {
                let s = name.as_str();
                s != NAME.as_str() && s != "org.freedesktop.DBus"
            })
            .map(|name| {
                let conn = conn.clone();
                let dbus_proxy = dbus_proxy.clone();
                let interface = interface.clone();
                let tracked = tracked.clone();
                async move {
                    if !Self::is_status_notifier_item(&conn, name.as_str()).await {
                        return;
                    }
                    let sender = match dbus_proxy.get_name_owner(BusName::from(name.clone())).await
                    {
                        Ok(owner) => owner,
                        Err(_) => return,
                    };
                    // Skip if already tracked (by the method handler, the
                    // `RegisteredItems` sync, or a previous probe) to
                    // avoid re-emitting `StatusNotifierItemRegistered`.
                    if tracked.contains(sender.as_str()) {
                        return;
                    }
                    let emitter = match SignalEmitter::new(&conn, OBJECT_PATH) {
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
                }
            });
        iced::futures::future::join_all(futures).await;
        Ok(())
    }

    /// Probe whether a well-known name exposes an `org.kde.StatusNotifierItem`
    /// interface. Some SNI clients (Qt `QSystemTrayIcon`, GTK `GtkStatusIcon`,
    /// Chromium/CEF) register the item under a well-known name that does *not*
    /// contain the `StatusNotifierItem` suffix, so a suffix-only match misses
    /// them. We probe the standard object path `/StatusNotifierItem` with a
    /// short timeout (500ms) so discovery stays fast for late-start.
    async fn is_status_notifier_item(conn: &Connection, name: &str) -> bool {
        // Fast path: name contains the suffix (most clients, e.g.
        // `org.kde.StatusNotifierItem-<pid>-<n>` or `org.gnome.StatusNotifierItem`).
        if name.contains("StatusNotifierItem") {
            return true;
        }

        // Slow path: probe the standard object path for the SNI interface.
        // A single property read (`IconName`) confirms the item exists.
        // Disable property caching so zbus does not call `GetAll` on
        // `/StatusNotifierItem` at `build()` time — that emits a WARN
        // ("Object does not exist at path /StatusNotifierItem") on every
        // 3s discovery tick for names that do not yet (or no longer)
        // expose the object. The probe only needs a single method
        // call (`icon_name`), which works without a cached property.
        let builder = match StatusNotifierItemProxy::builder(conn)
            .destination(name.to_owned())
            .and_then(|b| b.path("/StatusNotifierItem"))
        {
            Ok(b) => b,
            Err(_) => return false,
        };
        // `.cache_properties` is infallible (returns `Builder`, not `Result`);
        // call it before `.build()` so the `GetAll` side-effect never fires.
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
        // A host registers itself with the watcher. Save its name and emit
        // the `StatusNotifierHostRegistered` signal exactly once. Emitting it
        // on every call would loop (clients re-register), so guard on
        // `host_name`.
        let sender = match header.sender() {
            Some(s) => s.to_owned(),
            None => {
                warn!("D-Bus message has no sender");
                return;
            }
        };
        if self.host_name.replace(sender).is_some() {
            // already registered a host; ignore subsequent registrations
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
        // `1` is the protocol version clients check for (`> 0`). Returning
        // `0` may be interpreted as "unsupported" by some SNI clients.
        1
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

impl Icon {
    /// Pixel count (width * height). Used to pick the largest pixmap.
    fn pixel_count(&self) -> u32 {
        (self.width.max(0) as u32)
            .checked_mul(self.height.max(0) as u32)
            .unwrap_or(0)
    }
}

/// Pick the largest icon (by pixel count) from a set of pixmaps. SNI clients
/// expose `IconPixmap` as an array of pixmaps at multiple resolutions; the
/// largest is preferred for rendering.
pub fn best_icon_pixmap(icons: &[Icon]) -> Option<&Icon> {
    icons.iter().max_by_key(|i| i.pixel_count())
}

/// Split a registered service name into its unique sender and object path.
/// A method-registered item is `<unique-sender>/<object-path>`; a
/// well-known-name registration is a bare `<object-path>`.
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

    /// SNI `Category`: `ApplicationStatus` / `Communications` /
    /// `SystemServices` / `Hardware`.
    #[zbus(property)]
    fn category(&self) -> zbus::Result<String>;

    /// SNI `Status`: `Passive` / `Active` / `NeedsAttention`.
    #[zbus(property)]
    fn status(&self) -> zbus::Result<String>;

    /// SNI `Title` (tooltip title).
    #[zbus(property)]
    fn title(&self) -> zbus::Result<String>;

    /// SNI `Id`: stable application id.
    #[zbus(property)]
    fn id(&self) -> zbus::Result<String>;

    /// SNI `IconThemePath`: extra icon theme search path.
    #[zbus(property)]
    fn icon_theme_path(&self) -> zbus::Result<String>;

    /// SNI `AttentionIconName`.
    #[zbus(property)]
    fn attention_icon_name(&self) -> zbus::Result<String>;

    /// SNI `AttentionIconPixmap`.
    #[zbus(property)]
    fn attention_icon_pixmap(&self) -> zbus::Result<Vec<Icon>>;

    /// SNI `OverlayIconName`.
    #[zbus(property)]
    fn overlay_icon_name(&self) -> zbus::Result<String>;

    /// SNI `OverlayIconPixmap`.
    #[zbus(property)]
    fn overlay_icon_pixmap(&self) -> zbus::Result<Vec<Icon>>;

    /// SNI `ItemId` (stable id) — some clients expose it as `Id` instead.
    #[zbus(property)]
    fn item_id(&self) -> zbus::Result<String>;

    #[zbus(signal)]
    async fn new_icon(&self) -> zbus::Result<()>;

    /// SNI `NewStatus` signal (status changed).
    #[zbus(signal)]
    async fn new_status(&self, status: &str) -> zbus::Result<()>;

    fn activate(&self, x: i32, y: i32) -> zbus::Result<()>;

    /// SNI `ContextMenu` — request the menu be shown at (x, y).
    fn context_menu(&self, x: i32, y: i32) -> zbus::Result<()>;

    /// SNI `SecondaryActivate` — secondary click.
    fn secondary_activate(&self, x: i32, y: i32) -> zbus::Result<()>;

    /// SNI `Scroll` — scroll delta + orientation (`Horizontal`/`Vertical`).
    fn scroll(&self, delta: i32, orientation: &str) -> zbus::Result<()>;
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

    /// `GetGroupProperties` — batch read properties for a set of item ids.
    /// Returns a D-Bus value (array of `{sv}` dicts), deserialized on demand.
    fn get_group_properties(
        &self,
        item_ids: &[i32],
        property_names: &[&str],
    ) -> zbus::Result<OwnedValue>;

    /// `GetProperty` — single property read for one item id.
    fn get_property(&self, item_id: i32, name: &str) -> zbus::Result<OwnedValue>;

    #[zbus(signal)]
    fn layout_updated(&self, revision: u32, parent: i32) -> zbus::Result<()>;

    /// `ItemsRemoved` is a dbusmenu signal (not an SNI signal). It indicates
    /// the menu layout changed (items removed), so we refetch the menu.
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
    use super::{Icon, StatusNotifierWatcher, best_icon_pixmap, split_service_name};

    fn icon(width: i32, height: i32) -> Icon {
        Icon {
            width,
            height,
            bytes: vec![0u8; (width.max(0) as usize) * (height.max(0) as usize) * 4],
        }
    }

    #[test]
    fn protocol_version_is_one() {
        let watcher = StatusNotifierWatcher::default();
        assert_eq!(watcher.protocol_version(), 1);
    }

    #[test]
    fn host_is_reported_registered() {
        let watcher = StatusNotifierWatcher::default();
        assert!(watcher.is_status_notifier_host_registered());
    }

    #[test]
    fn best_icon_pixmap_picks_largest_by_pixel_count() {
        let small = icon(16, 16);
        let large = icon(48, 48);
        let medium = icon(24, 24);

        let icons = vec![small, large.clone(), medium];
        let best = best_icon_pixmap(&icons).expect("expected a best icon");

        assert_eq!(best.width, 48);
        assert_eq!(best.height, 48);
        assert_eq!(best.pixel_count(), 48 * 48);
    }

    #[test]
    fn best_icon_pixmap_ignores_zero_dimension_entries() {
        let zero = icon(0, 32);
        let valid = icon(16, 16);

        let icons = vec![zero, valid];
        let best = best_icon_pixmap(&icons).expect("expected a best icon");

        assert_eq!(best.width, 16);
        assert_eq!(best.height, 16);
        // The zero-dimension entry has pixel count 0 and must not win.
        assert_eq!(best.pixel_count(), 16 * 16);
    }

    #[test]
    fn best_icon_pixmap_returns_none_for_empty_set() {
        assert!(best_icon_pixmap(&[]).is_none());
    }

    #[test]
    fn pixel_count_saturates_on_overflow() {
        let huge = Icon {
            width: i32::MAX,
            height: i32::MAX,
            bytes: Vec::new(),
        };
        assert_eq!(huge.pixel_count(), 0);
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
        // A bare path (leading `/`) splits at index 0: the sender is the
        // empty string and the path is the bare path itself.
        let (sender, path) = split_service_name("/StatusNotifierItem");
        assert_eq!(sender, "");
        assert_eq!(path, "/StatusNotifierItem");
    }
}
