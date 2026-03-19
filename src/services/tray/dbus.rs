use iced::futures::StreamExt;
use log::{debug, info, warn};
use std::time::Duration;
use zbus::{
    Connection, Result,
    fdo::{DBusProxy, IntrospectableProxy, RequestNameFlags, RequestNameReply},
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
    items: std::collections::HashMap<String, String>,
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

        let flags = RequestNameFlags::AllowReplacement.into();
        if dbus_proxy.request_name(NAME, flags).await? == RequestNameReply::InQueue {
            warn!("Bus name '{NAME}' already owned");
        }

        let internal_connection = connection.clone();
        let internal_interface = interface.clone();
        tokio::spawn(async move {
            let mut have_bus_name = false;
            let unique_name = internal_connection.unique_name().map(|x| x.as_ref());

            let mut name_owner_changed_stream = name_owner_changed_stream.fuse();
            let mut interval = tokio::time::interval(Duration::from_secs(30));

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
                            if let Some(service) = interface.items.remove(&name.to_string()) {
                                let emitter = match SignalEmitter::new(&internal_connection, OBJECT_PATH) {
                                    Ok(emitter) => emitter,
                                    Err(err) => {
                                        warn!("Failed to create signal emitter for unregistration: {err}");
                                        continue;
                                    }
                                };
                                if let Err(err) = StatusNotifierWatcher::status_notifier_item_unregistered(
                                    &emitter, &service,
                                )
                                .await {
                                    warn!("Failed to emit status notifier item unregistered signal for service '{service}': {err}");
                                }
                            }
                        }
                    }
                    _ = interval.tick() => {
                        if let Err(e) = Self::discover_items(&internal_connection, &internal_interface).await {
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

    async fn discover_items(
        conn: &Connection,
        interface: &zbus::object_server::InterfaceRef<StatusNotifierWatcher>,
    ) -> anyhow::Result<()> {
        let dbus_proxy = DBusProxy::new(conn).await?;
        let names = dbus_proxy.list_names().await?;

        for name in names {
            let name_str = name.as_str();
            if name_str.starts_with(':')
                || name_str == NAME.as_str()
                || name_str == "org.freedesktop.DBus"
            {
                continue;
            }

            // Only try to register services that explicitly mention StatusNotifierItem
            if name_str.contains("StatusNotifierItem") {
                Self::try_register_item(
                    conn,
                    interface,
                    &dbus_proxy,
                    &BusName::from(name.clone()),
                    "/StatusNotifierItem",
                )
                .await;
            } else {
                // Try introspection for other services
                if let Some(path_hint) = Self::find_status_notifier_path(conn, name_str).await {
                    Self::try_register_item(
                        conn,
                        interface,
                        &dbus_proxy,
                        &BusName::from(name.clone()),
                        &path_hint,
                    )
                    .await;
                }
            }
        }
        Ok(())
    }

    async fn try_register_item(
        conn: &Connection,
        interface: &zbus::object_server::InterfaceRef<StatusNotifierWatcher>,
        dbus_proxy: &DBusProxy<'_>,
        name: &BusName<'_>,
        service_path: &str,
    ) {
        let sender = match dbus_proxy.get_name_owner(name.clone()).await {
            Ok(owner) => owner,
            Err(_) => return,
        };

        let mut watcher = interface.get_mut().await;
        let emitter = match SignalEmitter::new(conn, OBJECT_PATH) {
            Ok(emitter) => emitter,
            Err(err) => {
                warn!("Failed to create signal emitter for registration: {err}");
                return;
            }
        };
        watcher
            .register_status_notifier_item_manual(service_path, sender.into_inner(), &emitter)
            .await;
    }
}

impl StatusNotifierWatcher {
    async fn find_status_notifier_path(conn: &Connection, name: &str) -> Option<String> {
        debug!("Attempting to find StatusNotifier path for service: {name}");
        let candidates = [
            "/StatusNotifierItem",
            "/org/ayatana/NotificationItem",
            "/MenuBar",
        ];

        for path in candidates {
            debug!("Trying path: {path} for service: {name}");
            let builder = match IntrospectableProxy::builder(conn).destination(name.to_string()) {
                Ok(builder) => builder,
                Err(err) => {
                    debug!("Failed to create proxy builder for {name} at {path}: {err}");
                    continue;
                }
            };

            let builder = match builder.path(path) {
                Ok(builder) => builder,
                Err(err) => {
                    debug!("Failed to set path {path} for {name}: {err}");
                    continue;
                }
            };

            let proxy = match builder.build().await {
                Ok(proxy) => proxy,
                Err(err) => {
                    debug!("Failed to build proxy for {name} at {path}: {err}");
                    continue;
                }
            };

            if let Ok(xml_result) =
                tokio::time::timeout(tokio::time::Duration::from_secs(5), proxy.introspect()).await
            {
                if let Ok(xml) = xml_result
                    && xml.contains("org.kde.StatusNotifierItem")
                {
                    info!("Found StatusNotifierItem at {path} for service: {name}");
                    return Some(path.to_string());
                } else {
                    debug!(
                        "Introspection successful for {name} at {path}, but no StatusNotifierItem interface found"
                    );
                }
            } else {
                warn!("Introspection timeout for {name} at {path} after 5 seconds");
            }
        }

        debug!("No StatusNotifierItem path found for service: {name}");
        None
    }

    async fn register_status_notifier_item_manual(
        &mut self,
        service: &str,
        sender: UniqueName<'static>,
        emitter: &SignalEmitter<'_>,
    ) {
        let service = if service.starts_with('/') {
            format!("{sender}{service}")
        } else {
            service.to_string()
        };

        let sender_key = sender.to_string();

        if let Some(existing_service) = self.items.get(&sender_key)
            && existing_service == &service
        {
            return;
        }

        // Check if this service is already registered by a different sender
        if let Some((old_sender, old_service)) = self
            .items
            .iter()
            .find(|(_, s)| *s == &service)
            .map(|(k, v)| (k.clone(), v.clone()))
        {
            // Emit unregistered signal for the old entry before removing it
            if let Err(err) = Self::status_notifier_item_unregistered(emitter, &old_service).await {
                warn!(
                    "Failed to emit status_notifier_item_unregistered for duplicate service '{old_service}': {err}"
                );
            }
            self.items.remove(&old_sender);
        }

        if let Err(err) = Self::status_notifier_item_registered(emitter, &service).await {
            warn!("Failed to emit status_notifier_item_registered for '{service}': {err:?}");
        }

        self.items.insert(sender_key, service);
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
            Some(sender) => sender.to_owned(),
            None => {
                warn!("Received status notifier item registration signal without sender header");
                return;
            }
        };
        self.register_status_notifier_item_manual(service, sender, &emitter)
            .await;
    }

    fn register_status_notifier_host(&mut self, _service: &str) {}

    #[zbus(property)]
    fn registered_status_notifier_items(&self) -> Vec<String> {
        self.items.values().cloned().collect()
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

/// Convert ARGB pixel format to RGBA
/// ARGB format is [A, R, G, B], RGBA format is [R, G, B, A]
/// rotate_left(1) moves the alpha byte from position 0 to position 3
pub fn convert_argb_to_rgba(mut icon: Icon) -> Icon {
    for pixel in icon.bytes.chunks_exact_mut(4) {
        pixel.rotate_left(1);
    }
    icon
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
    fn id(&self) -> zbus::Result<String>;

    #[zbus(property)]
    fn title(&self) -> zbus::Result<String>;
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
}
