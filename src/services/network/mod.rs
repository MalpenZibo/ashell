use super::{Service, ServiceEvent};
use crate::services::{ReadOnlyService, bluetooth::BluetoothService};
use dbus::ConnectivityState;
use dbus::NetworkDbus;
use iced::futures::TryFutureExt;
use iced::{
    Subscription, Task,
    futures::{
        FutureExt, SinkExt, Stream, StreamExt,
        channel::mpsc::Sender,
        stream::{pending, select_all},
    },
    stream::channel,
};
use iwd_dbus::{
    IwdDbus, //DeviceState,
    //NetworkDbus,
    //NetworkSettingsDbus,
    //WirelessDeviceProxy,
    adapter::AdapterProxy,
    device::DeviceProxy,
    station::StationProxy,
};
use log::{debug, error, info};
use std::{any::TypeId, collections::HashMap, ops::Deref};
use tokio::process::Command;
use zbus::zvariant::{ObjectPath, OwnedObjectPath, Value};

pub mod dbus;
pub mod iwd_dbus;

/// Trait defining the interface for a network backend.
/// This allows abstracting the specific D-Bus implementation (like IWD or NetworkManager).
pub trait NetworkBackend: Send + Sync {
    /// Checks if the dbus server is running.
    async fn is_available(&self) -> anyhow::Result<bool>;

    /// Initializes the backend and fetches the initial network data.
    async fn initialize_data(&self) -> anyhow::Result<NetworkData>;

    /// Subscribes to network events from the backend.
    /// Returns a stream of `NetworkEvent`s.
    //async fn subscribe_events(&self) -> anyhow::Result<impl Stream<Item = NetworkEvent>>;

    /// Toggles the airplane mode.
    async fn set_airplane_mode(&self, enable: bool) -> anyhow::Result<()>;

    /// Scans for nearby Wi-Fi networks.
    async fn scan_nearby_wifi(&self) -> anyhow::Result<()>;

    /// Enables or disables Wi-Fi.
    async fn set_wifi_enabled(&self, enable: bool) -> anyhow::Result<()>;

    /// Connects to a specific access point, potentially with a password.
    /// Returns the updated list of known connections.
    async fn select_access_point(
        &self,
        ap: &AccessPoint,
        password: Option<String>,
    ) -> anyhow::Result<()>;

    async fn known_connections(&self) -> anyhow::Result<Vec<KnownConnection>>;

    /// Enables or disables a VPN connection.
    /// Returns the updated list of known connections.
    async fn set_vpn(
        &self,
        connection_path: OwnedObjectPath,
        enable: bool,
    ) -> anyhow::Result<Vec<KnownConnection>>;
}

#[derive(Debug, Clone)]
pub enum NetworkEvent {
    WiFiEnabled(bool),
    AirplaneMode(bool),
    Connectivity(ConnectivityState),
    WirelessDevice {
        wifi_present: bool,
        wireless_access_points: Vec<AccessPoint>,
    },
    ActiveConnections(Vec<ActiveConnectionInfo>),
    KnownConnections(Vec<KnownConnection>),
    WirelessAccessPoint(Vec<AccessPoint>),
    Strength((String, u8)),
    RequestPasswordForSSID(String),
    ScanningNearbyWifi,
}

#[derive(Debug, Clone)]
pub enum NetworkCommand {
    ScanNearByWiFi,
    ToggleWiFi,
    ToggleAirplaneMode,
    SelectAccessPoint((AccessPoint, Option<String>)),
    ToggleVpn(Vpn),
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct AccessPoint {
    pub ssid: String,
    pub strength: u8,
    pub state: dbus::DeviceState,
    pub public: bool,
    pub working: bool,
    pub path: ObjectPath<'static>,
    pub device_path: ObjectPath<'static>,
}

#[derive(Debug, Clone)]
pub struct Vpn {
    pub name: String,
    pub path: OwnedObjectPath,
}

#[derive(Debug, Clone)]
pub enum KnownConnection {
    AccessPoint(AccessPoint),
    Vpn(Vpn),
}

#[derive(Debug, Clone)]
pub enum ActiveConnectionInfo {
    Wired {
        name: String,
        speed: u32,
    },
    WiFi {
        id: String,
        name: String,
        strength: u8,
    },
    Vpn {
        name: String,
        object_path: OwnedObjectPath,
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
    pub wifi_present: bool,
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
    backend_choice: BackendChoice,
}

impl Deref for NetworkService {
    type Target = NetworkData;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

enum State {
    Init,
    Active(zbus::Connection, BackendChoice),
    Error,
}

impl ReadOnlyService for NetworkService {
    type UpdateEvent = NetworkEvent;
    type Error = ();

    fn update(&mut self, event: Self::UpdateEvent) {
        match event {
            NetworkEvent::AirplaneMode(airplane_mode) => {
                self.data.airplane_mode = airplane_mode;
            }
            NetworkEvent::WiFiEnabled(wifi_enabled) => {
                debug!("WiFi enabled: {}", wifi_enabled);
                self.data.wifi_enabled = wifi_enabled;
            }
            NetworkEvent::ScanningNearbyWifi => {
                self.data.scanning_nearby_wifi = true;
            }
            NetworkEvent::WirelessDevice {
                wifi_present,
                wireless_access_points,
            } => {
                self.data.wifi_present = wifi_present;
                self.data.scanning_nearby_wifi = false;
                self.data.wireless_access_points = wireless_access_points;
            }
            NetworkEvent::ActiveConnections(active_connections) => {
                self.data.active_connections = active_connections;
            }
            NetworkEvent::KnownConnections(known_connections) => {
                self.data.known_connections = known_connections;
            }
            NetworkEvent::Strength((ssid, new_strength)) => {
                if let Some(ap) = self
                    .data
                    .wireless_access_points
                    .iter_mut()
                    .find(|ap| ap.ssid == ssid)
                {
                    ap.strength = new_strength;

                    if let Some(ActiveConnectionInfo::WiFi { strength, .. }) = self
                        .data
                        .active_connections
                        .iter_mut()
                        .find(|ac| ac.name() == ap.ssid)
                    {
                        *strength = new_strength;
                    }
                }
            }
            NetworkEvent::Connectivity(connectivity) => {
                self.data.connectivity = connectivity;
            }
            NetworkEvent::WirelessAccessPoint(wireless_access_points) => {
                self.data.wireless_access_points = wireless_access_points;
            }
            NetworkEvent::RequestPasswordForSSID(_) => {}
        }
    }

    fn subscribe() -> Subscription<ServiceEvent<Self>> {
        let id = TypeId::of::<Self>();

        Subscription::run_with_id(
            id,
            channel(50, async |mut output| {
                let mut state = State::Init;

                loop {
                    state = NetworkService::start_listening(state, &mut output).await;
                }
            }),
        )
    }
}

#[derive(Debug, Clone)]
enum BackendChoice {
    NetworkManager,
    Iwd,
}

impl BackendChoice {
    fn with_connection(self, conn: zbus::Connection) -> BackendChoiceWithConnection {
        BackendChoiceWithConnection { choice: self, conn }
    }
}

struct BackendChoiceWithConnection {
    choice: BackendChoice,
    conn: zbus::Connection,
}

impl NetworkBackend for BackendChoiceWithConnection {
    async fn is_available(&self) -> anyhow::Result<bool> {
        match self.choice {
            BackendChoice::NetworkManager => {
                NetworkDbus::new(&self.conn).await?.is_available().await
            }
            BackendChoice::Iwd => IwdDbus::new(&self.conn).await?.is_available().await,
        }
    }

    async fn initialize_data(&self) -> anyhow::Result<NetworkData> {
        match self.choice {
            BackendChoice::NetworkManager => {
                NetworkDbus::new(&self.conn).await?.initialize_data().await
            }
            BackendChoice::Iwd => IwdDbus::new(&self.conn).await?.initialize_data().await,
        }
    }

    async fn set_airplane_mode(&self, enable: bool) -> anyhow::Result<()> {
        match self.choice {
            BackendChoice::NetworkManager => {
                NetworkDbus::new(&self.conn)
                    .await?
                    .set_airplane_mode(enable)
                    .await
            }
            BackendChoice::Iwd => {
                IwdDbus::new(&self.conn)
                    .await?
                    .set_airplane_mode(enable)
                    .await
            }
        }
    }

    async fn scan_nearby_wifi(&self) -> anyhow::Result<()> {
        match self.choice {
            BackendChoice::NetworkManager => {
                NetworkDbus::new(&self.conn).await?.scan_nearby_wifi().await
            }
            BackendChoice::Iwd => IwdDbus::new(&self.conn).await?.scan_nearby_wifi().await,
        }
    }

    async fn set_wifi_enabled(&self, enable: bool) -> anyhow::Result<()> {
        match self.choice {
            BackendChoice::NetworkManager => {
                NetworkDbus::new(&self.conn)
                    .await?
                    .set_wifi_enabled(enable)
                    .await
            }
            BackendChoice::Iwd => {
                IwdDbus::new(&self.conn)
                    .await?
                    .set_wifi_enabled(enable)
                    .await
            }
        }
    }

    async fn select_access_point(
        &self,
        ap: &AccessPoint,
        password: Option<String>,
    ) -> anyhow::Result<()> {
        match self.choice {
            BackendChoice::NetworkManager => {
                NetworkDbus::new(&self.conn)
                    .await?
                    .select_access_point(ap, password)
                    .await
            }
            BackendChoice::Iwd => {
                IwdDbus::new(&self.conn)
                    .await?
                    .select_access_point(ap, password)
                    .await
            }
        }
    }

    async fn set_vpn(
        &self,
        connection_path: OwnedObjectPath,
        enable: bool,
    ) -> anyhow::Result<Vec<KnownConnection>> {
        match self.choice {
            BackendChoice::NetworkManager => {
                NetworkDbus::new(&self.conn)
                    .await?
                    .set_vpn(connection_path, enable)
                    .await
            }
            // IWD does not handle VPNs directly
            BackendChoice::Iwd => Err(anyhow::anyhow!("IWD does not support VPN management")),
        }
    }

    async fn known_connections(&self) -> anyhow::Result<Vec<KnownConnection>> {
        match self.choice {
            BackendChoice::NetworkManager => {
                NetworkDbus::new(&self.conn)
                    .await?
                    .known_connections()
                    .await
            }
            BackendChoice::Iwd => IwdDbus::new(&self.conn).await?.known_connections().await,
        }
    }
}

impl NetworkService {
    async fn start_listening(state: State, output: &mut Sender<ServiceEvent<Self>>) -> State {
        match state {
            State::Init => match zbus::Connection::system().await {
                Ok(conn) => {
                    // get first backend that is available
                    // TODO: check if i even need "is_available"
                    info!("Connecting to backend");
                    let maybe_backend: Result<(NetworkData, BackendChoice), _> =
                        match NetworkDbus::new(&conn)
                            .and_then(|nm| async move { nm.initialize_data().await })
                            .await
                        {
                            Ok(data) => {
                                info!("NetworkManager service initialized");
                                Ok((data, BackendChoice::NetworkManager))
                            }
                            Err(err) => {
                                info!(
                                    "Failed to initialize NetworkManager. Falling back to iwd. Error: {}",
                                    err
                                );
                                match IwdDbus::new(&conn)
                                    .and_then(|iwd| async move { iwd.initialize_data().await })
                                    .await
                                {
                                    Ok(data) => {
                                        info!("IWD service initialized");
                                        Ok((data, BackendChoice::Iwd))
                                    }
                                    Err(err) => {
                                        error!("Failed to initialize network service: {}", err);
                                        Err(err)
                                    }
                                }
                            }
                        };
                    info!("Connected");

                    match maybe_backend {
                        Ok((data, choice)) => {
                            info!("Network service initialized");
                            let _ = output
                                .send(ServiceEvent::Init(NetworkService {
                                    data,
                                    conn: conn.clone(),
                                    backend_choice: choice.clone(),
                                }))
                                .await;
                            State::Active(conn, choice)
                        }
                        Err(err) => {
                            if err.is::<zbus::Error>() {
                                error!("Failed to connect to system bus: {}", err);
                            } else {
                                error!("Failed to initialize network service: {}", err);
                            }
                            State::Error
                        }
                    }
                }
                Err(err) => {
                    error!("Failed to connect to system bus: {}", err);

                    State::Error
                }
            },
            State::Active(conn, choice) => {
                info!("Listening for network events");

                // TODO: i dont know how to combine the opaque types.. rust streams
                match choice {
                    BackendChoice::NetworkManager => {
                        let nm = NetworkDbus::new(&conn).await.unwrap();

                        match nm.subscribe_events().await {
                            Ok(mut events) => {
                                while let Some(event) = events.next().await {
                                    let mut exit_loop = false;
                                    // TODO: why do we do this?
                                    if let NetworkEvent::WirelessDevice { .. } = event {
                                        exit_loop = true;
                                    }
                                    let _ = output.send(ServiceEvent::Update(event)).await;

                                    if exit_loop {
                                        break;
                                    }
                                }

                                debug!("Network service exit events stream");

                                State::Active(conn, choice)
                            }
                            Err(err) => {
                                error!("Failed to listen for network events: {}", err);

                                State::Error
                            }
                        }
                    }
                    BackendChoice::Iwd => {
                        let iwd = IwdDbus::new(&conn).await.unwrap();
                        match iwd.subscribe_events().await {
                            Ok(mut events) => {
                                while let Some(event) = events.next().await {
                                    info!("Event: {:?}", event);
                                    let mut exit_loop = false;
                                    // TODO: why do we do this?
                                    if let NetworkEvent::WirelessDevice { .. } = event {
                                        exit_loop = true;
                                    }
                                    let _ = output.send(ServiceEvent::Update(event)).await;

                                    if exit_loop {
                                        break;
                                    }
                                }

                                debug!("Network service exit events stream");

                                State::Active(conn, choice)
                            }
                            Err(err) => {
                                error!("Failed to listen for network events: {}", err);

                                State::Error
                            }
                        }
                    }
                }
            }
            State::Error => {
                error!("Network service error");

                let _ = pending::<u8>().next().await;

                State::Error
            }
        }
    }
}

impl Service for NetworkService {
    type Command = NetworkCommand;

    fn command(&mut self, command: Self::Command) -> Task<ServiceEvent<Self>> {
        debug!("Command: {:?}", command);
        let conn = self.conn.clone();
        let bc = self.backend_choice.clone().with_connection(conn);
        match command {
            NetworkCommand::ToggleAirplaneMode => {
                let airplane_mode = self.airplane_mode;

                Task::perform(
                    async move {
                        debug!("Toggling airplane mode to: {}", !airplane_mode);
                        let res = bc.set_airplane_mode(!airplane_mode).await;

                        if res.is_ok() {
                            !airplane_mode
                        } else {
                            airplane_mode
                        }
                    },
                    |airplane_mode| ServiceEvent::Update(NetworkEvent::AirplaneMode(airplane_mode)),
                )
            }
            NetworkCommand::ScanNearByWiFi => Task::perform(
                async move {
                    let _ = bc.scan_nearby_wifi().await;
                },
                |_| ServiceEvent::Update(NetworkEvent::ScanningNearbyWifi),
            ),
            NetworkCommand::ToggleWiFi => {
                let wifi_enabled = self.wifi_enabled;

                Task::perform(
                    async move {
                        let res = bc.set_wifi_enabled(!wifi_enabled).await;

                        if res.is_ok() {
                            !wifi_enabled
                        } else {
                            wifi_enabled
                        }
                    },
                    |wifi_enabled| ServiceEvent::Update(NetworkEvent::WiFiEnabled(wifi_enabled)),
                )
            }
            NetworkCommand::SelectAccessPoint((access_point, password)) => Task::perform(
                async move {
                    bc.select_access_point(&access_point, password)
                        .await
                        .unwrap_or_default();
                    bc.known_connections().await.unwrap_or_default()
                },
                |known_connections| {
                    ServiceEvent::Update(NetworkEvent::KnownConnections(known_connections))
                },
            ),
            NetworkCommand::ToggleVpn(vpn) => {
                let mut active_vpn = self.active_connections.iter().find_map(|kc| match kc {
                    ActiveConnectionInfo::Vpn { name, object_path } if name == &vpn.name => {
                        Some(object_path.clone())
                    }
                    _ => None,
                });

                Task::perform(
                    async move {
                        let (object_path, new_state) = if let Some(active_vpn) = active_vpn.take() {
                            (active_vpn, false)
                        } else {
                            (vpn.path, true)
                        };
                        bc.set_vpn(object_path, new_state).await.unwrap_or_default();
                        let res = bc.known_connections().await;
                        debug!("VPN toggled: {:?}", res);
                        res.unwrap_or_default()
                    },
                    |known_connections| {
                        ServiceEvent::Update(NetworkEvent::KnownConnections(known_connections))
                    },
                )
            }
        }
    }
}
