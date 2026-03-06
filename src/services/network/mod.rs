use dbus::{ConnectivityState, NetworkDbus};
use futures::StreamExt;
use guido::prelude::*;
use log::{error, info};
use zbus::zvariant::OwnedObjectPath;

pub mod dbus;

pub use dbus::DeviceState;

#[derive(Clone)]
pub enum NetworkCmd {
    ScanNearByWiFi,
    /// bool = current wifi_enabled state (read on main thread)
    ToggleWiFi(bool),
    /// bool = current airplane_mode state (read on main thread)
    ToggleAirplaneMode(bool),
    SelectAccessPoint((AccessPoint, Option<String>)),
    /// (vpn, active_vpn_path if currently connected)
    ToggleVpn(Vpn, Option<OwnedObjectPath>),
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct AccessPoint {
    pub ssid: String,
    pub strength: u8,
    pub state: DeviceState,
    pub public: bool,
    pub working: bool,
    pub path: OwnedObjectPath,
    pub device_path: OwnedObjectPath,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Vpn {
    pub name: String,
    pub path: OwnedObjectPath,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KnownConnection {
    AccessPoint(AccessPoint),
    Vpn(Vpn),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ActiveConnectionInfo {
    Wired {
        name: String,
    },
    WiFi {
        name: String,
        strength: u8,
    },
    Vpn {
        name: String,
        object_path: OwnedObjectPath,
    },
}

impl ActiveConnectionInfo {
    #[allow(dead_code)]
    pub fn name(&self) -> &str {
        match self {
            Self::Wired { name, .. } => name,
            Self::WiFi { name, .. } => name,
            Self::Vpn { name, .. } => name,
        }
    }
}

#[derive(Clone, PartialEq, guido::SignalFields)]
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

impl Default for NetworkData {
    fn default() -> Self {
        Self {
            wifi_present: false,
            wireless_access_points: Vec::new(),
            active_connections: Vec::new(),
            known_connections: Vec::new(),
            wifi_enabled: false,
            airplane_mode: false,
            connectivity: ConnectivityState::Unknown,
            scanning_nearby_wifi: false,
        }
    }
}

pub fn create() -> (NetworkDataSignals, Service<NetworkCmd>) {
    let data = NetworkDataSignals::new(NetworkData::default());
    let svc = start_network_service(data.writers());
    (data, svc)
}

async fn refresh_network_data(conn: &zbus::Connection, writers: &NetworkDataWriters) {
    let Ok(nm) = NetworkDbus::new(conn).await else {
        return;
    };
    let wifi_present = nm.wifi_device_present().await.unwrap_or_default();
    let wifi_enabled = nm.wireless_enabled().await.unwrap_or_default();
    let active_connections = nm.active_connections_info().await.unwrap_or_default();
    let wireless_access_points = nm.wireless_access_points().await.unwrap_or_default();
    let known_connections = nm.known_connections().await.unwrap_or_default();
    let connectivity = nm.connectivity().await.unwrap_or_default();
    let bt_blocked = check_rfkill_soft_block().await;
    let airplane_mode = bt_blocked && !wifi_enabled;

    writers.set(NetworkData {
        wifi_present,
        wireless_access_points,
        active_connections,
        known_connections,
        wifi_enabled,
        airplane_mode,
        connectivity,
        scanning_nearby_wifi: false,
    });
}

fn start_network_service(writers: NetworkDataWriters) -> Service<NetworkCmd> {
    create_service::<NetworkCmd, _, _>(move |mut rx, ctx| async move {
        let conn = match zbus::Connection::system().await {
            Ok(c) => c,
            Err(e) => {
                error!("Failed to connect to system bus: {e}");
                return;
            }
        };

        let nm = match dbus::NetworkManagerProxy::new(&conn).await {
            Ok(nm) => nm,
            Err(e) => {
                error!("Failed to create NetworkManager proxy: {e}");
                return;
            }
        };

        // Initialize data
        refresh_network_data(&conn, &writers).await;
        info!("Network service initialized");

        // Set up event streams directly from proxy (keeps proxy alive)
        let mut wireless_enabled = nm
            .receive_wireless_enabled_changed()
            .await
            .map(|_| ())
            .boxed();
        let mut connectivity = nm.receive_connectivity_changed().await.map(|_| ()).boxed();
        let mut active_conns = nm
            .receive_active_connections_changed()
            .await
            .map(|_| ())
            .boxed();
        let mut devices_changed = nm.receive_devices_changed().await.map(|_| ()).boxed();

        // Settings proxy for known connection changes
        let settings = dbus::SettingsProxy::new(&conn).await.ok();
        let mut connections_changed = match &settings {
            Some(s) => s.receive_connections_changed().await.map(|_| ()).boxed(),
            None => futures::stream::pending().boxed(),
        };

        while ctx.is_running() {
            tokio::select! {
                cmd = rx.recv() => {
                    match cmd {
                        Some(cmd) => handle_network_cmd(&conn, &writers, cmd).await,
                        None => break,
                    }
                }
                _ = wireless_enabled.next() => {
                    refresh_network_data(&conn, &writers).await;
                }
                _ = connectivity.next() => {
                    refresh_network_data(&conn, &writers).await;
                }
                _ = active_conns.next() => {
                    refresh_network_data(&conn, &writers).await;
                }
                _ = devices_changed.next() => {
                    refresh_network_data(&conn, &writers).await;
                }
                _ = connections_changed.next() => {
                    refresh_network_data(&conn, &writers).await;
                }
            }
        }
    })
}

async fn handle_network_cmd(
    conn: &zbus::Connection,
    writers: &NetworkDataWriters,
    cmd: NetworkCmd,
) {
    let nm = match NetworkDbus::new(conn).await {
        Ok(nm) => nm,
        Err(e) => {
            error!("Failed to create NetworkDbus for command: {e}");
            return;
        }
    };

    match cmd {
        NetworkCmd::ToggleWiFi(current) => {
            if nm.set_wifi_enabled(!current).await.is_ok() {
                writers.wifi_enabled.set(!current);
            }
        }
        NetworkCmd::ToggleAirplaneMode(current) => {
            if nm.set_airplane_mode(!current).await.is_ok() {
                writers.airplane_mode.set(!current);
            }
        }
        NetworkCmd::ScanNearByWiFi => {
            writers.scanning_nearby_wifi.set(true);
            let _ = nm.scan_nearby_wifi().await;
        }
        NetworkCmd::SelectAccessPoint((ap, password)) => {
            let _ = nm.select_access_point(&ap, password).await;
            if let Ok(kc) = nm.known_connections().await {
                writers.known_connections.set(kc);
            }
        }
        NetworkCmd::ToggleVpn(vpn, active_path) => {
            if let Some(active_path) = active_path {
                let _ = nm.set_vpn(active_path, false).await;
            } else {
                let _ = nm.set_vpn(vpn.path, true).await;
            }
            if let Ok(kc) = nm.known_connections().await {
                writers.known_connections.set(kc);
            }
        }
    }
}

async fn check_rfkill_soft_block() -> bool {
    let output = tokio::process::Command::new("rfkill")
        .args(["list", "bluetooth"])
        .output()
        .await;
    match output {
        Ok(o) => String::from_utf8_lossy(&o.stdout).contains("Soft blocked: yes"),
        Err(_) => false,
    }
}
