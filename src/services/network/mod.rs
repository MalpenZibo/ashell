use super::{Service, ServiceEvent};
use crate::services::ReadOnlyService;
use dbus::{ActiveConnectionState, ConnectivityState, DeviceState, NetworkDbus};
use iced::{
    futures::{
        channel::mpsc::{unbounded, Sender, UnboundedReceiver, UnboundedSender},
        stream::pending,
        SinkExt, Stream, StreamExt,
    },
    subscription::channel,
    Subscription,
};
use log::{debug, error, info};
use std::{any::TypeId, ops::Deref};
use tokio::process::Command;
use zbus::zvariant::ObjectPath;

mod dbus;

#[derive(Debug, Clone)]
pub enum NetworkEvent {
    WiFiEnabled(bool),
    AirplaneMode(bool),
    Connectivity(ConnectivityState),
    WirelessAccessPoints(Vec<AccessPoint>),
    ActiveConnections(Vec<ActiveConnectionInfo>),
    KnownConnections(Vec<KnownConnection>),
    ScanningNearbyWifi(bool),
}

#[derive(Debug, Clone)]
pub enum NetworkCommand {
    ScanNearbyWifi,
    ToggleWiFi,
    ToggleAirplaneMode,
    ActivateWiFi(String, Option<String>),
    RequestWiFiPassword(String),
    ToggleConnection(String),
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
            _ => {}
        }
    }

    fn subscribe() -> Subscription<ServiceEvent<Self>> {
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

    async fn start_listening(state: State, output: &mut Sender<ServiceEvent<Self>>) -> State {
        match state {
            State::Init => match zbus::Connection::system().await {
                Ok(conn) => {
                    let (tx, rx) = unbounded();

                    let data = NetworkService::initialize_data(&conn).await;

                    match data {
                        Ok(data) => {
                            info!("Network service initialized");

                            let _ = output
                                .send(ServiceEvent::Init(NetworkService {
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

                match NetworkService::events(&conn).await {
                    Ok(mut events) => {
                        while let Some(event) = events.next().await {
                            match event {
                                NetworkEvent::WiFiEnabled(wifi_enabled) => {
                                    debug!("WiFi enabled: {}", wifi_enabled);
                                    let _ = output
                                        .send(ServiceEvent::Update(NetworkEvent::WiFiEnabled(
                                            wifi_enabled,
                                        )))
                                        .await;
                                }
                                _ => {}
                            }
                        }

                        State::Active(conn, rx)
                    }
                    Err(_) => State::Error,
                }
            }
            State::Error => {
                error!("Network service error");

                let _ = pending::<u8>().next().await;

                State::Error
            }
        }
    }

    async fn events(conn: &zbus::Connection) -> anyhow::Result<impl Stream<Item = NetworkEvent>> {
        let nm = NetworkDbus::new(conn).await?;

        Ok(nm.receive_wireless_enabled_changed().await.map(move |_| {
            debug!("WiFi enabled changed");
            NetworkEvent::WiFiEnabled(
                nm.cached_wireless_enabled()
                    .unwrap_or_default()
                    .unwrap_or_default(),
            )
        }))
    }

    async fn set_airplane_mode(conn: &zbus::Connection, airplane_mode: bool) -> anyhow::Result<()> {
        Command::new("rfkill")
            .arg(if airplane_mode { "block" } else { "unblock" })
            .arg("bluetooth")
            .output()
            .await?;

        let nm = NetworkDbus::new(conn).await?;
        nm.set_wireless_enabled(!airplane_mode).await?;

        Ok(())
    }
}

impl Service for NetworkService {
    type Command = NetworkCommand;

    fn command(&self, command: Self::Command) -> iced::Command<ServiceEvent<Self>> {
        debug!("Command: {:?}", command);
        match command {
            NetworkCommand::ToggleAirplaneMode => iced::Command::perform(
                {
                    let conn = self.conn.clone();
                    let airplane_mode = self.airplane_mode;
                    async move {
                        debug!("Toggling airplane mode to: {}", !airplane_mode);
                        let res = Self::set_airplane_mode(&conn, !airplane_mode).await;

                        if res.is_ok() {
                            !airplane_mode
                        } else {
                            airplane_mode
                        }
                    }
                },
                |airplane_mode| ServiceEvent::Update(NetworkEvent::AirplaneMode(airplane_mode)),
            ),
            _ => iced::Command::none(),
        }
    }
}
