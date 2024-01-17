use crate::{
    components::icons::Icons,
    modules::settings::net::NetMessage,
    style::{RED, TEXT, YELLOW},
};
use iced::{
    futures::{
        stream::{self},
        FutureExt, SinkExt, StreamExt,
    },
    Color, Subscription,
};
use std::collections::HashMap;
use zbus::{
    dbus_proxy,
    zvariant::{OwnedObjectPath, OwnedValue, Value},
    Result,
};

static WIFI_SIGNAL_ICONS: [Icons; 5] = [
    Icons::Wifi0,
    Icons::Wifi1,
    Icons::Wifi2,
    Icons::Wifi3,
    Icons::Wifi4,
];

#[dbus_proxy(
    interface = "org.freedesktop.NetworkManager",
    default_service = "org.freedesktop.NetworkManager",
    default_path = "/org/freedesktop/NetworkManager"
)]
trait NetworkManager {
    fn activate_connection(
        &self,
        connection: OwnedObjectPath,
        device: OwnedObjectPath,
        specific_object: OwnedObjectPath,
    ) -> Result<OwnedObjectPath>;

    fn deactivate_connection(&self, connection: OwnedObjectPath) -> Result<()>;

    #[dbus_proxy(property)]
    fn active_connections(&self) -> Result<Vec<OwnedObjectPath>>;
}

#[dbus_proxy(
    default_service = "org.freedesktop.NetworkManager",
    default_path = "/org/freedesktop/NetworkManager/Connection/Active",
    interface = "org.freedesktop.NetworkManager.Connection.Active"
)]
trait ActiveConnection {
    #[dbus_proxy(property)]
    fn id(&self) -> Result<String>;

    #[dbus_proxy(property)]
    fn uuid(&self) -> Result<String>;

    #[dbus_proxy(property, name = "Type")]
    fn connection_type(&self) -> Result<String>;

    #[dbus_proxy(property)]
    fn state(&self) -> Result<u32>;

    #[dbus_proxy(property)]
    fn vpn(&self) -> Result<bool>;

    #[dbus_proxy(property)]
    fn devices(&self) -> Result<Vec<OwnedObjectPath>>;
}

#[dbus_proxy(
    default_service = "org.freedesktop.NetworkManager",
    default_path = "/org/freedesktop/NetworkManager/Device",
    interface = "org.freedesktop.NetworkManager.Device"
)]
trait Device {
    #[dbus_proxy(property)]
    fn device_type(&self) -> Result<u8>;
}

#[dbus_proxy(
    default_service = "org.freedesktop.NetworkManager",
    default_path = "/org/freedesktop/NetworkManager/Device/Wireless",
    interface = "org.freedesktop.NetworkManager.Device.Wireless"
)]
trait DeviceWireless {
    #[dbus_proxy(property)]
    fn active_access_point(&self) -> Result<OwnedObjectPath>;
}

#[dbus_proxy(
    default_service = "org.freedesktop.NetworkManager",
    default_path = "/org/freedesktop/NetworkManager/AccessPoint",
    interface = "org.freedesktop.NetworkManager.AccessPoint"
)]
trait AccessPoint {
    #[dbus_proxy(property)]
    fn strength(&self) -> Result<u8>;
}

#[dbus_proxy(
    default_service = "org.freedesktop.NetworkManager",
    default_path = "/org/freedesktop/NetworkManager/Settings",
    interface = "org.freedesktop.NetworkManager.Settings"
)]
trait Settings {
    #[dbus_proxy(property)]
    fn connections(&self) -> Result<Vec<OwnedObjectPath>>;
}

#[dbus_proxy(
    default_service = "org.freedesktop.NetworkManager",
    default_path = "/org/freedesktop/NetworkManager/Settings/Connection",
    interface = "org.freedesktop.NetworkManager.Settings.Connection"
)]
trait SettingsConnection {
    fn get_settings(&self) -> Result<HashMap<String, HashMap<String, OwnedValue>>>;
}

#[derive(Debug, Clone)]
pub enum ActiveConnection {
    Wifi(Wifi),
    Wired,
}

#[derive(Debug, Clone)]
pub struct Wifi {
    pub connection_ssid: String,
    signal: u8,
}

impl ActiveConnection {
    pub fn get_icon(&self) -> Icons {
        match self {
            ActiveConnection::Wifi(wifi) => {
                WIFI_SIGNAL_ICONS[f32::floor(wifi.signal as f32 / 100.) as usize % (4 - 1 + 1) + 1]
            }
            ActiveConnection::Wired => Icons::Ethernet,
        }
    }

    pub fn get_color(&self) -> Color {
        match self {
            ActiveConnection::Wifi(wifi) => match wifi.signal {
                0 => RED,
                1 => YELLOW,
                _ => TEXT,
            },
            _ => TEXT,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionType {
    Wired,
    Wifi,
    Vpn,
}

#[derive(Debug, Clone)]
pub struct Connection {
    id: String,
    r#type: ConnectionType,
    is_active: bool,
}

#[derive(Debug, Clone)]
pub struct Vpn {
    pub object_path: OwnedObjectPath,
    pub name: String,
    pub active_object_path: Option<OwnedObjectPath>,
}

async fn get_current_connection<'a>(
    connections: &Vec<OwnedObjectPath>,
    conn: zbus::Connection,
) -> Option<(ConnectionType, String, Option<AccessPointProxy<'a>>)> {
    let mut connections = stream::iter(connections.iter())
        .filter_map(|c| {
            let conn = conn.clone();
            async move {
                let builder = ActiveConnectionProxy::builder(&conn).path(c.to_owned());
                if let Ok(builder) = builder {
                    let connection = builder.build().await;
                    if let Ok(connection) = connection {
                        let connection_type = connection.connection_type().await;
                        if let Ok(connection_type) = connection_type {
                            return match connection_type.as_str() {
                                "802-11-wireless" => Some((ConnectionType::Wifi, connection)),
                                "802-3-ethernet" => Some((ConnectionType::Wired, connection)),
                                _ => None,
                            };
                        }
                    }
                }

                None
            }
        })
        .collect::<Vec<_>>()
        .await;

    let index = connections
        .iter()
        .position(|(t, _)| t == &ConnectionType::Wired)
        .or_else(|| {
            connections
                .iter()
                .position(|(t, _)| t == &ConnectionType::Wifi)
        });

    if let Some((connection_type, connection_proxy)) =
        index.map(|index| connections.swap_remove(index))
    {
        let id = connection_proxy.id().await.unwrap();
        if connection_type == ConnectionType::Wifi {
            let devices = connection_proxy.devices().await.unwrap();

            let wifi_devices = stream::iter(devices.iter())
                .filter_map(|d| {
                    let conn = conn.clone();
                    async move {
                        let device = DeviceWirelessProxy::builder(&conn)
                            .path(d)
                            .unwrap()
                            .build()
                            .await
                            .unwrap();
                        let access_point = AccessPointProxy::builder(&conn)
                            .path(device.active_access_point().await.unwrap().to_owned())
                            .unwrap()
                            .build()
                            .await;

                        if let Ok(access_point) = access_point {
                            Some(access_point)
                        } else {
                            None
                        }
                    }
                })
                .collect::<Vec<_>>()
                .await;

            Some((connection_type, id, wifi_devices.into_iter().next()))
        } else {
            Some((connection_type, id, None))
        }
    } else {
        None
    }
}

async fn get_vpn_active(connections: &Vec<OwnedObjectPath>, conn: zbus::Connection) -> bool {
    stream::iter(connections.iter())
        .any(|c| {
            let conn = conn.clone();
            async move {
                ActiveConnectionProxy::builder(&conn)
                    .path(c.to_owned())
                    .unwrap()
                    .build()
                    .await
                    .unwrap()
                    .vpn()
                    .await
                    .unwrap_or_default()
            }
        })
        .await
}

async fn get_vpn_connections(
    connections: &Vec<OwnedObjectPath>,
    active_connections: &Vec<OwnedObjectPath>,
    conn: zbus::Connection,
) -> Vec<Vpn> {
    println!("VPN Connections: {:?}", active_connections);

    let active_vpns_name = stream::iter(active_connections.iter())
        .filter_map(|c| {
            let conn = conn.clone();
            async move {
                let connection = ActiveConnectionProxy::builder(&conn)
                    .path(c.to_owned())
                    .unwrap()
                    .build()
                    .await
                    .unwrap();

                let id = connection.id().await.unwrap();

                connection
                    .connection_type()
                    .await
                    .map(|v| match v.as_str() {
                        "vpn" => Some((c, id)),
                        _ => None,
                    })
                    .unwrap_or_default()
            }
        })
        .collect::<Vec<_>>()
        .await;

    stream::iter(connections.iter())
        .filter_map(|c| {
            let conn = conn.clone();
            let active_vpns_name = &active_vpns_name;
            async move {
                let connection = SettingsConnectionProxy::builder(&conn)
                    .path(c.to_owned())
                    .unwrap()
                    .build()
                    .await
                    .unwrap();

                let settings = connection.get_settings().await.unwrap();

                let id = settings
                    .get("connection")
                    .unwrap()
                    .get("id")
                    .map(|v| match v.into() {
                        Value::Str(v) => v.to_string(),
                        _ => "".to_string(),
                    })
                    .unwrap_or_default();

                settings
                    .get("connection")
                    .unwrap()
                    .get("type")
                    .and_then(|v| match v.into() {
                        Value::Str(v) => match v.as_str() {
                            "vpn" => {
                                let path = active_vpns_name.iter().find_map(|(path, name)| {
                                    if name == &id {
                                        Some((*path).to_owned())
                                    } else {
                                        None
                                    }
                                });
                                Some(Vpn {
                                    object_path: c.clone(),
                                    name: id,
                                    active_object_path: path,
                                })
                            }
                            _ => None,
                        },
                        _ => None,
                    })
            }
        })
        .collect::<Vec<_>>()
        .await
}

pub enum NetCommand {
    GetVpnConnections,
    ActivateVpn(OwnedObjectPath),
    DeactivateVpn(OwnedObjectPath),
}

pub fn subscription(
    rx: Option<tokio::sync::mpsc::UnboundedReceiver<NetCommand>>,
) -> Subscription<NetMessage> {
    iced::Subscription::batch(vec![iced::subscription::channel(
        "nm-dbus-connection-listener",
        100,
        |mut output| async move {
            let mut rx = rx.unwrap();

            let conn = zbus::Connection::system().await.unwrap();
            let nm = NetworkManagerProxy::new(&conn).await.unwrap();
            let settings = SettingsProxy::new(&conn).await.unwrap();

            let mut active_connections = nm.receive_active_connections_changed().await;

            let mut current_access_point_proxy: Option<AccessPointProxy> = None;
            let mut active_connection: Option<ActiveConnection> = None;

            let mut vpn_active = false;

            loop {
                let mut strength_changed =
                    if let Some(access_point_proxy) = current_access_point_proxy.as_ref() {
                        access_point_proxy.receive_strength_changed().await.boxed()
                    } else {
                        stream::pending().boxed()
                    };
                iced::futures::select_biased! {
                    v = rx.recv().fuse() => {
                        if let Some(v) = v {
                            match v {
                                NetCommand::GetVpnConnections => {
                                    let connections = settings.connections().await.unwrap();
                                    let active_connections = nm.active_connections().await.unwrap();

                                    output.send(NetMessage::VpnConnections(
                                        get_vpn_connections(&connections, &active_connections, conn.clone()).await
                                    )).await.unwrap();
                                }
                                NetCommand::ActivateVpn(object_path) => {
                                    let _ = nm.activate_connection(
                                        object_path,
                                        OwnedObjectPath::try_from("/").unwrap(),
                                        OwnedObjectPath::try_from("/").unwrap()
                                    ).await;
                                }
                                NetCommand::DeactivateVpn(object_path) => {
                                    let _ = nm.deactivate_connection(object_path).await;
                                }
                            }
                        }
                    },
                    v = active_connections.next().fuse() => {
                        if let Some(connections) = v {
                            let connections = connections.get().await.unwrap();

                            active_connection =
                                match get_current_connection(&connections, conn.clone()).await {
                                    Some((ConnectionType::Wifi, id, Some(access_point_proxy))) => {
                                        let strength = access_point_proxy
                                            .strength()
                                            .await
                                            .unwrap_or_default();

                                        current_access_point_proxy.replace(access_point_proxy);

                                        Some(ActiveConnection::Wifi(Wifi {
                                            connection_ssid: id,
                                            signal: strength,
                                        }))
                                    }
                                    Some((ConnectionType::Wired, _, _)) => {
                                        current_access_point_proxy = None;

                                        Some(ActiveConnection::Wired)
                                    }
                                    _ => None,
                                };

                            let current_vpn_active = get_vpn_active(&connections, conn.clone()).await;
                            if current_vpn_active != vpn_active {
                                vpn_active = current_vpn_active;
                                let _ = output
                                    .send(NetMessage::VpnActive(vpn_active))
                                    .await;
                            }

                            let _ = output
                                .send(NetMessage::ActiveConnection(active_connection.clone()))
                                .await;
                        }
                    },
                    v = strength_changed.next().fuse() => {
                        if let Some(strength) = v {
                            if let Some(active_connection) = active_connection.as_mut() {
                                if let ActiveConnection::Wifi(wifi) = active_connection {
                                    let value = strength.get().await.unwrap();

                                    if value.abs_diff(wifi.signal) > 10 {
                                        wifi.signal = value;
                                        let _ = output
                                            .send(NetMessage::ActiveConnection(Some(active_connection.clone())))
                                            .await;
                                    }
                                }
                            }
                        }
                    },
                }
            }
        },
    )])
}
