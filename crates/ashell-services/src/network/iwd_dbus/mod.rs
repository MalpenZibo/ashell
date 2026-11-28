pub mod adapter;
pub mod agent_manager;
pub mod network;
pub mod station;

// source for dbus: https://git.kernel.org/pub/scm/network/wireless/iwd.git/tree/doc

use super::{
    AccessPointData, AccessPointOf, ActiveConnectionInfo, Backend, ConnectivityState, Event, IWD,
    ManagedObjects, NetworkBackend, NetworkData, Patch, Properties, Redacted, Vpn, best_per_ssid,
    interface, known_access_points, managed_objects, object_path, object_property, property,
};
use crate::bus::{changes_any, interface_properties_changed};
use futures::{
    StreamExt,
    channel::mpsc::UnboundedSender,
    future::{join_all, ready},
    stream::BoxStream,
    stream_select,
};
use log::{debug, warn};
use std::{
    collections::{HashMap, HashSet},
    mem::take,
    sync::{Arc, Mutex, MutexGuard, PoisonError},
};
use zbus::{
    OwnedObjectPath, fdo::ObjectManagerProxy, fdo::PropertiesChanged, interface,
    proxy::CacheProperties,
};

use adapter::AdapterProxy;
use agent_manager::AgentManagerProxy;
use network::NetworkProxy;
use station::StationProxy;

const ADAPTER: &str = "net.connman.iwd.Adapter";
const DEVICE: &str = "net.connman.iwd.Device";
const STATION: &str = "net.connman.iwd.Station";
const KNOWN_NETWORK: &str = "net.connman.iwd.KnownNetwork";
const NETWORK: &str = "net.connman.iwd.Network";

const AGENT_MANAGER_PATH: &str = "/net/connman/iwd";
const PASSWORD_AGENT: &str = "/ashell/pwagent/main";

pub struct IwdDbus<'a> {
    manager: ObjectManagerProxy<'a>,
    passphrases: Passphrases,
    /// The stations `events()` registered a signal level agent on.
    signal_agents: Mutex<Vec<OwnedObjectPath>>,
}

/// The passphrases of the networks being joined, by network path, until iwd
/// asks for them. Shared by every `IwdDbus` of a handle and its one agent.
#[derive(Debug, Clone, Default)]
pub struct Passphrases(Arc<Mutex<HashMap<OwnedObjectPath, Redacted<String>>>>);

impl Passphrases {
    fn lock(&self) -> MutexGuard<'_, HashMap<OwnedObjectPath, Redacted<String>>> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// Map IWD SignalLevelAgent discrete level (bucket index) to UI percentage.
/// Levels correspond to NM's bar ranges: >80% (5 bars), >55% (4 bars), >30% (3 bars), >5% (2 bars), ≤5% (1 bar)
/// Note: Wifi0 is only used for no signal (disconnected), not signal levels.
fn map_iwd_level_to_percent(level: u8) -> u8 {
    match level {
        0 => 90, // Wifi5: Excellent (>80%, mapped to midpoint ~90%)
        1 => 67, // Wifi4: Good (>55% and ≤80%, mapped to midpoint ~67%)
        2 => 42, // Wifi3: Fair (>30% and ≤55%, mapped to midpoint ~42%)
        3 => 17, // Wifi2: Poor (>5% and ≤30%, mapped to midpoint ~17%)
        4 => 2,  // Wifi1: Very Weak (≤5%, mapped to ~2%)
        _ => 0,  // No signal (Wifi0 - disconnected state)
    }
}

/// Map IWD RSSI (reported in hundredths of dBm, e.g. -3900 for -39 dBm) to 0..100 percent.
/// Based on NetworkManager's approach: best = -40 dBm = 100%, worst = -100 dBm = 0%,
/// linear in between. See nm-core-utils.c in NetworkManager.
fn map_iwd_rssi_to_percent(rssi_hundredths: i16) -> u8 {
    let rssi_dbm = rssi_hundredths as f32 / 100.0;

    // Clamp to realistic WiFi range (-100 to -40 dBm)
    let clamped = rssi_dbm.clamp(-100.0, -40.0);

    // Normalize to 0..60 range (where -40 dBm = 0, -100 dBm = 60)
    let normalized = (clamped + 40.0).abs();

    // Calculate percentage: 100% at -40 dBm, 0% at -100 dBm
    let percent = 100.0 - (100.0 * normalized / 60.0);

    percent.round() as u8
}

fn connectivity(station_state: &str) -> ConnectivityState {
    match station_state {
        "inactive" | "disconnected" => ConnectivityState::None,
        "portal" => ConnectivityState::Portal,
        "failed" => ConnectivityState::Loss,
        "connected" => ConnectivityState::Full,
        // scanning, connecting
        _ => ConnectivityState::Unknown,
    }
}

impl NetworkBackend for IwdDbus<'_> {
    async fn read(&self) -> anyhow::Result<NetworkData> {
        let objects = self.objects().await?;
        let stations: HashMap<_, _> = with_interface(&objects, STATION).collect();

        let wireless_devices: Vec<_> = with_interface(&objects, DEVICE)
            .filter(|(_, device)| property::<&str>(device, "Mode") == Some("station"))
            .collect();
        let wifi_present = !wireless_devices.is_empty();

        let wifi_enabled = wireless_devices
            .iter()
            .any(|(_, device)| property(device, "Powered").unwrap_or_default());
        debug!("Wifi enabled: {wifi_enabled}");

        // Signal strength isn't a property: only the stations' ordered networks have it
        let ordered = join_all(
            stations
                .keys()
                .map(|station| self.ordered_networks(station)),
        )
        .await;

        let mut access_points = Vec::new();
        let mut known_ssids = HashSet::new();
        let mut active_connections = Vec::new();
        for (path, rssi) in ordered
            .into_iter()
            .collect::<anyhow::Result<Vec<_>>>()?
            .into_iter()
            .flatten()
        {
            let Some(network) = interface(&objects, &path, NETWORK) else {
                continue;
            };
            let Some(ssid) = property::<&str>(network, "Name") else {
                continue;
            };
            let Some(device_path) = object_property(network, "Device") else {
                continue;
            };
            let strength = map_iwd_rssi_to_percent(rssi);

            let working = stations.get(&device_path).is_some_and(|station| {
                property::<&str>(station, "State") == Some("connected")
                    && object_property(station, "ConnectedNetwork").as_ref() == Some(&path)
            });

            if property(network, "Connected").unwrap_or_default() {
                active_connections.push(ActiveConnectionInfo::WiFi {
                    name: ssid.to_owned(),
                    strength,
                    frequency: 0,
                });
            }
            // Known networks have a KnownNetwork object
            if network.contains_key("KnownNetwork") {
                known_ssids.insert(ssid);
            }

            access_points.push(AccessPointData {
                ssid: ssid.to_owned(),
                strength,
                max_bitrate: 0,
                frequency: 0,
                public: property::<&str>(network, "Type") == Some("open"),
                working,
                path,
                device_path,
            });
        }
        active_connections.sort_by(|a, b| a.name().cmp(b.name()));
        debug!("Active connections: {active_connections:?}");

        let wireless_access_points = best_per_ssid(access_points);
        debug!("Wireless access points: {wireless_access_points:?}");

        let known_connections: Vec<_> =
            known_access_points(&wireless_access_points, &known_ssids).collect();
        debug!("Known connections: {known_connections:?}");

        Ok(NetworkData {
            backend: Backend::Iwd,
            wifi_present,
            active_connections,
            wifi_enabled,
            connectivity: stations
                .values()
                .map(|station| connectivity(property(station, "State").unwrap_or_default()))
                .max()
                .unwrap_or_default(),
            wireless_access_points,
            known_connections,
            scanning_nearby_wifi: stations
                .values()
                .any(|station| property(station, "Scanning").unwrap_or_default()),
        })
    }

    async fn events(&self) -> anyhow::Result<BoxStream<'static, Event>> {
        let conn = self.manager.inner().connection();

        // Subscribe before enumerating stations so station additions/removals cannot be missed.
        let added = self
            .manager
            .receive_interfaces_added()
            .await?
            .filter_map(|signal| {
                ready(signal.args().ok().and_then(|args| {
                    interfaces_event(
                        args.interfaces_and_properties()
                            .keys()
                            .map(|interface| interface.as_str()),
                    )
                }))
            });
        let removed = self
            .manager
            .receive_interfaces_removed()
            .await?
            .filter_map(|signal| {
                ready(signal.args().ok().and_then(|args| {
                    interfaces_event(args.interfaces().iter().map(|interface| interface.as_str()))
                }))
            });

        let properties = interface_properties_changed(
            conn,
            IWD,
            "/net/connman/iwd",
            &[ADAPTER, DEVICE, STATION],
        )
        .await?
        .filter(|signal| ready(shows_change(signal)))
        .map(|_| Event::Changed);

        // iwd forgets agents when it restarts, and a restart starts a new session
        if let Err(err) = self.register_password_agent().await {
            warn!("Failed to register the iwd password agent: {err}");
        }

        let objects = self.objects().await?;
        let (tx, rx) = futures::channel::mpsc::unbounded::<(OwnedObjectPath, u8)>();
        for (station, _) in with_interface(&objects, STATION) {
            self.register_signal_agent(station, tx.clone()).await?;
        }

        let levels = rx.map(|(station, level)| {
            let strength = map_iwd_level_to_percent(level);
            debug!("Signal level on {station} changed: {strength}%");

            Event::Patch(Patch::Strength(AccessPointOf::Station(station), strength))
        });

        Ok(stream_select!(added, removed, properties, levels).boxed())
    }

    async fn unsubscribe(&self) {
        let stations = take(
            &mut *self
                .signal_agents
                .lock()
                .unwrap_or_else(PoisonError::into_inner),
        );
        let server = self.manager.inner().connection().object_server();

        for station in stations {
            let agent = signal_agent_path(&station);
            if let Ok(proxy) = self.station(&station).await
                && let Err(err) = proxy.unregister_signal_level_agent(&agent).await
            {
                debug!("Failed to unregister the signal level agent of {station}: {err}");
            }
            let _ = server.remove::<SignalAgent, _>(&agent).await;
        }
    }

    async fn toggle_wifi(&self) -> anyhow::Result<()> {
        let objects = self.objects().await?;
        let enabled = with_interface(&objects, DEVICE).any(|(_, device)| {
            property::<&str>(device, "Mode") == Some("station")
                && property(device, "Powered").unwrap_or_default()
        });

        for (path, _) in with_interface(&objects, ADAPTER) {
            AdapterProxy::builder(self.manager.inner().connection())
                .destination(IWD)
                .path(path)
                .cache_properties(CacheProperties::No)
                .build()
                .await?
                .set_powered(!enabled)
                .await?;
        }
        Ok(())
    }

    async fn scan(&self) -> anyhow::Result<()> {
        let objects = self.objects().await?;
        for (path, station) in with_interface(&objects, STATION) {
            if property(station, "Scanning").unwrap_or_default() {
                debug!("Already scanning");
                continue;
            }
            self.station(path).await?.scan().await?;
        }
        Ok(())
    }

    async fn select_access_point(
        &self,
        ap: &AccessPointData,
        password: Option<Redacted<String>>,
        connect_once: bool,
    ) -> anyhow::Result<()> {
        // IWD has no volatile join; see
        // https://github.com/MalpenZibo/ashell/issues/509. The UI hides the option here.
        if connect_once {
            warn!("Temporary connections are unsupported on the IWD backend, connecting normally");
        }

        if let Some(password) = password {
            self.passphrases.lock().insert(ap.path.clone(), password);
        }

        let connect = async {
            NetworkProxy::builder(self.manager.inner().connection())
                .destination(IWD)
                .path(ap.path.clone())
                .cache_properties(CacheProperties::No)
                .build()
                .await?
                .connect()
                .await
        };
        let result = connect.await;
        // A passphrase iwd didn't ask for must not answer a later connect
        self.passphrases.lock().remove(&ap.path);

        Ok(result?)
    }

    async fn toggle_vpn(&self, _vpn: &Vpn) -> anyhow::Result<()> {
        Err(anyhow::anyhow!(
            "VPN management not implemented for IWD backend"
        ))
    }
}

/// The objects with `interface`, and its properties.
fn with_interface<'a>(
    objects: &'a ManagedObjects,
    interface: &'a str,
) -> impl Iterator<Item = (&'a OwnedObjectPath, &'a Properties)> {
    objects
        .iter()
        .filter_map(move |(path, interfaces)| Some((path, interfaces.get(interface)?)))
}

/// `Restart` when a station comes or goes, `Changed` for the other objects
/// `NetworkData` shows.
fn interfaces_event<'a>(interfaces: impl Iterator<Item = &'a str>) -> Option<Event> {
    let mut event = None;
    for interface in interfaces {
        match interface {
            STATION => return Some(Event::Restart),
            ADAPTER | DEVICE | KNOWN_NETWORK => event = Some(Event::Changed),
            _ => {}
        }
    }
    event
}

/// Whether a `PropertiesChanged` touches something `NetworkData` shows.
fn shows_change(signal: &PropertiesChanged) -> bool {
    let Ok(args) = signal.args() else {
        return false;
    };

    let shown: &[&str] = match args.interface_name().as_str() {
        ADAPTER => &["Powered"],
        DEVICE => &["Powered", "Mode"],
        STATION => &["State", "Scanning", "ConnectedNetwork"],
        _ => return false,
    };

    changes_any(&args, shown)
}

/// iwd answers AlreadyExists to an agent registered again: it already has it.
fn already_registered(result: zbus::Result<()>) -> zbus::Result<()> {
    match result {
        Err(zbus::Error::MethodError(name, _, _))
            if name.as_str() == "net.connman.iwd.AlreadyExists" =>
        {
            Ok(())
        }
        result => result,
    }
}

/// One path per station, so a registration iwd still holds reaches the agent
/// exported again there.
fn signal_agent_path(station: &OwnedObjectPath) -> OwnedObjectPath {
    let station_id = station.as_str().trim_matches('/').replace('/', "_");
    OwnedObjectPath::try_from(format!("/com/ashell/signalagent/{station_id}"))
        .expect("an object path with its slashes replaced is a valid element")
}

struct SignalAgent {
    tx: UnboundedSender<(OwnedObjectPath, u8)>,
}

#[interface(name = "net.connman.iwd.SignalLevelAgent")]
impl SignalAgent {
    /// Called by iwd whenever RSSI crosses a threshold
    #[zbus(name = "Changed")]
    fn changed(&self, path: OwnedObjectPath, level: u8) {
        debug!(
            "SignalLevelAgent::Changed path={} level={}",
            path.as_str(),
            level
        );
        // The receiver is gone once the session ends
        let _ = self.tx.unbounded_send((path, level));
    }
}

struct PasswordAgent {
    passphrases: Passphrases,
}

#[interface(name = "net.connman.iwd.Agent")]
impl PasswordAgent {
    #[zbus(name = "RequestPassphrase")]
    fn request_passphrase(&self, network_path: OwnedObjectPath) -> zbus::fdo::Result<String> {
        self.passphrases
            .lock()
            .remove(&network_path)
            .map(Redacted::into_inner)
            .ok_or_else(|| {
                warn!("No password available for {network_path}");
                zbus::fdo::Error::Failed("No password set".into())
            })
    }
}

impl IwdDbus<'static> {
    pub async fn new(conn: &zbus::Connection, passphrases: Passphrases) -> anyhow::Result<Self> {
        let manager = ObjectManagerProxy::builder(conn)
            .destination(IWD)
            .path("/")
            .cache_properties(CacheProperties::No)
            .build()
            .await?;

        Ok(Self {
            manager,
            passphrases,
            signal_agents: Mutex::default(),
        })
    }
}

impl IwdDbus<'_> {
    /// Exports the one password agent of this connection and registers it
    /// with iwd. Both are no-ops when already done.
    async fn register_password_agent(&self) -> anyhow::Result<()> {
        self.manager
            .inner()
            .connection()
            .object_server()
            .at(
                PASSWORD_AGENT,
                PasswordAgent {
                    passphrases: self.passphrases.clone(),
                },
            )
            .await?;

        let agent_manager = AgentManagerProxy::builder(self.manager.inner().connection())
            .destination(IWD)
            .path(AGENT_MANAGER_PATH)
            .cache_properties(CacheProperties::No)
            .build()
            .await?;
        already_registered(
            agent_manager
                .register_agent(&object_path(PASSWORD_AGENT))
                .await,
        )?;

        Ok(())
    }

    /// Exports a signal level agent for `station` and registers it, with
    /// thresholds aligned to NM's bar ranges (above 80, 55, 30 and 5 percent),
    /// which NM's linear mapping puts at -52, -67, -82 and -97 dBm.
    async fn register_signal_agent(
        &self,
        station: &OwnedObjectPath,
        tx: UnboundedSender<(OwnedObjectPath, u8)>,
    ) -> anyhow::Result<()> {
        let agent = signal_agent_path(station);
        let server = self.manager.inner().connection().object_server();
        // `at` keeps an agent already exported there, with a sender gone
        let _ = server.remove::<SignalAgent, _>(&agent).await;
        server.at(&agent, SignalAgent { tx }).await?;
        self.signal_agents
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(station.clone());

        let signal_thresholds = [-52, -67, -82, -97];
        already_registered(
            self.station(station)
                .await?
                .register_signal_level_agent(&agent, &signal_thresholds)
                .await,
        )?;

        Ok(())
    }

    async fn objects(&self) -> zbus::Result<ManagedObjects> {
        managed_objects(self.manager.inner().connection(), IWD, "/").await
    }

    async fn station<'p>(&self, path: &'p OwnedObjectPath) -> zbus::Result<StationProxy<'p>> {
        StationProxy::builder(self.manager.inner().connection())
            .destination(IWD)
            .path(path)
            .cache_properties(CacheProperties::No)
            .build()
            .await
    }

    /// The networks a station sees, with their signal strength.
    async fn ordered_networks(
        &self,
        station: &OwnedObjectPath,
    ) -> anyhow::Result<Vec<(OwnedObjectPath, i16)>> {
        Ok(self.station(station).await?.get_ordered_networks().await?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_best_station_connectivity_wins() {
        let best = ["disconnected", "portal", "connecting"]
            .into_iter()
            .map(connectivity)
            .max();

        assert_eq!(best, Some(ConnectivityState::Portal));
    }

    #[test]
    fn signal_agent_paths_are_valid() {
        let station = OwnedObjectPath::try_from("/net/connman/iwd/0/4").unwrap();

        assert_eq!(
            signal_agent_path(&station).as_str(),
            "/com/ashell/signalagent/net_connman_iwd_0_4"
        );
    }
}
