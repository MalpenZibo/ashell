use crate::{
    components::icons::Icons,
    modules::settings::net::NetMessage,
    style::{RED, TEXT, YELLOW},
};
use iced::{
    futures::{
        stream::{self, select_all, SelectAll},
        FutureExt, SinkExt, StreamExt,
    },
    Color, Subscription,
};
use std::collections::HashMap;
use zbus::{
    dbus_proxy,
    zvariant::{OwnedObjectPath, OwnedValue, Value},
    PropertyStream, Result,
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

    #[dbus_proxy(property)]
    fn devices(&self) -> Result<Vec<OwnedObjectPath>>;

    #[dbus_proxy(property)]
    fn wireless_enabled(&self) -> Result<bool>;

    #[dbus_proxy(property)]
    fn set_wireless_enabled(&self, value: bool) -> Result<()>;
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
    fn device_type(&self) -> Result<u32>;

    #[dbus_proxy(property)]
    fn available_connections(&self) -> Result<Vec<OwnedObjectPath>>;

    #[dbus_proxy(property)]
    fn active_connection(&self) -> Result<OwnedObjectPath>;
}

#[dbus_proxy(
    default_service = "org.freedesktop.NetworkManager",
    default_path = "/org/freedesktop/NetworkManager/Device/Wired",
    interface = "org.freedesktop.NetworkManager.Device.Wired"
)]
trait DeviceWired {}

#[dbus_proxy(
    default_service = "org.freedesktop.NetworkManager",
    default_path = "/org/freedesktop/NetworkManager/Device/Wireless",
    interface = "org.freedesktop.NetworkManager.Device.Wireless"
)]
trait DeviceWireless {
    #[dbus_proxy(property)]
    fn active_access_point(&self) -> Result<OwnedObjectPath>;

    #[dbus_proxy(property)]
    fn access_points(&self) -> Result<Vec<OwnedObjectPath>>;

    fn request_scan(&self, options: HashMap<String, OwnedValue>) -> Result<()>;
}

#[dbus_proxy(
    default_service = "org.freedesktop.NetworkManager",
    default_path = "/org/freedesktop/NetworkManager/AccessPoint",
    interface = "org.freedesktop.NetworkManager.AccessPoint"
)]
trait AccessPoint {
    #[dbus_proxy(property)]
    fn ssid(&self) -> Result<Vec<u8>>;

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
    pub ssid: String,
    signal: u8,
}

pub fn get_wifi_icon(signal: u8) -> Icons {
    WIFI_SIGNAL_ICONS[1 + f32::round(signal as f32 / 100. * 3.) as usize]
}

impl ActiveConnection {
    pub fn get_icon(&self) -> Icons {
        match self {
            ActiveConnection::Wifi(wifi) => get_wifi_icon(wifi.signal),
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

#[derive(PartialEq, Eq, Debug, Clone)]
pub enum WifiDeviceState {
    Unavailable,
    Active,
    Inactive,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionType {
    Wired,
    Wifi,
}

#[derive(Debug, Clone)]
pub struct Vpn {
    pub name: String,
    pub is_active: bool,
}

#[derive(Debug, Clone)]
pub struct WifiConnection {
    pub ssid: String,
    pub strength: u8,
}

impl PartialEq for WifiConnection {
    fn eq(&self, other: &Self) -> bool {
        self.ssid == other.ssid
    }
}

async fn get_wifi_devices<'a>(
    devices: Vec<OwnedObjectPath>,
    conn: zbus::Connection,
) -> Vec<DeviceWirelessProxy<'a>> {
    stream::iter(devices.into_iter())
        .filter_map(|d| {
            let conn = conn.clone();
            async move {
                let device = DeviceProxy::builder(&conn)
                    .path(d.to_owned())
                    .unwrap()
                    .build()
                    .await
                    .unwrap();

                if device.device_type().await == Ok(2) {
                    let device = DeviceWirelessProxy::builder(&conn)
                        .path(d)
                        .unwrap()
                        .build()
                        .await
                        .unwrap();

                    Some(device)
                } else {
                    None
                }
            }
        })
        .collect::<Vec<_>>()
        .await
}

async fn get_current_connection<'a>(
    connections: &[OwnedObjectPath],
    conn: zbus::Connection,
    current_access_point_proxy: &mut Option<AccessPointProxy<'a>>,
) -> Option<ActiveConnection> {
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

    let active_connection_data = if let Some((connection_type, connection_proxy)) =
        index.map(|index| connections.swap_remove(index))
    {
        let id = connection_proxy.id().await.unwrap();
        if connection_type == ConnectionType::Wifi {
            let devices = connection_proxy.devices().await.unwrap();

            let access_point = stream::iter(get_wifi_devices(devices, conn.clone()).await.iter())
                .filter_map(|d| {
                    let conn = conn.clone();
                    async move {
                        let access_point = AccessPointProxy::builder(&conn)
                            .path(d.active_access_point().await.unwrap().to_owned())
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
                .await
                .into_iter()
                .next();

            Some((connection_type, id, access_point))
        } else {
            Some((connection_type, id, None))
        }
    } else {
        None
    };

    match active_connection_data {
        Some((ConnectionType::Wifi, id, Some(access_point_proxy))) => {
            let strength = access_point_proxy.strength().await.unwrap_or_default();

            current_access_point_proxy.replace(access_point_proxy);

            Some(ActiveConnection::Wifi(Wifi {
                ssid: id,
                signal: strength,
            }))
        }
        Some((ConnectionType::Wired, _, _)) => {
            *current_access_point_proxy = None;

            Some(ActiveConnection::Wired)
        }
        _ => None,
    }
}

async fn get_vpn_active(connections: &[OwnedObjectPath], conn: zbus::Connection) -> bool {
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
    connections: &[OwnedObjectPath],
    active_connections: &[OwnedObjectPath],
    conn: zbus::Connection,
) -> Vec<Vpn> {
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
                        "vpn" => Some(id),
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
                                let is_active =
                                    active_vpns_name.iter().any(|name| name == id.as_str());
                                Some(Vpn {
                                    name: id,
                                    is_active,
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

async fn find_connection(
    name: &str,
    connections: &[OwnedObjectPath],
    conn: zbus::Connection,
) -> Option<OwnedObjectPath> {
    stream::iter(connections.iter())
        .filter_map(|c| {
            let conn = conn.clone();
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

                if id == name {
                    Some(c.to_owned())
                } else {
                    None
                }
            }
        })
        .collect::<Vec<_>>()
        .await
        .into_iter()
        .next()
}

async fn find_active_connection(
    name: &str,
    active_connections: &[OwnedObjectPath],
    conn: zbus::Connection,
) -> Option<OwnedObjectPath> {
    stream::iter(active_connections.iter())
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

                if id == name {
                    Some(c.to_owned())
                } else {
                    None
                }
            }
        })
        .collect::<Vec<_>>()
        .await
        .into_iter()
        .next()
}

async fn get_kown_wifi_connection<'a>(
    settings: SettingsProxy<'a>,
    conn: &zbus::Connection,
) -> Vec<String> {
    let connections = settings.connections().await.unwrap();

    stream::iter(connections.iter())
        .filter_map(|c| {
            let conn = conn.clone();
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
                            "802-11-wireless" => Some(id),
                            _ => None,
                        },
                        _ => None,
                    })
            }
        })
        .collect::<Vec<_>>()
        .await
}

async fn get_nearby_wifi<'a>(
    wifi_devices: Vec<DeviceWirelessProxy<'a>>,
    conn: zbus::Connection,
    known_connections: &[String],
    active_wifi_connection: Option<&String>,
) -> Vec<WifiConnection> {
    let mut connections: Vec<WifiConnection> = vec![];
    for d in wifi_devices.iter() {
        let cure = stream::iter(d.access_points().await.unwrap())
            .filter_map(|ap| {
                let conn = conn.clone();
                async move {
                    let access_point = AccessPointProxy::builder(&conn)
                        .path(ap.to_owned())
                        .unwrap()
                        .build()
                        .await;

                    if let Ok(access_point) = access_point {
                        let id = String::from_utf8(access_point.ssid().await.unwrap()).unwrap();
                        let strength = access_point.strength().await.unwrap_or_default();

                        Some(WifiConnection { ssid: id, strength })
                    } else {
                        None
                    }
                }
            })
            .collect::<Vec<_>>()
            .await;

        connections.extend(cure);
    }

    connections.dedup();

    let get_sort_value = |e: &WifiConnection| {
        if Some(&e.ssid) == active_wifi_connection {
            return 0;
        }

        if known_connections.iter().any(|c| c == &e.ssid) {
            return 1;
        }

        return 2;
    };

    connections.sort_by(|a, b| get_sort_value(a).cmp(&get_sort_value(b)));

    connections
}

async fn get_nearby_wifi_stream<'a>(
    wifi_device: Vec<OwnedObjectPath>,
    conn: zbus::Connection,
) -> SelectAll<PropertyStream<'a, Vec<OwnedObjectPath>>> {
    let wifi_devices = get_wifi_devices(wifi_device, conn).await;
    let mut nearby_wifi = vec![];
    for d in wifi_devices.iter() {
        nearby_wifi.push(d.receive_access_points_changed().await);
    }

    select_all(nearby_wifi)
}

fn wireless_enabled_to_state(enabled: bool) -> WifiDeviceState {
    if enabled {
        WifiDeviceState::Active
    } else {
        WifiDeviceState::Inactive
    }
}

pub enum NetCommand {
    ScanNearByWifi,
    ToggleWifi,
    GetVpnConnections,
    ActivateVpn(String),
    DeactivateVpn(String),
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

            let mut current_access_point_proxy: Option<AccessPointProxy> = None;
            let mut active_connection = get_current_connection(
                &nm.active_connections().await.unwrap(),
                conn.clone(),
                &mut current_access_point_proxy,
            )
            .await;
            let _ = output
                .send(NetMessage::ActiveConnection(active_connection.clone()))
                .await;

            let mut vpn_active = false;

            let wifi_devices = get_wifi_devices(nm.devices().await.unwrap(), conn.clone()).await;
            let _ = output
                .send(NetMessage::WifiDeviceState(if wifi_devices.len() > 0 {
                    wireless_enabled_to_state(nm.wireless_enabled().await.unwrap())
                } else {
                    WifiDeviceState::Unavailable
                }))
                .await;

            let mut wireless_enabled_changed = nm.receive_wireless_enabled_changed().await;
            let mut devices_changed = nm.receive_devices_changed().await;
            let mut active_connections = nm.receive_active_connections_changed().await;
            let mut nearby_wifi =
                get_nearby_wifi_stream(nm.devices().await.unwrap(), conn.clone()).await;

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
                                NetCommand::ScanNearByWifi => {
                                    let wifi_devices = get_wifi_devices(nm.devices().await.unwrap(), conn.clone()).await;
                                    for d in wifi_devices.iter() {
                                        let _ = d.request_scan(HashMap::new()).await;
                                    }
                                }
                                NetCommand::ToggleWifi => {
                                    let _ = if nm.wireless_enabled().await.unwrap() {
                                        nm.set_wireless_enabled(false).await
                                    } else {
                                        nm.set_wireless_enabled(true).await
                                    };
                                }
                                NetCommand::GetVpnConnections => {
                                    let connections = settings.connections().await.unwrap();
                                    let active_connections = nm.active_connections().await.unwrap();

                                    output.send(NetMessage::VpnConnections(
                                        get_vpn_connections(&connections, &active_connections, conn.clone()).await
                                    )).await.unwrap();
                                }
                                NetCommand::ActivateVpn(name) => {
                                    let object_path = find_connection(
                                        &name,
                                        &settings.connections().await.unwrap(),
                                        conn.clone()
                                    ).await;

                                    if let Some(object_path) = object_path {
                                        let _ = nm.activate_connection(
                                            object_path,
                                            OwnedObjectPath::try_from("/").unwrap(),
                                            OwnedObjectPath::try_from("/").unwrap()
                                        ).await;
                                    }
                                }
                                NetCommand::DeactivateVpn(name) => {
                                    let object_path = find_active_connection(
                                        &name,
                                        &nm.active_connections().await.unwrap(),
                                        conn.clone()
                                    ).await;

                                    if let Some(object_path) = object_path {
                                        let _ = nm.deactivate_connection(
                                            object_path,
                                        ).await;
                                    }
                                }
                            }
                        }
                    },
                    v = wireless_enabled_changed.next().fuse() => {
                        if let Some(state) = v {
                            let _ = output.send(NetMessage::WifiDeviceState(
                                wireless_enabled_to_state(state.get().await.unwrap())
                            )).await;
                        }
                    }
                    v = devices_changed.next().fuse() => {
                        if let Some(_) = v {
                            let wifi_devices = get_wifi_devices(nm.devices().await.unwrap(), conn.clone()).await;
                            if wifi_devices.len() > 0 {
                                nearby_wifi = get_nearby_wifi_stream(
                                    nm.devices().await.unwrap(),
                                    conn.clone()
                                ).await;
                                let _ = output.send(NetMessage::WifiDeviceState(
                                    wireless_enabled_to_state(nm.wireless_enabled().await.unwrap())
                                )).await;
                            } else {
                                let _ = output.send(NetMessage::WifiDeviceState(WifiDeviceState::Unavailable)).await;
                            };
                        }
                    }
                    v = active_connections.next().fuse() => {
                        if let Some(connections) = v {
                            let connections = connections.get().await.unwrap();

                            active_connection =
                               get_current_connection(&connections, conn.clone(), &mut current_access_point_proxy).await;

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
                    v = nearby_wifi.next().fuse() => {
                        if let Some(_) = v {
                            let wifi_connections = get_nearby_wifi(
                                get_wifi_devices(
                                    nm.devices().await.unwrap(),
                                    conn.clone()
                                ).await,
                                conn.clone(),
                                &get_kown_wifi_connection(settings.clone(), &conn).await,
                                if let Some(ActiveConnection::Wifi(wifi)) = active_connection.as_ref() {
                                    Some(&wifi.ssid)
                                } else {
                                    None
                                }
                            ).await;

                            let _ = output.send(NetMessage::NearByWifi(wifi_connections)).await;
                        }
                    }
                }
            }
        },
    )])
}
