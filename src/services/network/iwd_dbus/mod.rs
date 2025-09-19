pub mod access_point;
pub mod adapter;
pub mod agent_manager;
pub mod basic_service_set;
pub mod daemon;
pub mod device;
pub mod device_provisioning;
pub mod known_network;
pub mod network;
pub mod service_manager;
pub mod shared_code_device_provisioning;
pub mod simple_configuration;
pub mod station;
pub mod station_diagnostic;

use tokio_stream::wrappers::UnboundedReceiverStream;
use uuid::Uuid;

// source for dbus: https://git.kernel.org/pub/scm/network/wireless/iwd.git/tree/doc
//info!("{:?}",n.inner().introspect().await?); => can use this to generate proxy implementations

use crate::services::bluetooth::BluetoothService;

use zbus::interface;

use super::dbus::DeviceState;
use super::{AccessPoint, ActiveConnectionInfo, KnownConnection, NetworkBackend, NetworkEvent};
use iced::futures::future::join_all;
use iced::futures::stream::select_all;
use iced::futures::{Stream, StreamExt};

use log::{debug, info, warn};
use std::ops::Deref;
use tokio::process::Command;
use zbus::fdo::ObjectManagerProxy;
use zbus::zvariant::OwnedObjectPath;

use access_point::AccessPointProxy;
use adapter::AdapterProxy;
use agent_manager::AgentManagerProxy;
use device::DeviceProxy;
use known_network::KnownNetworkProxy;
use network::NetworkProxy;
use station::StationProxy;

/// Wrapper around the IWD D-Bus ObjectManager
pub struct IwdDbus<'a> {
    _inner: ObjectManagerProxy<'a>,
}

impl<'a> Deref for IwdDbus<'a> {
    type Target = ObjectManagerProxy<'a>;
    fn deref(&self) -> &Self::Target {
        &self._inner
    }
}

#[allow(unused_variables)]
impl super::NetworkBackend for IwdDbus<'_> {
    async fn initialize_data(&self) -> anyhow::Result<super::NetworkData> {
        let nm = self;

        // airplane mode
        let bluetooth_soft_blocked = BluetoothService::check_rfkill_soft_block()
            .await
            .unwrap_or_default();

        let wifi_present = nm.wifi_device_present().await?;

        let wifi_enabled = nm.wireless_enabled().await.unwrap_or_default();
        debug!("Wifi enabled: {wifi_enabled}");

        let airplane_mode = bluetooth_soft_blocked && !wifi_enabled;
        debug!("Airplane mode: {airplane_mode}");

        let active_connections = nm.active_connections_info().await?;
        debug!("Active connections: {active_connections:?}");

        let wireless_access_points = nm.wireless_access_points().await?;
        debug!("Wireless access points: {wireless_access_points:?}");

        let known_connections = nm.known_connections().await?;
        debug!("Known connections: {known_connections:?}");

        let is_scanning = join_all(self.stations().await?.iter().map(|s| s.scanning()))
            .await
            .into_iter()
            .filter_map(|v| v.ok())
            .any(|v| v);

        Ok(super::NetworkData {
            wifi_present,
            active_connections,
            wifi_enabled,
            airplane_mode,
            connectivity: nm
                .connectivity()
                .await?
                .into_iter()
                .map(super::ConnectivityState::from)
                .collect::<Vec<super::ConnectivityState>>()
                .into(),
            wireless_access_points,
            known_connections,
            scanning_nearby_wifi: is_scanning,
        })
    }

    /// List known (provisioned) SSIDs
    async fn known_connections(&self) -> anyhow::Result<Vec<KnownConnection>> {
        let nets = self.reachable_networks().await?;
        let mut networks = Vec::new();
        for (n, s) in nets {
            if n.known_network().await.is_err() {
                continue;
            }
            let ssid = n.name().await?;
            let path = n.inner().path().clone().into();
            let device_path = n.device().await?.clone();
            networks.push(KnownConnection::AccessPoint(AccessPoint {
                ssid,
                path,
                device_path,
                strength: ((s / 100) + 100) as u8,
                state: DeviceState::Unknown, // TODO:
                public: n.type_().await? == "open",
                working: false, // TODO:
            }));
        }
        Ok(networks)
    }

    async fn scan_nearby_wifi(&self) -> anyhow::Result<()> {
        for station in self.stations().await? {
            if station.scanning().await? {
                debug!("Already scanning");
                continue;
            }
            station.scan().await?;
        }
        Ok(())
    }

    async fn set_wifi_enabled(&self, enabled: bool) -> anyhow::Result<()> {
        AdapterProxy::new(self.inner().connection())
            .await?
            .set_powered(enabled)
            .await?;
        Ok(())
    }

    async fn select_access_point(
        &mut self,
        ap: &AccessPoint,
        password: Option<String>,
    ) -> anyhow::Result<()> {
        // Get the agent manager
        let agent_manager = self.agent_manager().await?;

        // If password is provided, register a new agent to handle it
        if let Some(p) = password {
            let path = OwnedObjectPath::try_from("/ashell/pwagent/main").unwrap();

            match agent_manager.unregister_agent(&path).await {
                Ok(_) => info!("Successfully unregistered agent at {path}"),
                Err(e) => info!("Failed to unregister agent at {path}: {e}"),
            }

            // Create a new agent with the password
            let (tx, password_rx) = tokio::sync::mpsc::unbounded_channel::<String>();

            // Register the new agent
            let pw_agent = PWAgent { password_rx };
            self.inner()
                .connection()
                .object_server()
                .at(path.clone(), pw_agent)
                .await?;

            agent_manager.register_agent(&path).await?;

            // Send the password to the agent channel
            tx.send(p)?;
        }

        let net = NetworkProxy::builder(self.inner().connection())
            .destination("net.connman.iwd")?
            .path(ap.path.clone())?
            .build()
            .await?;
        net.connect().await?;
        Ok(())
    }

    async fn set_vpn(
        &self,
        path: OwnedObjectPath,
        enable: bool,
    ) -> anyhow::Result<Vec<KnownConnection>> {
        // IWD doesn't natively support VPN management
        // This would need to be implemented with additional VPN management tools
        Err(anyhow::anyhow!(
            "VPN management not implemented for IWD backend"
        ))
    }

    async fn set_airplane_mode(&self, airplane: bool) -> anyhow::Result<()> {
        Command::new("/usr/sbin/rfkill")
            .arg(if airplane { "block" } else { "unblock" })
            .arg("bluetooth")
            .output()
            .await?;
        self.set_wifi_enabled(!airplane).await?;
        Ok(())
    }
}

/// Macro to simplify listing proxies based on their interface name.
macro_rules! list_proxies {
    ($manager:expr, $interface:expr, $proxy_type:ty) => {
        async {
            let objects = $manager.get_managed_objects().await?;
            let mut proxies = Vec::new();
            for (path, ifs) in objects {
                if ifs.contains_key($interface) {
                    proxies.push(
                        <$proxy_type>::builder($manager.inner().connection())
                            .destination("net.connman.iwd")?
                            .path(path.clone())?
                            .build()
                            .await?,
                    );
                }
            }
            Ok::<_, anyhow::Error>(proxies)
        }
    };
}

#[allow(unused)]
enum IwdStationState {
    Connected,
    Disconnected,
    Connecting,
    Disconnecting,
    Roaming,
}

impl From<String> for IwdStationState {
    fn from(state: String) -> Self {
        match state.as_str() {
            "connected" => IwdStationState::Connected,
            "disconnected" => IwdStationState::Disconnected,
            "connecting" => IwdStationState::Connecting,
            "disconnecting" => IwdStationState::Disconnecting,
            "roaming" => IwdStationState::Roaming,
            _ => IwdStationState::Disconnected,
        }
    }
}

struct SignalAgent {
    tx: tokio::sync::mpsc::UnboundedSender<i16>,
}

#[interface(name = "net.connman.iwd.SignalLevelAgent")]
impl SignalAgent {
    /// Called by iwd whenever RSSI crosses a threshold
    #[zbus(name = "Changed")]
    fn changed(&self, level: i16) {
        // ignore failure if receiver was dropped
        warn!("Signal level changed: {level}");
        let _ = self.tx.send(level);
    }
}

struct PWAgent {
    // Channel for receiving passwords
    password_rx: tokio::sync::mpsc::UnboundedReceiver<String>,
}

#[interface(name = "net.connman.iwd.Agent")]
impl PWAgent {
    #[zbus(name = "RequestPassphrase")]
    async fn request_passphrase(
        &mut self,
        _network_path: OwnedObjectPath,
    ) -> zbus::fdo::Result<String> {
        // Try to receive a password from the channel
        if let Ok(pass) = self.password_rx.try_recv() {
            Ok(pass)
        } else {
            warn!("No password available");
            Err(zbus::fdo::Error::Failed("No password set".into()))
        }
    }
}

#[allow(dead_code, unused_variables)]
impl IwdDbus<'_> {
    /// Connect to the system bus and the IWD service
    pub async fn new(conn: &zbus::Connection) -> anyhow::Result<Self> {
        let manager = ObjectManagerProxy::builder(conn)
            .destination("net.connman.iwd")?
            .path("/")?
            .build()
            .await?;

        Ok(Self { _inner: manager })
    }

    // adapter <- device (station mode) <- station

    pub async fn stations(&'_ self) -> anyhow::Result<Vec<StationProxy<'_>>> {
        list_proxies!(&self._inner, "net.connman.iwd.Station", StationProxy).await
    }

    pub async fn adapters(&'_ self) -> anyhow::Result<Vec<AdapterProxy<'_>>> {
        list_proxies!(&self._inner, "net.connman.iwd.Adapter", AdapterProxy).await
    }

    pub async fn devices(&'_ self) -> anyhow::Result<Vec<DeviceProxy<'_>>> {
        list_proxies!(&self._inner, "net.connman.iwd.Device", DeviceProxy).await
    }

    pub async fn agent_manager(&'_ self) -> anyhow::Result<AgentManagerProxy<'_>> {
        list_proxies!(
            &self._inner,
            "net.connman.iwd.AgentManager",
            AgentManagerProxy
        )
        .await?
        .first()
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("No AgentManagerProxy found"))
    }

    pub async fn known_networks_proxies(&'_ self) -> anyhow::Result<Vec<KnownNetworkProxy<'_>>> {
        list_proxies!(
            &self._inner,
            "net.connman.iwd.KnownNetwork",
            KnownNetworkProxy
        )
        .await
    }

    pub async fn networks_proxies(&'_ self) -> anyhow::Result<Vec<NetworkProxy<'_>>> {
        list_proxies!(&self._inner, "net.connman.iwd.Network", NetworkProxy).await
    }

    pub async fn access_points_proxies(&'_ self) -> anyhow::Result<Vec<AccessPointProxy<'_>>> {
        // Note: AccessPoint interface might not be directly on the root object manager.
        // It might be associated with a Device or Station. This function assumes they might appear.
        // If this doesn't work as expected, the logic might need refinement based on IWD's structure.
        list_proxies!(
            &self._inner,
            "net.connman.iwd.AccessPoint",
            AccessPointProxy
        )
        .await
    }

    pub async fn reachable_networks(&'_ self) -> anyhow::Result<Vec<(NetworkProxy<'_>, i16)>> {
        let stations = self.stations().await?;
        let mut networks = Vec::new();

        for station in stations {
            let networks_proxies = station.get_ordered_networks().await?;
            for (path, strength) in networks_proxies {
                let network = NetworkProxy::builder(self.inner().connection())
                    .destination("net.connman.iwd")?
                    .path(path.clone())?
                    .build()
                    .await?;
                networks.push((network, strength));
            }
        }
        Ok(networks)
    }

    pub async fn subscribe_events(&self) -> anyhow::Result<impl Stream<Item = Vec<NetworkEvent>>> {
        let _conn = self.inner().connection();
        let iwd = self;

        //self.register_psk_agent().await?;

        // --- WiFi Enabled ---
        let mut wireless_enabled_changes = vec![];
        for adapter_proxy in self.adapters().await? {
            let stream = adapter_proxy
                .receive_powered_changed()
                .await
                .then({
                    move |p| async move {
                        // Add move here
                        let value = p.get().await.unwrap_or(false);
                        debug!("Adapter Powered changed: {value}");
                        // We need to check *all* adapters to determine overall wifi state
                        let wifi_enabled = iwd.wireless_enabled().await.unwrap_or(false);
                        vec![NetworkEvent::WiFiEnabled(wifi_enabled)]
                    }
                })
                .boxed();
            wireless_enabled_changes.push(stream);
        }

        // connectivity, access points, strengths and known - all in one
        let stations = self.stations().await?;
        let mut connectivity_changes = vec![];
        let mut ap_s_kap_changes = vec![];
        let mut signal_level_updates = vec![];
        for station in stations {
            // this gets also triggered when connecting to new networks, so no need to listen to
            // network changes
            let cstream = station
                .receive_state_changed()
                .await
                .then({
                    move |p| async move {
                        let value = p.get().await.unwrap_or_default();
                        debug!("Station state changed: {value:?}");
                        // We need to check *all* stations to determine overall wifi state
                        vec![
                            NetworkEvent::Connectivity(
                                iwd.connectivity()
                                    .await
                                    .unwrap_or_default()
                                    .into_iter()
                                    .map(super::ConnectivityState::from)
                                    .collect::<Vec<super::ConnectivityState>>()
                                    .into(),
                            ),
                            NetworkEvent::ActiveConnections(
                                iwd.active_connections_info().await.unwrap_or_default(),
                            ),
                        ]
                    }
                })
                .boxed();
            connectivity_changes.push(cstream);

            let apstream = station
                .receive_scanning_changed()
                .await
                .then({
                    move |s| async move {
                        let is_scanning = s.get().await.unwrap_or(false);

                        let aps = iwd.wireless_access_points().await.unwrap_or_default();
                        let kcs = iwd.known_connections().await.unwrap_or_default();

                        let mut events = vec![
                            NetworkEvent::KnownConnections(kcs),
                            // TODO: Strength((String, u8)), <- responibility for the signal agent
                        ];
                        if is_scanning {
                            debug!("Scanning wifi");
                            events.push(NetworkEvent::ScanningNearbyWifi);
                            // to update list, if scanning stopped use device
                            events.push(NetworkEvent::WirelessAccessPoint(aps));
                        } else {
                            debug!("Stopped scanning wifi");
                            events.push(NetworkEvent::WirelessDevice {
                                // TODO: can we reasonably assume this is true here?
                                wifi_present: iwd.wireless_enabled().await.unwrap_or(false),
                                wireless_access_points: aps,
                            });
                        }
                        events
                    }
                })
                .boxed();
            ap_s_kap_changes.push(apstream);

            // 2) channel
            let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<i16>();
            // 3) export agent
            let agent = SignalAgent { tx };

            let agent_path = OwnedObjectPath::try_from(format!(
                "/com/ashell/signalagent/{}",
                Uuid::new_v4().as_simple()
            ))?;

            let server = self
                .inner()
                .connection()
                .object_server()
                .at(&agent_path, agent)
                .await?;
            // 6) turn receiver into a Stream
            signal_level_updates.push(
                UnboundedReceiverStream::new(rx)
                    .filter_map(|level| async move {
                        debug!("Signal level changed: {level}");
                        // TODO: get current network name
                        Some(vec![NetworkEvent::Strength(("".to_string(), level as u8))])
                    })
                    .boxed(),
            );

            station
                .register_signal_level_agent(&agent_path, &[-40, -50, -60])
                .await?;
            warn!("Registered signal level agent at {agent_path}");
        }

        // TODO: probably would need to listen to interfaces registered and unregistered
        //let devices = nm.wireless_devices().await.unwrap_or_default();

        //let wireless_devices_changed = nm
        //    .receive_devices_changed()
        //    .await
        //    .filter_map({
        //        let conn = conn.clone();
        //        let devices = devices.clone();
        //        move |_| {
        //            let conn = conn.clone();
        //            let devices = devices.clone();
        //            async move {
        //                let nm = NetworkDbus::new(&conn).await.unwrap();

        //                let current_devices = nm.wireless_devices().await.unwrap_or_default();
        //                if current_devices != devices {
        //                    let wifi_present = nm.wifi_device_present().await.unwrap_or_default();
        //                    let wireless_access_points =
        //                        nm.wireless_access_points().await.unwrap_or_default();

        //                    debug!(
        //                        "Wireless device changed: wifi present {:?}, wireless_access_points {:?}",
        //                        wifi_present, wireless_access_points,
        //                    );
        //                    Some(NetworkEvent::WirelessDevice {
        //                        wifi_present,
        //                        wireless_access_points,
        //                    })
        //                } else {
        //                    None
        //                }
        //            }
        //        }
        //    })
        //    .boxed();

        //TODO: likely need to register an auth agent and wait for it here, same goes for network
        //configuration etc - these all are agents registered with IWD - and represent device
        //states

        //// When devices list change I need to update the wireless device state changes
        //let wireless_ac = nm.wireless_access_points().await?;

        //let mut device_state_changes = Vec::with_capacity(wireless_ac.len());
        //for ac in wireless_ac.iter() {
        //    let dp = DeviceProxy::builder(conn)
        //        .path(ac.device_path.clone())?
        //        .build()
        //        .await?;

        //    device_state_changes.push(
        //        dp.receive_state_changed()
        //            .await
        //            .filter_map(|val| async move {
        //                let val = val.get().await;
        //                let val = val.map(DeviceState::from).unwrap_or_default();

        //                if val == DeviceState::NeedAuth {
        //                    Some(val)
        //                } else {
        //                    None
        //                }
        //            })
        //            .map(|_| {
        //                let ssid = ac.ssid.clone();

        //                debug!("Request password for ssid {}", ssid);
        //                NetworkEvent::RequestPasswordForSSID(ssid)
        //            }),
        //    );
        //}

        //let access_points = select_all(ac_changes).boxed();

        let events = select_all(vec![
            select_all(wireless_enabled_changes).boxed(),
            select_all(connectivity_changes).boxed(),
            select_all(ap_s_kap_changes).boxed(),
            select_all(signal_level_updates).boxed(),
            // TODO: add a future that waits for 10s to poll certain information like the signal
            // strength change
        ]);

        Ok(events)
    }

    /// Get the state of all station interfaces
    pub async fn connectivity(&self) -> anyhow::Result<Vec<String>> {
        let mut states = Vec::new();
        for s in self.stations().await? {
            let state = s.state().await?;
            states.push(state);
        }
        Ok(states)
    }

    /// Return true if any device in station mode is present
    pub async fn wifi_device_present(&self) -> anyhow::Result<bool> {
        let devices = self.wireless_devices().await?;

        for d in devices {
            if d.powered().await? {
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// List all networks currently connected (Connected = true)
    pub async fn active_connections(&'_ self) -> anyhow::Result<Vec<(NetworkProxy<'_>, i16)>> {
        let mut networks = Vec::new();
        for (net, strength) in self.reachable_networks().await? {
            if net.connected().await? {
                networks.push((net, strength));
            }
        }
        Ok(networks)
    }

    /// Detailed info on active connections
    pub async fn active_connections_info(&self) -> anyhow::Result<Vec<ActiveConnectionInfo>> {
        // INFO: probably way cleaner with a custom dbus object - SignalLevelAgent

        let nets = self.active_connections().await?;
        let mut info = Vec::new();
        for (net, s) in nets {
            let ssid = net.name().await?;
            // strength not directly on Network; placeholder 0
            info.push(ActiveConnectionInfo::WiFi {
                name: ssid,
                strength: (s / 100 + 100) as u8,
            });
        }
        Ok(info)
    }

    /// List all wireless (station-mode) devices
    pub async fn wireless_devices(&'_ self) -> anyhow::Result<Vec<DeviceProxy<'_>>> {
        let devices = self.devices().await?;
        let mut devs = Vec::new();
        for d in devices {
            if d.mode().await? == "station" {
                devs.push(d);
            }
        }
        Ok(devs)
    }

    /// Scan and list available access points
    pub async fn wireless_access_points(&self) -> anyhow::Result<Vec<AccessPoint>> {
        let mut aps = Vec::new();
        {
            let nets = self.reachable_networks().await?;
            for (net, s) in nets {
                let ssid = net.name().await?;
                let public = net.type_().await? == "open";
                let path = net.inner().path().clone().into();
                let device_path = net.device().await?.clone();
                aps.push(AccessPoint {
                    ssid,
                    state: DeviceState::Unknown, // TODO:
                    // _s is between 0 and -10000
                    // should be between 0 and 100
                    strength: ((s / 100) + 100) as u8,
                    public,
                    working: false, // TODO:
                    path,
                    device_path,
                });
            }
        }
        aps.sort_by(|a, b| b.strength.cmp(&a.strength));
        Ok(aps)
    }

    pub async fn wireless_enabled(&self) -> anyhow::Result<bool> {
        let devs = self.wireless_devices().await?;
        for d in devs {
            if d.powered().await? {
                return Ok(true);
            }
        }
        Ok(false)
    }
}
