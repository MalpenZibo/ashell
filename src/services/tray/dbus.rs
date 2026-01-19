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
                            if let Some(idx) = interface
                                .items
                                .iter()
                                .position(|(unique_name, _)| unique_name == name)
                            {
                                let emitter =
                                    SignalEmitter::new(&internal_connection, OBJECT_PATH).unwrap();
                                let service = interface.items.remove(idx).1;
                                StatusNotifierWatcher::status_notifier_item_unregistered(
                                    &emitter, &service,
                                )
                                .await
                                .unwrap();
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

            if name_str.contains("StatusNotifierItem") {
                let sender = match dbus_proxy.get_name_owner(BusName::from(name.clone())).await {
                    Ok(owner) => owner,
                    Err(_) => continue,
                };

                let mut watcher = interface.get_mut().await;
                let emitter = SignalEmitter::new(conn, OBJECT_PATH).unwrap();
                watcher
                    .register_status_notifier_item_manual(
                        "/StatusNotifierItem",
                        sender.into_inner(),
                        &emitter,
                    )
                    .await;
            }
        }
        Ok(())
    }
}

impl StatusNotifierWatcher {
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

        if self.items.iter().any(|(_, s)| s == &service) {
            return;
        }

        Self::status_notifier_item_registered(emitter, &service)
            .await
            .unwrap();

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
        let sender = header.sender().unwrap().to_owned();
        self.register_status_notifier_item_manual(service, sender, &emitter)
            .await;
    }

    fn register_status_notifier_host(&mut self, _service: &str) {}

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

#[proxy(interface = "org.kde.StatusNotifierItem")]
pub trait StatusNotifierItem {
    #[zbus(property)]
    fn icon_name(&self) -> zbus::Result<String>;

    #[zbus(property)]
    fn icon_pixmap(&self) -> zbus::Result<Vec<Icon>>;

    #[zbus(property)]
    fn menu(&self) -> zbus::Result<OwnedObjectPath>;
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
