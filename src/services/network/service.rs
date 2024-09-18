use super::dbus::{ActiveConnectionState, ConnectivityState, DeviceState, NetworkDbus};
use crate::{
    components::icons::{icon, Icons},
    services::{ReadOnlyService, Service},
    utils::IndicatorState,
};
use iced::{
    futures::{
        channel::mpsc::{unbounded, Sender, UnboundedReceiver, UnboundedSender}, stream::pending, SinkExt, StreamExt
    },
    subscription::channel,
    widget::container,
    Command, Element, Subscription, Theme,
};
use log::{debug, error, info, warn};
use std::{any::TypeId, ops::Deref};
use zbus::zvariant::ObjectPath;

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
    pub public: bool,
    pub known: bool,
}

impl PartialEq for WifiConnection {
    fn eq(&self, other: &Self) -> bool {
        self.ssid == other.ssid
    }
}

#[derive(Debug, Clone)]
pub enum NetworkEvent {
    Init(NetworkService),
}

enum NetworkCommand {
    ScanNearbyWifi,
}
#[derive(Debug, Clone)]
pub struct AccessPoint {
    pub ssid: String,
    pub strength: u8,
    pub state: DeviceState,
    pub public: bool,
    pub working: bool,
    pub path: ObjectPath<'static>,
}

#[derive(Debug, Clone)]
pub enum KnownConnection {
    AccessPoint(AccessPoint),
    Vpn(String),
}

#[derive(Debug, Clone)]
pub enum ActiveConnectionInfo {
    Wired {
        name: String,
        speed: u32,
    },
    WiFi {
        name: String,
        state: ActiveConnectionState,
        strength: u8,
    },
    Vpn {
        name: String,
    },
}

impl ActiveConnectionInfo {
    pub fn name(&self) -> String {
        match &self {
            Self::Wired { name, .. } => name.clone(),
            Self::WiFi { name, .. } => name.clone(),
            Self::Vpn { name, .. } => name.clone(),
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct NetworkData {
    pub wireless_access_points: Vec<AccessPoint>,
    pub active_connections: Vec<ActiveConnectionInfo>,
    pub known_connections: Vec<KnownConnection>,
    pub wifi_enabled: bool,
    pub airplane_mode: bool,
    pub connectivity: ConnectivityState,
    pub scanning_nearby_wifi: bool,
}

#[derive(Debug, Clone)]
pub struct NetworkService {
    data: NetworkData,
    conn: zbus::Connection,
    commander: UnboundedSender<NetworkCommand>,
}

impl Deref for NetworkService {
    type Target = NetworkData;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

enum State {
    Init,
    Active(zbus::Connection, UnboundedReceiver<NetworkCommand>),
    Error,
}

impl ReadOnlyService for NetworkService {
    type Data = NetworkData;
    type Event = NetworkEvent;

    fn subscribe() -> Subscription<Self::Event> {
        let id = TypeId::of::<Self>();

        channel(id, 50, |mut output| async move {
            let mut state = State::Init;

            loop {
                state = NetworkService::start_listening(state, &mut output).await;
            }
        })
    }
}

impl NetworkService {
    async fn initialize_data(conn: &zbus::Connection) -> anyhow::Result<NetworkData> {
        let nm = NetworkDbus::new(conn).await?;

        // airplane mode
        let airplaine_mode = tokio::process::Command::new("rfkill")
            .arg("list")
            .arg("bluetooth")
            .output()
            .await?;
        let airplane_mode = std::str::from_utf8(&airplaine_mode.stdout).unwrap_or_default();

        let wifi_enabled = nm.wireless_enabled().await.unwrap_or_default();
        debug!("Wifi enabled: {}", wifi_enabled);

        let airplane_mode = airplane_mode.contains("Soft blocked: yes") && !wifi_enabled;
        debug!("Airplane mode: {}", airplane_mode);

        let active_connections = nm.active_connections().await?;
        debug!("Active connections: {:?}", active_connections);

        let wireless_access_points = nm.wireless_access_points().await?;
        debug!("Wireless access points: {:?}", wireless_access_points);

        let known_connections = nm
            .known_connections(&wireless_access_points, &active_connections)
            .await?;
        debug!("Known connections: {:?}", known_connections);

        Ok(NetworkData {
            active_connections,
            wifi_enabled,
            airplane_mode,
            connectivity: nm.connectivity().await?,
            wireless_access_points,
            known_connections,
            scanning_nearby_wifi: false,
        })
    }

    async fn start_listening(
        state: State,
        output: &mut Sender<<Self as ReadOnlyService>::Event>,
    ) -> State {
        match state {
            State::Init => match zbus::Connection::system().await {
                Ok(conn) => {
                    let (tx, rx) = unbounded();

                    let data = NetworkService::initialize_data(&conn).await;

                    match data {
                        Ok(data) => {
                            info!("Network service initialized");

                            let _ = output
                                .send(NetworkEvent::Init(NetworkService {
                                    data,
                                    conn: conn.clone(),
                                    commander: tx,
                                }))
                                .await;

                            State::Active(conn, rx)
                        }
                        Err(err) => {
                            error!("Failed to initialize network service: {}", err);

                            State::Error
                        }
                    }
                }
                Err(err) => {
                    error!("Failed to connect to system bus: {}", err);

                    State::Error
                }
            },
            State::Active(conn, mut rx) => {
                info!("Listening for network events");
                let _ = rx.next().await;

                State::Active(conn, rx)
            }
            State::Error => {
                error!("Network service error");

                let _ = pending::<u8>().next().await;

                State::Error
            }
        }
    }

    pub fn command(command: NetworkCommand) -> Command<NetworkEvent> {
        Command::none()
        // match command {
        //     NetworkCommand::ScanNearbyWifi => {
        //         Command::perform(async { NetworkService::scan_nearby_wifi().await }, |_| {
        //             NetMessage::ScanNearByWifi
        //         })
        //     }
        // }
    }
}

// pub fn subscription(// rx: Option<tokio::sync::mpsc::UnboundedReceiver<NetCommand>>,
// ) -> Subscription<NetMessage> {
//     subscription::channel(
//         "nm-dbus-connection-listener",
//         100,
//         |mut output| async move {
//             // let mut rx = rx.unwrap();
//
//             let conn = zbus::Connection::system().await.unwrap();
//             let nm = NetworkManagerProxy::new(&conn).await.unwrap();
//             let settings = SettingsProxy::new(&conn).await.unwrap();
//
//             let mut current_access_point_proxy: Option<AccessPointProxy> = None;
//             let mut active_connection = get_current_connection(
//                 &nm.active_connections().await.unwrap(),
//                 conn.clone(),
//                 &mut current_access_point_proxy,
//             )
//             .await;
//
//             let mut vpn_active = false;
//
//             loop {
//                 let _ = output
//                     .feed(NetMessage::ActiveConnection(active_connection.clone()))
//                     .await;
//
//                 let wifi_devices =
//                     get_wifi_devices(nm.devices().await.unwrap(), conn.clone()).await;
//                 let _ = output
//                     .feed(NetMessage::WifiDeviceState(if !wifi_devices.is_empty() {
//                         wireless_enabled_to_state(nm.wireless_enabled().await.unwrap())
//                     } else {
//                         WifiDeviceState::Unavailable
//                     }))
//                     .await;
//
//                 output.flush().await;
//
//                 let wireless_enabled_changed = nm
//                     .receive_wireless_enabled_changed()
//                     .await
//                     .then(|v| async move {
//                         if let Ok(state) = v.get().await {
//                             Some(NetworkEvent::WirelessEnabledChanged(state))
//                         } else {
//                             None
//                         }
//                     })
//                     .boxed();
//                 let devices_changed = nm
//                     .receive_devices_changed()
//                     .await
//                     .then(|v| async move {
//                         if v.get().await.is_ok() {
//                             Some(NetworkEvent::DevicesChanged)
//                         } else {
//                             None
//                         }
//                     })
//                     .boxed();
//                 let active_connections = nm
//                     .receive_active_connections_changed()
//                     .await
//                     .then(|v| async move {
//                         if let Ok(connections) = v.get().await {
//                             Some(NetworkEvent::ActiveConnectionsChanged(connections))
//                         } else {
//                             None
//                         }
//                     })
//                     .boxed();
//                 let nearby_wifi = get_nearby_wifi_stream(nm.devices().await.unwrap(), conn.clone())
//                     .await
//                     .then(|v| async move {
//                         if let Ok(data) = v.get().await {
//                             Some(NetworkEvent::NearbyWifiChanged(data))
//                         } else {
//                             None
//                         }
//                     })
//                     .boxed();
//
//                 let strength_changed =
//                     if let Some(access_point_proxy) = current_access_point_proxy.as_ref() {
//                         access_point_proxy
//                             .receive_strength_changed()
//                             .await
//                             .then(|v| async move {
//                                 if let Ok(strength) = v.get().await {
//                                     Some(NetworkEvent::StrengthChanged(strength))
//                                 } else {
//                                     None
//                                 }
//                             })
//                             .boxed()
//                     } else {
//                         stream::pending().boxed()
//                     };
//
//                 let mut combined = select_all(vec![
//                     wireless_enabled_changed,
//                     devices_changed,
//                     active_connections,
//                     strength_changed,
//                     nearby_wifi,
//                 ]);
//
//                 while let Some(event) = combined.next().await {
//                     if let Some(event) = event {
//                         let msg = match event {
//                             NetworkEvent::WirelessEnabledChanged(state) => Some(
//                                 NetMessage::WifiDeviceState(wireless_enabled_to_state(state)),
//                             ),
//                             NetworkEvent::DevicesChanged => {
//                                 warn!("Devices changed! re-initialize network loop");
//                                 break;
//                             }
//                             NetworkEvent::ActiveConnectionsChanged(connections) => {
//                                 active_connection = get_current_connection(
//                                     &connections,
//                                     conn.clone(),
//                                     &mut current_access_point_proxy,
//                                 )
//                                 .await;
//
//                                 let current_vpn_active =
//                                     get_vpn_active(&connections, conn.clone()).await;
//
//                                 Some(if current_vpn_active != vpn_active {
//                                     vpn_active = current_vpn_active;
//                                     NetMessage::VpnActive(vpn_active)
//                                 } else {
//                                     // TODO: check if we should send multiple messages
//                                     NetMessage::ActiveConnection(active_connection.clone())
//                                 })
//                             }
//                             NetworkEvent::StrengthChanged(strength) => {
//                                 if let Some(active_connection) = active_connection.as_mut() {
//                                     if let ActiveConnection::Wifi(wifi) = active_connection {
//                                         if strength.abs_diff(wifi.signal) > 10 {
//                                             wifi.signal = strength;
//                                             Some(NetMessage::ActiveConnection(Some(
//                                                 active_connection.clone(),
//                                             )))
//                                         } else {
//                                             None
//                                         }
//                                     } else {
//                                         None
//                                     }
//                                 } else {
//                                     None
//                                 }
//                             }
//                             NetworkEvent::NearbyWifiChanged(_data) => {
//                                 let wifi_connections = get_nearby_wifi(
//                                     get_wifi_devices(nm.devices().await.unwrap(), conn.clone())
//                                         .await,
//                                     conn.clone(),
//                                     &get_kown_wifi_connection(settings.clone(), &conn).await,
//                                     if let Some(ActiveConnection::Wifi(wifi)) =
//                                         active_connection.as_ref()
//                                     {
//                                         Some(&wifi.ssid)
//                                     } else {
//                                         None
//                                     },
//                                 )
//                                 .await;
//
//                                 Some(NetMessage::NearByWifi(wifi_connections))
//                             }
//                         };
//
//                         if let Some(msg) = msg {
//                             let _ = output.send(msg).await;
//                         }
//                     }
//                 }
//             }
// futures::select_biased! {
//     v = rx.recv().fuse() => {
//         if let Some(v) = v {
//             match v {
//                 NetCommand::ScanNearByWifi => {
//                     let wifi_devices = get_wifi_devices(nm.devices().await.unwrap(), conn.clone()).await;
//                     for d in wifi_devices.iter() {
//                         let _ = d.request_scan(HashMap::new()).await;
//                     }
//                 }
//                 NetCommand::ToggleWifi => {
//                     let _ = if nm.wireless_enabled().await.unwrap() {
//                         nm.set_wireless_enabled(false).await
//                     } else {
//                         nm.set_wireless_enabled(true).await
//                     };
//                 }
//                 NetCommand::ActivateWifiConnection(name, password) => {
//                     let res = activate_wifi_connection(
//                         &name,
//                         password,
//                         &nm,
//                         &settings,
//                         &conn
//                     ).await;
//
//                     if res.is_err() {
//                         let _ = output.send(NetMessage::RequestWifiPassword(name)).await;
//                     } else {
//                         let device = get_current_device(&name, &nm, &conn).await;
//                         if let Some(device) = device {
//                             let mut state_change = device.receive_state_changed().await;
//                             loop {
//                                 if let Some(state) = state_change.next().await {
//                                     let state = state.get().await.unwrap();
//                                     match state {
//                                         100 => {
//                                             info!("Wifi connection activated");
//                                             break
//                                         },
//                                         120 => {
//                                             info!("Wifi connection failed");
//                                             let _ = output.send(NetMessage::RequestWifiPassword(name)).await;
//                                             break
//                                         },
//                                         state => {
//                                             debug!("state {}, waiting...", state);
//                                         }
//                                     }
//                                 }
//                             }
//                         }
//                     }
//                 }
//                 NetCommand::GetVpnConnections => {
//                     let connections = settings.connections().await.unwrap();
//                     let active_connections = nm.active_connections().await.unwrap();
//
//                     output.send(NetMessage::VpnConnections(
//                         get_vpn_connections(&connections, &active_connections, conn.clone()).await
//                     )).await.unwrap();
//                 }
//                 NetCommand::ActivateVpn(name) => {
//                     let object_path = find_connection(
//                         &name,
//                         &settings.connections().await.unwrap(),
//                         conn.clone()
//                     ).await;
//
//                     if let Some(object_path) = object_path {
//                         let _ = nm.activate_connection(
//                             object_path,
//                             OwnedObjectPath::try_from("/").unwrap(),
//                             OwnedObjectPath::try_from("/").unwrap()
//                         ).await;
//                     }
//                 }
//                 NetCommand::DeactivateVpn(name) => {
//                     let object_path = find_active_connection(
//                         &name,
//                         &nm.active_connections().await.unwrap(),
//                         &conn
//                     ).await;
//
//                     if let Some(object_path) = object_path {
//                         let _ = nm.deactivate_connection(
//                             object_path,
//                         ).await;
//                     }
//                 }
//             }
//         }
//     },
//         },
//     )
// }
