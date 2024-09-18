use crate::services::{ReadOnlyService, Service};
use dbus::{ActiveConnectionState, ConnectivityState, DeviceState, NetworkDbus};
use iced::{
    futures::{
        channel::mpsc::{unbounded, Sender, UnboundedReceiver, UnboundedSender},
        stream::pending,
        SinkExt, StreamExt,
    },
    subscription::channel,
    Subscription,
};
use log::{debug, error, info};
use std::{any::TypeId, ops::Deref};
use zbus::zvariant::ObjectPath;

mod dbus;

#[derive(Debug, Clone)]
pub enum NetworkEvent {
    Init(NetworkService),
}

#[derive(Debug, Clone)]
pub enum NetworkCommand {
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
}
