use iced::futures::StreamExt;
use log::{info, warn};
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
        connection
            .object_server()
            .at(OBJECT_PATH, StatusNotifierWatcher::default())
            .await?;
        let interface = connection
            .object_server()
            .interface::<_, StatusNotifierWatcher>(OBJECT_PATH)
            .await?;

        let dbus_proxy = DBusProxy::new(&connection).await?;
        let mut name_owner_changed_stream = dbus_proxy.receive_name_owner_changed().await?;

        let flags = RequestNameFlags::AllowReplacement.into();
        if dbus_proxy.request_name(NAME, flags).await? == RequestNameReply::InQueue {
            warn!("Bus name '{NAME}' already owned");
        }

        let internal_connection = connection.clone();
        tokio::spawn(async move {
            let mut have_bus_name = false;
            let unique_name = internal_connection.unique_name().map(|x| x.as_ref());
            while let Some(evt) = name_owner_changed_stream.next().await {
                let args = match evt.args() {
                    Ok(args) => args,
                    Err(_) => {
                        continue;
                    }
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
                    let mut interface = interface.get_mut().await;
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
        });

        Ok(connection)
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
        let sender = header.sender().unwrap();
        let service = if service.starts_with('/') {
            format!("{sender}{service}")
        } else {
            service.to_string()
        };
        Self::status_notifier_item_registered(&emitter, &service)
            .await
            .unwrap();

        self.items.push((sender.to_owned(), service));
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
