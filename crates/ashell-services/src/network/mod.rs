//! Network state and control through NetworkManager or, without it, iwd.

use crate::{
    bus,
    retry::{RETRY_DELAY, failed},
    stream::{channel, send},
};
use dbus::{NetworkDbus, Profiles};
use futures::{
    FutureExt, SinkExt, Stream, StreamExt,
    channel::mpsc::Sender,
    future::{Either, ready, select},
    stream::{self, BoxStream},
    try_join,
};
use iwd_dbus::{IwdDbus, Passphrases};
use log::{debug, error, info};
use std::{
    cmp::Reverse,
    collections::{HashMap, HashSet},
    mem::{replace, take},
    pin::pin,
};
use tokio::sync::watch;
use zbus::{ObjectPath, OwnedObjectPath, OwnedVariant, Variant, proxy, proxy::CacheProperties};

mod dbus;
mod iwd_dbus;

const NETWORK_MANAGER: &str = "org.freedesktop.NetworkManager";
const IWD: &str = "net.connman.iwd";

/// A value that is never printed by `Debug`.
///
/// Wrap secrets in this and keep `#[derive(Debug)]` on the containing type:
/// a future variant cannot leak by forgetting a manual `Debug` impl.
#[derive(Clone)]
pub struct Redacted<T>(T);

impl<T> Redacted<T> {
    pub const fn new(value: T) -> Self {
        Self(value)
    }

    pub fn into_inner(self) -> T {
        self.0
    }

    pub fn as_inner(&self) -> &T {
        &self.0
    }
}

impl<T> std::fmt::Debug for Redacted<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("<redacted>")
    }
}

/// The daemon behind the network state. NetworkManager wins when both run,
/// including when it uses iwd as its Wi-Fi backend.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Backend {
    NetworkManager,
    Iwd,
}

impl Backend {
    fn bus_name(self) -> &'static str {
        match self {
            Backend::NetworkManager => NETWORK_MANAGER,
            Backend::Iwd => IWD,
        }
    }
}

/// How far the network reaches, worst first: several devices report the
/// best of them.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ConnectivityState {
    #[default]
    Unknown,
    None,
    Portal,
    Loss,
    Full,
}

/// The `T:` field of a `WIFI:` URI. SAE stays distinct because WPA3-only
/// access points reject `T:WPA`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WifiSecurity {
    Wpa,
    Sae,
}

impl WifiSecurity {
    fn qr_auth_type(self) -> &'static str {
        match self {
            WifiSecurity::Wpa => "WPA",
            WifiSecurity::Sae => "SAE",
        }
    }
}

#[derive(Debug, Clone)]
pub enum PskOutcome {
    Password {
        psk: Redacted<String>,
        security: WifiSecurity,
        /// The QR code needs `H:true` or scanners won't find the network.
        hidden: bool,
    },
    Open {
        hidden: bool,
    },
    /// Secured, but the passphrase isn't readable (agent-owned, not-saved,
    /// or polkit denied).
    PasswordUnavailable,
    /// Enterprise, WEP or OWE: not encodable in a `WIFI:` URI.
    Unsupported,
}

impl PskOutcome {
    /// The `WIFI:` URI a QR code carries to join `ssid`, if it has one.
    pub fn wifi_uri(&self, ssid: &str) -> Option<String> {
        let (password, hidden) = match self {
            PskOutcome::Password {
                psk,
                security,
                hidden,
            } => (Some((psk.as_inner().as_str(), *security)), *hidden),
            PskOutcome::Open { hidden } => (None, *hidden),
            PskOutcome::PasswordUnavailable | PskOutcome::Unsupported => return None,
        };

        let ssid = wifi_escape(ssid);
        let hidden = if hidden { "H:true;" } else { "" };
        // The last ';' is the empty field that terminates the URI (`;;`).
        Some(match password {
            Some((password, security)) if !password.is_empty() => format!(
                "WIFI:T:{};S:{ssid};P:{};{hidden};",
                security.qr_auth_type(),
                wifi_escape(password),
            ),
            _ => format!("WIFI:T:nopass;S:{ssid};{hidden};"),
        })
    }
}

fn wifi_escape(s: &str) -> String {
    // https://github.com/zxing/zxing/wiki/Barcode-Contents#wifi-network-config
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\\' | ';' | ',' | ':' | '"' => {
                out.push('\\');
                out.push(c);
            }
            _ => out.push(c),
        }
    }
    out
}

#[derive(Debug, Clone)]
pub enum NetworkCommand {
    ScanNearByWiFi,
    ToggleWiFi,
    /// `connect_once` avoids persisting the profile to disk. iwd can't, and
    /// connects normally.
    SelectAccessPoint {
        access_point: AccessPointData,
        password: Option<Redacted<String>>,
        connect_once: bool,
    },
    ToggleVpn(Vpn),
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct AccessPointData {
    pub ssid: String,
    pub strength: u8,
    pub max_bitrate: u32,
    pub frequency: u32,
    pub public: bool,
    /// The access point the device is connected through.
    pub working: bool,
    pub path: OwnedObjectPath,
    pub device_path: OwnedObjectPath,
}

impl AccessPointData {
    /// The one in use first, so its SSID keeps its path, then by max bitrate,
    /// frequency and strength: higher is better.
    fn is_better(&self, other: &Self) -> bool {
        (
            self.working,
            self.max_bitrate,
            self.frequency,
            self.strength,
        ) > (
            other.working,
            other.max_bitrate,
            other.frequency,
            other.strength,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Vpn {
    pub name: String,
    pub path: OwnedObjectPath,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KnownConnection {
    AccessPoint(AccessPointData),
    Vpn(Vpn),
}

/// A Wi-Fi frequency band.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Band {
    TwoPointFourGhz,
    FiveGhz,
    SixGhz,
    SixtyGhz,
}

impl Band {
    pub fn from_frequency(frequency_mhz: u32) -> Option<Self> {
        match frequency_mhz {
            2400..=2500 => Some(Band::TwoPointFourGhz),
            4900..=5924 => Some(Band::FiveGhz),
            5925..=7125 => Some(Band::SixGhz),
            57000..=71000 => Some(Band::SixtyGhz),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActiveConnectionInfo {
    Wired {
        name: String,
    },
    WiFi {
        name: String,
        strength: u8,
        frequency: u32,
    },
    Vpn {
        name: String,
        object_path: OwnedObjectPath,
    },
}

impl ActiveConnectionInfo {
    pub fn name(&self) -> &str {
        match self {
            Self::Wired { name, .. } | Self::WiFi { name, .. } | Self::Vpn { name, .. } => name,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetworkData {
    pub backend: Backend,
    pub wifi_present: bool,
    /// One per SSID, the strongest first.
    pub wireless_access_points: Vec<AccessPointData>,
    pub active_connections: Vec<ActiveConnectionInfo>,
    /// The visible access points with a saved profile, in the same order, then
    /// the VPNs by name.
    pub known_connections: Vec<KnownConnection>,
    pub wifi_enabled: bool,
    pub connectivity: ConnectivityState,
    pub scanning_nearby_wifi: bool,
}

impl NetworkData {
    /// Whether a saved Wi-Fi profile exists for `ssid`.
    pub fn is_known_ssid(&self, ssid: &str) -> bool {
        self.known_connections
            .iter()
            .any(|c| matches!(c, KnownConnection::AccessPoint(ap) if ap.ssid == ssid))
    }

    /// Whether joining `ssid` can skip saving it. Needs backend support and no
    /// saved profile, since an existing profile is reused as is.
    /// See <https://github.com/MalpenZibo/ashell/issues/509> for IWD support.
    pub fn can_connect_once(&self, ssid: &str) -> bool {
        self.backend == Backend::NetworkManager && !self.is_known_ssid(ssid)
    }

    /// Whether [`Network::read_psk`] can read the passphrase of the network in use.
    pub fn can_read_psk(&self) -> bool {
        self.backend == Backend::NetworkManager
    }
}

/// Wi-Fi devices NetworkManager scans on at our request. It has no property
/// that tells, only `LastScan` once a scan is done.
type PendingScans = watch::Sender<HashSet<OwnedObjectPath>>;

/// What `updates()` and `execute()` need from NetworkManager or iwd.
trait NetworkBackend: Sync {
    fn read(&self) -> impl Future<Output = anyhow::Result<NetworkData>> + Send;

    /// Everything that can change what [`read`](Self::read) returns,
    /// subscribed once.
    fn events(&self) -> impl Future<Output = anyhow::Result<BoxStream<'static, Event>>> + Send;

    /// Undoes what [`events`](Self::events) registered with the daemon, once
    /// its stream is dropped.
    fn unsubscribe(&self) -> impl Future<Output = ()> + Send {
        async {}
    }

    fn toggle_wifi(&self) -> impl Future<Output = anyhow::Result<()>> + Send;

    fn scan(&self) -> impl Future<Output = anyhow::Result<()>> + Send;

    fn select_access_point(
        &self,
        ap: &AccessPointData,
        password: Option<Redacted<String>>,
        connect_once: bool,
    ) -> impl Future<Output = anyhow::Result<()>> + Send;

    /// Disconnects `vpn` when it's active, connects it otherwise.
    fn toggle_vpn(&self, vpn: &Vpn) -> impl Future<Output = anyhow::Result<()>> + Send;
}

enum Event {
    /// The backend to use now.
    Backend(Option<Backend>),
    /// Something shown in `NetworkData` may have changed: read it again.
    Changed,
    /// iwd's stations changed: subscribe again.
    Restart,
    Patch(Patch),
}

/// A change applied to the last snapshot, so frequent ones don't read it all.
enum Patch {
    /// A new signal strength for an access point.
    Strength(AccessPointOf, u8),
    Scanning(bool),
}

enum AccessPointOf {
    /// NetworkManager: the access point at this path.
    Path(OwnedObjectPath),
    /// iwd: the one the station at this path is connected through.
    Station(OwnedObjectPath),
}

impl Patch {
    /// Whether `data` changed.
    fn apply(&self, data: &mut NetworkData) -> bool {
        match self {
            Patch::Strength(of, strength) => {
                let Some(access_point) =
                    data.wireless_access_points.iter_mut().find(|ap| match of {
                        AccessPointOf::Path(path) => &ap.path == path,
                        AccessPointOf::Station(station) => &ap.device_path == station && ap.working,
                    })
                else {
                    return false;
                };
                if replace(&mut access_point.strength, *strength) == *strength {
                    return false;
                }
                let path = access_point.path.clone();
                let ssid = access_point.ssid.clone();

                for known in &mut data.known_connections {
                    if let KnownConnection::AccessPoint(known) = known
                        && known.path == path
                    {
                        known.strength = *strength;
                    }
                }
                if let Some(active) =
                    data.active_connections
                        .iter_mut()
                        .find_map(|connection| match connection {
                            ActiveConnectionInfo::WiFi { name, strength, .. } if *name == ssid => {
                                Some(strength)
                            }
                            _ => None,
                        })
                {
                    *active = *strength;
                }

                sort_access_points(&mut data.wireless_access_points);
                // Stable: the VPNs keep their order after the access points
                data.known_connections.sort_by_key(|known| match known {
                    KnownConnection::AccessPoint(ap) => {
                        (0, Reverse(ap.strength), Some(ap.ssid.clone()))
                    }
                    KnownConnection::Vpn(_) => (1, Reverse(0), None),
                });
                true
            }
            Patch::Scanning(scanning) => {
                replace(&mut data.scanning_nearby_wifi, *scanning) != *scanning
            }
        }
    }
}

/// The changes to make before the next snapshot.
#[derive(Default)]
struct Changes {
    read: bool,
    patches: Vec<Patch>,
}

impl Changes {
    /// Records `event`, or tells how listening to `current` ends.
    fn add(&mut self, event: Event, current: Backend) -> Option<Outcome> {
        match event {
            Event::Backend(backend) if backend == Some(current) => {}
            Event::Backend(backend) => return Some(Outcome::Backend(backend)),
            Event::Changed => self.read = true,
            Event::Restart => return Some(Outcome::Backend(Some(current))),
            Event::Patch(patch) => self.patches.push(patch),
        }
        None
    }
}

/// Records the events already queued: one change fires a burst of signals.
fn drain_ready(
    events: &mut (impl Stream<Item = Event> + Unpin),
    changes: &mut Changes,
    current: Backend,
) -> Option<Outcome> {
    while let Some(event) = events.next().now_or_never().flatten() {
        if let Some(outcome) = changes.add(event, current) {
            return Some(outcome);
        }
    }
    None
}

/// How listening to one backend ended.
enum Outcome {
    /// The backend to use now: the same one when it has to be subscribed again.
    Backend(Option<Backend>),
    Failed(anyhow::Error),
    /// Nobody listens anymore, or the bus went away.
    Closed,
}

#[derive(Debug, Clone)]
pub struct Network {
    conn: zbus::Connection,
    pending_scans: PendingScans,
    profiles: Profiles,
    passphrases: Passphrases,
}

impl Network {
    pub async fn connect() -> anyhow::Result<Self> {
        let conn = zbus::Connection::system().await?;

        Ok(Self {
            conn,
            pending_scans: watch::Sender::new(HashSet::new()),
            profiles: Profiles::default(),
            passphrases: Passphrases::default(),
        })
    }

    /// Yields the current state once subscribed, then a fresh snapshot on every change.
    /// `None` while neither NetworkManager nor iwd runs: the stream waits for
    /// one of them to appear.
    pub fn updates(&self) -> impl Stream<Item = Option<NetworkData>> + Send + 'static {
        let network = self.clone();

        channel(100, |output| async move {
            let backends = match backend_watch(&network.conn).await {
                Ok(backends) => backends,
                Err(err) => {
                    error!("Failed to watch for network backends on the system bus: {err}");
                    return;
                }
            };

            Listener {
                network,
                backends,
                output,
                last: None,
                failing: false,
            }
            .run()
            .await;
        })
    }

    pub async fn execute(&self, command: NetworkCommand) -> anyhow::Result<()> {
        debug!("Network command: {command:?}");

        let backend = current_backend(&self.conn)
            .await?
            .ok_or_else(|| anyhow::anyhow!("No network backend (NetworkManager or iwd) running"))?;
        match backend {
            Backend::NetworkManager => run(&self.network_manager().await?, command).await,
            Backend::Iwd => run(&self.iwd().await?, command).await,
        }
    }

    /// Reads the stored passphrase of `ssid`, the Wi-Fi network in use, to
    /// share it. NetworkManager only.
    pub async fn read_psk(&self, ssid: &str) -> anyhow::Result<PskOutcome> {
        match current_backend(&self.conn).await? {
            Some(Backend::NetworkManager) => self.network_manager().await?.read_psk(ssid).await,
            _ => Err(anyhow::anyhow!(
                "Wi-Fi sharing requires the NetworkManager backend"
            )),
        }
    }

    async fn network_manager(&self) -> anyhow::Result<NetworkDbus<'static>> {
        NetworkDbus::new(
            &self.conn,
            self.pending_scans.clone(),
            self.profiles.clone(),
        )
        .await
    }

    async fn iwd(&self) -> anyhow::Result<IwdDbus<'static>> {
        IwdDbus::new(&self.conn, self.passphrases.clone()).await
    }
}

/// The state behind one `updates()` stream.
struct Listener {
    network: Network,
    backends: BoxStream<'static, Option<Backend>>,
    output: Sender<Option<NetworkData>>,
    last: Option<Option<NetworkData>>,
    /// Whether the last read failed, so a run of failures warns once.
    failing: bool,
}

impl Listener {
    async fn run(mut self) {
        let Some(mut backend) = self.backends.next().await else {
            return;
        };

        loop {
            let Some(current) = backend else {
                self.failing = false;
                if self.last != Some(None) {
                    info!("No network backend found (NetworkManager or iwd), waiting for one");
                }
                if !send(&mut self.output, &mut self.last, None).await {
                    return;
                }
                match self.backends.next().await {
                    Some(next) => backend = next,
                    None => return,
                }
                continue;
            };

            info!("Listening for {current:?} network events");
            let outcome = match current {
                Backend::NetworkManager => match self.network.network_manager().await {
                    Ok(nm) => self.listen(current, &nm).await,
                    Err(err) => Outcome::Failed(err),
                },
                Backend::Iwd => match self.network.iwd().await {
                    Ok(iwd) => self.listen(current, &iwd).await,
                    Err(err) => Outcome::Failed(err),
                },
            };

            match outcome {
                Outcome::Backend(next) => backend = next,
                Outcome::Closed => return,
                Outcome::Failed(err) => {
                    failed!(&mut self.failing, "Failed to read the network state: {err}");
                    tokio::time::sleep(RETRY_DELAY).await;

                    while let Some(next) = self.backends.next().now_or_never() {
                        match next {
                            Some(next) => backend = next,
                            None => return,
                        }
                    }
                }
            }
        }
    }

    async fn listen(&mut self, current: Backend, backend: &impl NetworkBackend) -> Outcome {
        let outcome = self.follow(current, backend).await;
        backend.unsubscribe().await;
        outcome
    }

    async fn follow(&mut self, current: Backend, backend: &impl NetworkBackend) -> Outcome {
        let events = match backend.events().await {
            Ok(events) => events,
            Err(err) => return Outcome::Failed(err),
        };
        let backends = (&mut self.backends).map(Event::Backend);
        let mut events = stream::select(events, backends);

        let mut changes = Changes {
            read: true,
            patches: Vec::new(),
        };

        loop {
            if !changes.read && changes.patches.is_empty() {
                let Some(event) = events.next().await else {
                    return Outcome::Closed;
                };
                if let Some(outcome) = changes.add(event, current) {
                    return outcome;
                }
            }
            if let Some(outcome) = drain_ready(&mut events, &mut changes, current) {
                return outcome;
            }

            let Changes { read, patches } = take(&mut changes);

            if !read {
                let Some(Some(last)) = self.last.as_mut() else {
                    continue;
                };
                let mut changed = false;
                for patch in &patches {
                    changed |= patch.apply(last);
                }
                if changed && self.output.send(Some(last.clone())).await.is_err() {
                    return Outcome::Closed;
                }
                continue;
            }

            // Keep consuming events while waiting for the reply: once a zbus
            // signal queue is full, the connection stops reading replies too.
            let mut reading = pin!(backend.read());
            let result = loop {
                match select(reading.as_mut(), events.next()).await {
                    Either::Left((result, _)) => break result,
                    Either::Right((Some(event), _)) => {
                        if let Some(outcome) = changes.add(event, current) {
                            return outcome;
                        }
                    }
                    Either::Right((None, _)) => return Outcome::Closed,
                }
            };

            let mut data = match result {
                Ok(data) => {
                    self.failing = false;
                    data
                }
                Err(err) => {
                    // An object that vanished mid-read fails it: the change is on its way
                    if let Some(outcome) = drain_ready(&mut events, &mut changes, current) {
                        return outcome;
                    }
                    // So does the daemon stopping, before its owner change arrives
                    if let Ok(false) = bus::has_owner(&self.network.conn, current.bus_name()).await
                    {
                        return match current_backend(&self.network.conn).await {
                            Ok(backend) => Outcome::Backend(backend),
                            Err(err) => Outcome::Failed(err),
                        };
                    }
                    if !changes.read {
                        return Outcome::Failed(err);
                    }
                    debug!("Network state changed while reading it, reading again: {err}");
                    continue;
                }
            };

            for patch in &patches {
                patch.apply(&mut data);
            }

            if !send(&mut self.output, &mut self.last, Some(data)).await {
                return Outcome::Closed;
            }
        }
    }
}

async fn run(backend: &impl NetworkBackend, command: NetworkCommand) -> anyhow::Result<()> {
    match command {
        NetworkCommand::ScanNearByWiFi => backend.scan().await,
        NetworkCommand::ToggleWiFi => backend.toggle_wifi().await,
        NetworkCommand::SelectAccessPoint {
            access_point,
            password,
            connect_once,
        } => {
            backend
                .select_access_point(&access_point, password, connect_once)
                .await
        }
        NetworkCommand::ToggleVpn(vpn) => backend.toggle_vpn(&vpn).await,
    }
}

/// The backend to use right now. Asked to the bus itself, so it never
/// activates either daemon.
async fn current_backend(conn: &zbus::Connection) -> anyhow::Result<Option<Backend>> {
    let (network_manager, iwd) = try_join!(
        bus::has_owner(conn, NETWORK_MANAGER),
        bus::has_owner(conn, IWD)
    )?;

    Ok(choose_backend(network_manager, iwd))
}

/// The backend to use: the current one, then one per change.
async fn backend_watch(
    conn: &zbus::Connection,
) -> anyhow::Result<BoxStream<'static, Option<Backend>>> {
    let network_manager = bus::owner_watch(conn, NETWORK_MANAGER)
        .await?
        .map(|owned| (Backend::NetworkManager, owned));
    let iwd = bus::owner_watch(conn, IWD)
        .await?
        .map(|owned| (Backend::Iwd, owned));

    Ok(stream::select(network_manager, iwd)
        .scan((None, None), |owners, (backend, owned)| {
            match backend {
                Backend::NetworkManager => owners.0 = Some(owned),
                Backend::Iwd => owners.1 = Some(owned),
            }
            // Nothing to choose until both owners are known
            let known = owners.0.zip(owners.1);
            ready(Some(known.map(|(nm, iwd)| choose_backend(nm, iwd))))
        })
        .filter_map(ready)
        .boxed())
}

fn choose_backend(network_manager: bool, iwd: bool) -> Option<Backend> {
    if network_manager {
        Some(Backend::NetworkManager)
    } else if iwd {
        Some(Backend::Iwd)
    } else {
        None
    }
}

/// The best access point of each SSID, sorted.
fn best_per_ssid(access_points: Vec<AccessPointData>) -> Vec<AccessPointData> {
    let mut best = HashMap::<String, AccessPointData>::new();
    for access_point in access_points {
        match best.get(&access_point.ssid) {
            Some(kept) if !access_point.is_better(kept) => {}
            _ => {
                best.insert(access_point.ssid.clone(), access_point);
            }
        }
    }

    let mut access_points: Vec<_> = best.into_values().collect();
    sort_access_points(&mut access_points);
    access_points
}

/// Strongest first, then by SSID so equal strengths keep their order.
fn sort_access_points(access_points: &mut [AccessPointData]) {
    access_points.sort_by(|a, b| {
        b.strength
            .cmp(&a.strength)
            .then_with(|| a.ssid.cmp(&b.ssid))
    });
}

/// The access points whose SSID has a saved profile, in the same order.
fn known_access_points(
    access_points: &[AccessPointData],
    known_ssids: &HashSet<&str>,
) -> impl Iterator<Item = KnownConnection> {
    access_points
        .iter()
        .filter(|ap| known_ssids.contains(ap.ssid.as_str()))
        .cloned()
        .map(KnownConnection::AccessPoint)
}

type Properties = HashMap<String, OwnedVariant>;
type ManagedObjects = HashMap<OwnedObjectPath, HashMap<String, Properties>>;

#[proxy(interface = "org.freedesktop.DBus.ObjectManager")]
trait Objects {
    #[zbus(no_autostart)]
    fn get_managed_objects(&self) -> zbus::Result<ManagedObjects>;
}

/// Every object `service` has under `path` with all its properties, in one
/// call: a consistent snapshot, and no object can vanish halfway.
async fn managed_objects(
    conn: &zbus::Connection,
    service: &'static str,
    path: &'static str,
) -> zbus::Result<ManagedObjects> {
    ObjectsProxy::builder(conn)
        .destination(service)
        .path(path)
        .cache_properties(CacheProperties::No)
        .build()
        .await?
        .get_managed_objects()
        .await
}

/// The properties of `interface` on the object at `path`.
fn interface<'a>(
    objects: &'a ManagedObjects,
    path: &OwnedObjectPath,
    interface: &str,
) -> Option<&'a Properties> {
    objects.get(path)?.get(interface)
}

fn property<'a, T>(properties: &'a Properties, name: &str) -> Option<T>
where
    T: TryFrom<&'a Variant<'a>>,
    <T as TryFrom<&'a Variant<'a>>>::Error: Into<zbus::Error>,
{
    properties.get(name)?.downcast_ref().ok()
}

fn object_property(properties: &Properties, name: &str) -> Option<OwnedObjectPath> {
    property::<ObjectPath>(properties, name).map(OwnedObjectPath::from)
}

/// An array property, `None` when missing or not an array of `T`.
fn array<T>(properties: &Properties, name: &str) -> Option<Vec<T>>
where
    Vec<T>: TryFrom<OwnedVariant>,
{
    let value = properties.get(name)?.try_clone().ok()?;
    Vec::<T>::try_from(value).ok()
}

/// An array property, empty when missing.
fn list<T>(properties: &Properties, name: &str) -> Vec<T>
where
    Vec<T>: TryFrom<OwnedVariant>,
{
    array(properties, name).unwrap_or_default()
}

fn object_path(path: &'static str) -> OwnedObjectPath {
    OwnedObjectPath::from(ObjectPath::from_static_str_unchecked(path))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn access_point(ssid: &str, strength: u8, path: &str) -> AccessPointData {
        AccessPointData {
            ssid: ssid.to_string(),
            strength,
            max_bitrate: 1000,
            frequency: 5180,
            public: false,
            working: false,
            path: OwnedObjectPath::try_from(path).unwrap(),
            device_path: OwnedObjectPath::try_from("/org/freedesktop/NetworkManager/Devices/1")
                .unwrap(),
        }
    }

    fn home_net() -> AccessPointData {
        access_point(
            "HomeNet",
            80,
            "/org/freedesktop/NetworkManager/AccessPoint/1",
        )
    }

    fn data(access_points: Vec<AccessPointData>) -> NetworkData {
        NetworkData {
            backend: Backend::NetworkManager,
            wifi_present: true,
            wireless_access_points: access_points,
            active_connections: Vec::new(),
            known_connections: Vec::new(),
            wifi_enabled: true,
            connectivity: ConnectivityState::Full,
            scanning_nearby_wifi: false,
        }
    }

    #[test]
    fn command_debug_redacts_password() {
        let command = NetworkCommand::SelectAccessPoint {
            access_point: home_net(),
            password: Some(Redacted::new("correct horse battery staple".to_string())),
            connect_once: false,
        };

        let debug = format!("{command:?}");

        assert!(debug.contains("HomeNet"));
        assert!(debug.contains("Some(<redacted>)"));
        assert!(!debug.contains("correct horse battery staple"));
    }

    #[test]
    fn command_debug_without_password_keeps_normal_shape() {
        let command = NetworkCommand::SelectAccessPoint {
            access_point: home_net(),
            password: None,
            connect_once: true,
        };

        let debug = format!("{command:?}");

        assert!(debug.contains("SelectAccessPoint"));
        assert!(debug.contains("password: None"));
        assert!(debug.contains("connect_once: true"));
        assert!(!debug.contains("<redacted>"));
    }

    #[test]
    fn psk_outcome_debug_redacts_psk() {
        let outcome = PskOutcome::Password {
            psk: Redacted::new("correct horse battery staple".to_string()),
            security: WifiSecurity::Wpa,
            hidden: false,
        };

        let debug = format!("{outcome:?}");

        assert!(debug.contains("Password"));
        assert!(debug.contains("<redacted>"));
        assert!(!debug.contains("correct horse battery staple"));
    }

    #[test]
    fn psk_outcome_debug_without_password_keeps_normal_shape() {
        let outcome = PskOutcome::Open { hidden: true };

        let debug = format!("{outcome:?}");

        assert!(debug.contains("Open"));
        assert!(debug.contains("hidden: true"));
        assert!(!debug.contains("<redacted>"));
    }

    #[test]
    fn wifi_uri_escapes_and_marks_hidden_networks() {
        let password = PskOutcome::Password {
            psk: Redacted::new("p;a:ss".to_string()),
            security: WifiSecurity::Sae,
            hidden: true,
        };
        assert_eq!(
            password.wifi_uri("My\\Net").as_deref(),
            Some("WIFI:T:SAE;S:My\\\\Net;P:p\\;a\\:ss;H:true;;")
        );

        assert_eq!(
            PskOutcome::Open { hidden: false }
                .wifi_uri("Cafe")
                .as_deref(),
            Some("WIFI:T:nopass;S:Cafe;;")
        );
        assert_eq!(PskOutcome::Unsupported.wifi_uri("Corp"), None);
    }

    #[test]
    fn network_manager_wins_over_iwd() {
        assert_eq!(choose_backend(true, true), Some(Backend::NetworkManager));
        assert_eq!(choose_backend(true, false), Some(Backend::NetworkManager));
        assert_eq!(choose_backend(false, true), Some(Backend::Iwd));
        assert_eq!(choose_backend(false, false), None);
    }

    #[test]
    fn best_connectivity_wins() {
        let states = [ConnectivityState::Portal, ConnectivityState::Full];
        assert_eq!(states.into_iter().max(), Some(ConnectivityState::Full));
        assert!(ConnectivityState::Unknown < ConnectivityState::None);
    }

    #[test]
    fn strength_updates_the_access_point_and_the_active_wifi_only() {
        let mut data = data(vec![home_net(), access_point("Other", 60, "/ap/2")]);
        data.active_connections = vec![
            ActiveConnectionInfo::Vpn {
                name: "HomeNet".to_string(),
                object_path: OwnedObjectPath::try_from("/vpn").unwrap(),
            },
            ActiveConnectionInfo::WiFi {
                name: "HomeNet".to_string(),
                strength: 80,
                frequency: 5180,
            },
        ];
        data.known_connections = vec![KnownConnection::AccessPoint(home_net())];

        let patch = Patch::Strength(AccessPointOf::Path(home_net().path), 40);
        assert!(patch.apply(&mut data));

        let ssids: Vec<_> = data
            .wireless_access_points
            .iter()
            .map(|ap| (ap.ssid.as_str(), ap.strength))
            .collect();
        assert_eq!(ssids, [("Other", 60), ("HomeNet", 40)]);
        assert!(matches!(
            data.active_connections[1],
            ActiveConnectionInfo::WiFi { strength: 40, .. }
        ));
        assert!(matches!(
            &data.known_connections[0],
            KnownConnection::AccessPoint(ap) if ap.strength == 40
        ));

        assert!(!patch.apply(&mut data));
    }

    #[test]
    fn station_strength_updates_the_access_point_in_use() {
        let mut in_use = home_net();
        in_use.working = true;
        let mut data = data(vec![in_use]);

        let station = home_net().device_path;
        assert!(Patch::Strength(AccessPointOf::Station(station), 17).apply(&mut data));
        assert_eq!(data.wireless_access_points[0].strength, 17);
    }

    #[test]
    fn strength_of_an_unlisted_access_point_changes_nothing() {
        let mut data = data(vec![home_net()]);
        let before = data.clone();

        let changed = Patch::Strength(
            AccessPointOf::Path(OwnedObjectPath::try_from("/ap/9").unwrap()),
            10,
        )
        .apply(&mut data);

        assert!(!changed);
        assert_eq!(data, before);
    }

    #[test]
    fn best_per_ssid_keeps_the_best_and_sorts_by_strength_then_ssid() {
        let mut slow = access_point("c", 90, "/ap/4");
        slow.max_bitrate = 10;
        let access_points = best_per_ssid(vec![
            access_point("b", 50, "/ap/1"),
            slow,
            access_point("c", 70, "/ap/2"),
            access_point("a", 50, "/ap/3"),
        ]);

        let kept: Vec<_> = access_points
            .iter()
            .map(|ap| (ap.ssid.as_str(), ap.path.as_str()))
            .collect();
        assert_eq!(kept, [("c", "/ap/2"), ("a", "/ap/3"), ("b", "/ap/1")]);
    }

    #[test]
    fn best_per_ssid_keeps_the_access_point_in_use() {
        let mut in_use = access_point("a", 10, "/ap/2");
        in_use.working = true;

        let access_points = best_per_ssid(vec![access_point("a", 90, "/ap/1"), in_use]);

        assert_eq!(access_points[0].path.as_str(), "/ap/2");
    }
}
