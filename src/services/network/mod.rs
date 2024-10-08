use super::{Service, ServiceEvent};
use crate::services::{bluetooth::BluetoothService, ReadOnlyService};
use dbus::{
    AccessPointProxy, ActiveConnectionState, ConnectivityState, DeviceProxy, DeviceState,
    NetworkDbus, NetworkSettingsDbus, WirelessDeviceProxy,
};
use iced::{
    futures::{
        channel::mpsc::Sender,
        stream::{pending, select_all},
        stream_select, SinkExt, Stream, StreamExt,
    },
    subscription::channel,
    Subscription,
};
use log::{debug, error, info};
use std::{any::TypeId, collections::HashMap, ops::Deref};
use tokio::process::Command;
use zbus::zvariant::{ObjectPath, OwnedObjectPath};

mod dbus;

#[derive(Debug, Clone)]
pub enum NetworkEvent {
    WiFiEnabled(bool),
    AirplaneMode(bool),
    Connectivity(ConnectivityState),
    Device {
        wifi_present: bool,
        wireless_access_points: Vec<AccessPoint>,
    },
    ActiveConnections(Vec<ActiveConnectionInfo>),
    KnownConnections(Vec<KnownConnection>),
    Strenght((usize, u8)),
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

#[derive(Debug, Clone)]
pub struct AccessPoint {
    pub ssid: String,
    pub strength: u8,
    pub state: DeviceState,
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
        name: String,
        state: ActiveConnectionState,
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
}

impl Deref for NetworkService {
    type Target = NetworkData;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

enum State {
    Init,
    Active(zbus::Connection),
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
            NetworkEvent::Device {
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
            NetworkEvent::Strenght((index, new_strenght)) => {
                if let Some(ap) = self.data.wireless_access_points.get_mut(index) {
                    ap.strength = new_strenght;

                    if let Some(ActiveConnectionInfo::WiFi { strength, .. }) = self
                        .data
                        .active_connections
                        .iter_mut()
                        .find(|ac| ac.name() == ap.ssid)
                    {
                        *strength = new_strenght;
                    }
                }
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
        let bluetooth_soft_blocked = BluetoothService::check_rfkill_soft_block().await?;

        let wifi_present = nm.wifi_device_present().await?;

        let wifi_enabled = nm.wireless_enabled().await.unwrap_or_default();
        debug!("Wifi enabled: {}", wifi_enabled);

        let airplane_mode = bluetooth_soft_blocked && !wifi_enabled;
        debug!("Airplane mode: {}", airplane_mode);

        let active_connections = nm.active_connections().await?;
        debug!("Active connections: {:?}", active_connections);

        let wireless_access_points = nm.wireless_access_points().await?;
        debug!("Wireless access points: {:?}", wireless_access_points);

        let known_connections = nm.known_connections(&wireless_access_points).await?;
        debug!("Known connections: {:?}", known_connections);

        Ok(NetworkData {
            wifi_present,
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
                    let data = NetworkService::initialize_data(&conn).await;

                    match data {
                        Ok(data) => {
                            info!("Network service initialized");

                            let _ = output
                                .send(ServiceEvent::Init(NetworkService {
                                    data,
                                    conn: conn.clone(),
                                }))
                                .await;

                            State::Active(conn)
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
            State::Active(conn) => {
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
                                NetworkEvent::ActiveConnections(active_connections) => {
                                    debug!("Active connections: {:?}", active_connections);
                                    let _ = output
                                        .send(ServiceEvent::Update(
                                            NetworkEvent::ActiveConnections(active_connections),
                                        ))
                                        .await;
                                }
                                NetworkEvent::Device {
                                    wifi_present,
                                    wireless_access_points,
                                } => {
                                    debug!("Wireless access points: {:?}", wireless_access_points);
                                    let _ = output
                                        .send(ServiceEvent::Update(NetworkEvent::Device {
                                            wifi_present,
                                            wireless_access_points,
                                        }))
                                        .await;
                                }
                                NetworkEvent::KnownConnections(known_connections) => {
                                    debug!("Known connections: {:?}", known_connections);
                                    let _ = output
                                        .send(ServiceEvent::Update(NetworkEvent::KnownConnections(
                                            known_connections,
                                        )))
                                        .await;
                                }
                                _ => {}
                            }
                        }

                        State::Active(conn)
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
        let settings = NetworkSettingsDbus::new(conn).await?;

        enum ReceiveEvent {
            WiFiEnabled,
            ActiveConnections,
            Device,
            Strength((usize, u8)),
            KnownConnections,
            PasswordRequested(AccessPoint),
        }

        let wireless_enabled = nm.receive_wireless_enabled_changed().await.map(|_| {
            debug!("WiFi enabled changed");
            ReceiveEvent::WiFiEnabled
        });

        let active_connections = nm.receive_active_connections_changed().await.map(|_| {
            debug!("Active connections changed");
            ReceiveEvent::ActiveConnections
        });

        let devices = nm.receive_devices_changed().await.map(|_| {
            debug!("Wireless access points changed");
            ReceiveEvent::Device
        });

        let wireless_ac = nm.wireless_access_points().await?;

        let mut device_state_changes = Vec::with_capacity(wireless_ac.len());
        for ac in wireless_ac.iter() {
            let dp = DeviceProxy::builder(conn)
                .path(ac.device_path.clone())?
                .build()
                .await?;

            device_state_changes.push(
                dp.receive_state_changed()
                    .await
                    .filter_map(|val| async move {
                        let val = val.get().await;
                        let val = val.map(DeviceState::from).unwrap_or_default();

                        if val == DeviceState::NeedAuth {
                            Some(val)
                        } else {
                            None
                        }
                    })
                    .map(|_| {
                        debug!("Device state changed");
                        ReceiveEvent::PasswordRequested(ac.clone())
                    }),
            );
        }

        let mut ac_changes = Vec::with_capacity(wireless_ac.len());
        for ac in wireless_ac.iter() {
            let dp = WirelessDeviceProxy::builder(conn)
                .path(ac.device_path.clone())?
                .build()
                .await?;

            ac_changes.push(dp.receive_access_points_changed().await);
        }

        let wireless_access_points = nm.wireless_access_points().await?;
        let mut strength_changes = Vec::with_capacity(wireless_access_points.len());
        for (index, ap) in wireless_access_points.iter().enumerate() {
            let app = AccessPointProxy::builder(conn)
                .path(ap.path.clone())?
                .build()
                .await?;

            let current_strength = ap.strength;

            strength_changes.push(
                app.receive_strength_changed()
                    .await
                    .filter_map(move |val| async move {
                        let value = val.get().await.unwrap_or_default();
                        if value.abs_diff(current_strength) > 10 {
                            Some(value)
                        } else {
                            None
                        }
                    })
                    .map(move |v| {
                        debug!("Strength changed value: {}", v);
                        ReceiveEvent::Strength((index, v))
                    })
                    .boxed(),
            );
        }
        let strength_changes = select_all(strength_changes);

        let access_points = select_all(ac_changes).map(|_| {
            debug!("Wireless access points changed");
            ReceiveEvent::Device
        });

        let known_connections = settings.receive_connections_changed().await.map(|_| {
            debug!("Known connections changed");
            ReceiveEvent::KnownConnections
        });

        let events = stream_select!(
            wireless_enabled,
            active_connections,
            devices,
            access_points,
            strength_changes,
            known_connections
        )
        .then({
            let conn = conn.clone();

            move |event| {
                let conn = conn.clone();

                async move {
                    let nm = NetworkDbus::new(&conn).await.unwrap();

                    match event {
                        ReceiveEvent::WiFiEnabled => NetworkEvent::WiFiEnabled(
                            nm.wireless_enabled().await.unwrap_or_default(),
                        ),
                        ReceiveEvent::ActiveConnections => NetworkEvent::ActiveConnections(
                            nm.active_connections().await.unwrap_or_default(),
                        ),
                        ReceiveEvent::Device => NetworkEvent::Device {
                            wifi_present: nm.wifi_device_present().await.unwrap_or_default(),
                            wireless_access_points: nm
                                .wireless_access_points()
                                .await
                                .unwrap_or_default(),
                        },
                        ReceiveEvent::Strength((index, strength)) => {
                            NetworkEvent::Strenght((index, strength))
                        }
                        ReceiveEvent::KnownConnections => {
                            let wireless_access_points =
                                nm.wireless_access_points().await.unwrap_or_default();

                            let known_connections = nm
                                .known_connections(&wireless_access_points)
                                .await
                                .unwrap_or_default();

                            NetworkEvent::KnownConnections(known_connections)
                        }
                        ReceiveEvent::PasswordRequested(ac) => {
                            NetworkEvent::RequestPasswordForSSID(ac.ssid)
                        }
                    }
                }
            }
        })
        .boxed();

        Ok(events)
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

    async fn scan_nearby_wifi(
        conn: &zbus::Connection,
        wireless_devices: Vec<ObjectPath<'static>>,
    ) -> anyhow::Result<()> {
        for device_path in wireless_devices {
            let device = WirelessDeviceProxy::builder(conn)
                .path(device_path)?
                .build()
                .await?;

            device.request_scan(HashMap::new()).await?;
        }

        Ok(())
    }

    async fn set_wifi_enabled(conn: &zbus::Connection, enabled: bool) -> anyhow::Result<()> {
        let nm = NetworkDbus::new(conn).await?;
        nm.set_wireless_enabled(enabled).await?;

        Ok(())
    }

    async fn select_access_point(
        conn: &zbus::Connection,
        access_point: &AccessPoint,
        password: Option<String>,
    ) -> anyhow::Result<Vec<KnownConnection>> {
        let nm = NetworkDbus::new(conn).await?;
        nm.select_access_point(access_point, password).await?;

        let wireless_ac = nm.wireless_access_points().await?;
        let known_connections = nm.known_connections(&wireless_ac).await?;
        Ok(known_connections)
    }

    async fn set_vpn(
        conn: &zbus::Connection,
        connection: OwnedObjectPath,
        state: bool,
    ) -> anyhow::Result<Vec<KnownConnection>> {
        let nm = NetworkDbus::new(conn).await?;

        if state {
            debug!("Activating VPN: {:?}", connection);
            nm.activate_connection(
                connection,
                OwnedObjectPath::try_from("/").unwrap(),
                OwnedObjectPath::try_from("/").unwrap(),
            )
            .await?;
        } else {
            debug!("Deactivating VPN: {:?}", connection);
            nm.deactivate_connection(connection).await?;
        }

        let wireless_ac = nm.wireless_access_points().await?;
        let known_connections = nm.known_connections(&wireless_ac).await?;
        Ok(known_connections)
    }
}

impl Service for NetworkService {
    type Command = NetworkCommand;

    fn command(&mut self, command: Self::Command) -> iced::Command<ServiceEvent<Self>> {
        debug!("Command: {:?}", command);
        match command {
            NetworkCommand::ToggleAirplaneMode => {
                let conn = self.conn.clone();
                let airplane_mode = self.airplane_mode;

                iced::Command::perform(
                    async move {
                        debug!("Toggling airplane mode to: {}", !airplane_mode);
                        let res = Self::set_airplane_mode(&conn, !airplane_mode).await;

                        if res.is_ok() {
                            !airplane_mode
                        } else {
                            airplane_mode
                        }
                    },
                    |airplane_mode| ServiceEvent::Update(NetworkEvent::AirplaneMode(airplane_mode)),
                )
            }
            NetworkCommand::ScanNearByWiFi => {
                let conn = self.conn.clone();
                let wireless_ac = self
                    .wireless_access_points
                    .iter()
                    .map(|ap| ap.path.clone())
                    .collect();

                iced::Command::perform(
                    async move {
                        let _ = NetworkService::scan_nearby_wifi(&conn, wireless_ac).await;
                    },
                    |_| ServiceEvent::Update(NetworkEvent::ScanningNearbyWifi),
                )
            }
            NetworkCommand::ToggleWiFi => {
                let conn = self.conn.clone();
                let wifi_enabled = self.wifi_enabled;

                iced::Command::perform(
                    async move {
                        let res = NetworkService::set_wifi_enabled(&conn, !wifi_enabled).await;

                        if res.is_ok() {
                            !wifi_enabled
                        } else {
                            wifi_enabled
                        }
                    },
                    |wifi_enabled| ServiceEvent::Update(NetworkEvent::WiFiEnabled(wifi_enabled)),
                )
            }
            NetworkCommand::SelectAccessPoint((access_point, password)) => {
                let conn = self.conn.clone();

                iced::Command::perform(
                    async move {
                        let res =
                            NetworkService::select_access_point(&conn, &access_point, password)
                                .await;

                        res.unwrap_or_default()
                    },
                    |known_connections| {
                        ServiceEvent::Update(NetworkEvent::KnownConnections(known_connections))
                    },
                )
            }
            NetworkCommand::ToggleVpn(vpn) => {
                let conn = self.conn.clone();
                let mut active_vpn = self.active_connections.iter().find_map(|kc| match kc {
                    ActiveConnectionInfo::Vpn { name, object_path } if name == &vpn.name => {
                        Some(object_path.clone())
                    }
                    _ => None,
                });

                iced::Command::perform(
                    async move {
                        let (object_path, new_state) = if let Some(active_vpn) = active_vpn.take() {
                            (active_vpn, false)
                        } else {
                            (vpn.path, true)
                        };
                        let res = NetworkService::set_vpn(&conn, object_path, new_state).await;

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
