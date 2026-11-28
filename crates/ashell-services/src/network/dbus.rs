use super::{
    AccessPointData, AccessPointOf, ActiveConnectionInfo, Backend, ConnectivityState, Event,
    KnownConnection, ManagedObjects, NETWORK_MANAGER, NetworkBackend, NetworkData, Patch,
    PendingScans, Properties, PskOutcome, Redacted, Vpn, WifiSecurity, array, best_per_ssid,
    interface, known_access_points, list, managed_objects, object_path, object_property, property,
};
use crate::bus::{self, changes_any, interface_properties_changed};
use futures::{
    StreamExt,
    future::{join_all, ready, try_join_all},
    stream::{self, BoxStream},
    stream_select,
};
use log::{debug, warn};
use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex, MutexGuard, PoisonError},
};
use zbus::{
    MatchRule, MessageStream, ObjectPath, OwnedObjectPath, OwnedVariant, Result, Variant,
    fdo::PropertiesChanged, message::Type, proxy, proxy::CacheProperties,
};

const MANAGER: &str = "org.freedesktop.NetworkManager";
const SETTINGS: &str = "org.freedesktop.NetworkManager.Settings";
const DEVICE: &str = "org.freedesktop.NetworkManager.Device";
const WIRELESS_DEVICE: &str = "org.freedesktop.NetworkManager.Device.Wireless";
const ACCESS_POINT: &str = "org.freedesktop.NetworkManager.AccessPoint";
const ACTIVE_CONNECTION: &str = "org.freedesktop.NetworkManager.Connection.Active";
const CONNECTION: &str = "org.freedesktop.NetworkManager.Settings.Connection";

const MANAGER_PATH: &str = "/org/freedesktop/NetworkManager";
const SETTINGS_PATH: &str = "/org/freedesktop/NetworkManager/Settings";

pub struct NetworkDbus<'a> {
    nm: NetworkManagerProxy<'a>,
    pending_scans: PendingScans,
    profiles: Profiles,
}

/// The saved profiles the reads have seen, by path, shared by every
/// `NetworkDbus` of a handle. Only new or updated profiles are read again.
/// Joining a network doesn't trust it: it reads the profiles afresh.
#[derive(Debug, Clone, Default)]
pub struct Profiles(Arc<Mutex<ProfileCache>>);

impl Profiles {
    fn lock(&self) -> MutexGuard<'_, ProfileCache> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

#[derive(Debug, Default)]
struct ProfileCache {
    /// Bumped by every invalidation: settings fetched before one may predate
    /// the change, so they aren't stored.
    generation: u64,
    entries: HashMap<OwnedObjectPath, Entry>,
    /// Profiles whose read failed and was already warned about.
    warned: HashSet<OwnedObjectPath>,
}

#[derive(Debug)]
enum Entry {
    Read(Profile),
    /// Not read again until it is updated, e.g. a profile other users own.
    Failed,
}

impl ProfileCache {
    fn invalidate(&mut self, path: &OwnedObjectPath) {
        self.generation += 1;
        self.entries.remove(path);
    }

    fn clear(&mut self) {
        self.generation += 1;
        self.entries.clear();
        self.warned.clear();
    }

    /// The profiles of `paths` to fetch, and the generation the fetch starts at.
    fn unread(&self, paths: &[OwnedObjectPath]) -> (Vec<OwnedObjectPath>, u64) {
        let unread = paths
            .iter()
            .filter(|path| !self.entries.contains_key(*path))
            .cloned()
            .collect();
        (unread, self.generation)
    }

    /// Stores what a fetch started at `started` read for `path`, unless
    /// something was invalidated since.
    fn store(
        &mut self,
        started: u64,
        path: OwnedObjectPath,
        settings: std::result::Result<ConnectionSettingsMap, zbus::Error>,
    ) {
        if self.generation != started {
            return;
        }

        match settings {
            Ok(settings) => {
                self.warned.remove(&path);
                self.entries
                    .insert(path, Entry::Read(Profile::new(&settings)));
            }
            Err(err) => {
                if self.warned.insert(path.clone()) {
                    warn!("Failed to get settings for connection {path}: {err}");
                } else {
                    debug!("Failed to get settings for connection {path}: {err}");
                }
                self.entries.insert(path, Entry::Failed);
            }
        }
    }

    /// Forgets the profiles that are gone.
    fn retain(&mut self, paths: &[OwnedObjectPath]) {
        self.entries.retain(|path, _| paths.contains(path));
        self.warned.retain(|path| paths.contains(path));
    }

    fn profiles(&self) -> impl Iterator<Item = (&OwnedObjectPath, &Profile)> {
        self.entries.iter().filter_map(|(path, entry)| match entry {
            Entry::Read(profile) => Some((path, profile)),
            Entry::Failed => None,
        })
    }
}

/// What a saved profile is, as far as `NetworkData` cares.
#[derive(Debug, Clone)]
enum Profile {
    /// Joins the Wi-Fi network with this SSID as a client.
    Wifi(String),
    Vpn(String),
    Other,
}

impl Profile {
    fn new(settings: &ConnectionSettingsMap) -> Self {
        if settings.contains_key("802-11-wireless") {
            client_connection_ssid(settings).map_or(Profile::Other, Profile::Wifi)
        } else if settings.contains_key("vpn") || settings.contains_key("wireguard") {
            settings
                .get("connection")
                .and_then(|connection| property::<&str>(connection, "id"))
                .map_or(Profile::Other, |id| Profile::Vpn(id.to_owned()))
        } else {
            Profile::Other
        }
    }
}

impl NetworkBackend for NetworkDbus<'_> {
    async fn read(&self) -> anyhow::Result<NetworkData> {
        let objects = self.objects().await?;
        let manager = manager(&objects)?;

        let wireless_devices = wireless_devices(&objects, manager);

        let wifi_enabled = property(manager, "WirelessEnabled").unwrap_or_default();
        debug!("Wifi enabled: {wifi_enabled}");

        let active_connections = active_connections(&objects, manager);
        debug!("Active connections: {active_connections:?}");

        let wireless_access_points = access_points(&objects, &wireless_devices);
        debug!("Wireless access points: {wireless_access_points:?}");

        let known_connections = self
            .known_connections(&objects, &wireless_access_points)
            .await?;
        debug!("Known connections: {known_connections:?}");

        Ok(NetworkData {
            backend: Backend::NetworkManager,
            wifi_present: !wireless_devices.is_empty(),
            active_connections,
            wifi_enabled,
            connectivity: connectivity(property(manager, "Connectivity").unwrap_or_default()),
            wireless_access_points,
            known_connections,
            scanning_nearby_wifi: self.scanning(&wireless_devices),
        })
    }

    async fn events(&self) -> anyhow::Result<BoxStream<'static, Event>> {
        let conn = self.nm.inner().connection();

        let signals = interface_properties_changed(
            conn,
            NETWORK_MANAGER,
            MANAGER_PATH,
            &[MANAGER, SETTINGS, WIRELESS_DEVICE, ACCESS_POINT],
        )
        .await?;
        let pending_scans = self.pending_scans.clone();
        let properties = signals
            .filter_map(move |signal| ready(event(&signal, &pending_scans)))
            .boxed();

        // A profile edited in place keeps its path: read it again
        let rule = MatchRule::builder()
            .msg_type(Type::Signal)
            .sender(NETWORK_MANAGER)
            .interface(CONNECTION)
            .member("Updated")
            .path_namespace(SETTINGS_PATH)
            .build()?;
        let profiles = self.profiles.clone();
        let updated = MessageStream::for_match_rule(rule, conn, None)
            .await?
            .filter_map(move |message| {
                let path = message
                    .ok()
                    .and_then(|message| message.header().path().map(|path| path.to_owned()));
                if let Some(path) = path {
                    profiles.lock().invalidate(&OwnedObjectPath::from(path));
                }
                ready(Some(Event::Changed))
            })
            .boxed();

        // Paths are only valid for one NetworkManager instance, and another one
        // can take over the name without it ever going unowned
        let profiles = self.profiles.clone();
        let owner = bus::owner_watch(conn, NETWORK_MANAGER)
            .await?
            .skip(1)
            .map(move |_| {
                profiles.lock().clear();
                Event::Restart
            })
            .boxed();
        self.profiles.lock().clear();

        let scanning = stream::unfold(self.pending_scans.subscribe(), |mut pending| async move {
            pending.changed().await.ok()?;
            let scanning = !pending.borrow_and_update().is_empty();
            Some((Event::Patch(Patch::Scanning(scanning)), pending))
        })
        .boxed();

        Ok(stream_select!(properties, updated, owner, scanning).boxed())
    }

    async fn toggle_wifi(&self) -> anyhow::Result<()> {
        let enabled = self.nm.wireless_enabled().await?;
        self.nm.set_wireless_enabled(!enabled).await?;
        Ok(())
    }

    async fn scan(&self) -> anyhow::Result<()> {
        let objects = self.objects().await?;
        let device_paths: Vec<_> = wireless_devices(&objects, manager(&objects)?)
            .into_iter()
            .map(|device| device.path.clone())
            .collect();

        let requested = device_paths.iter().cloned().collect();
        // A scan already pending on the same devices changes nothing shown
        self.pending_scans.send_if_modified(|pending| {
            if *pending == requested {
                return false;
            }
            *pending = requested;
            true
        });

        for device_path in device_paths {
            let request = async {
                WirelessDeviceProxy::builder(self.nm.inner().connection())
                    .path(device_path.clone())
                    .cache_properties(CacheProperties::No)
                    .build()
                    .await?
                    .request_scan(HashMap::new())
                    .await
            };

            if let Err(e) = request.await {
                warn!("Scan request failed for device {device_path}: {e}");
                self.pending_scans
                    .send_if_modified(|pending| pending.remove(&device_path));
            }
        }

        Ok(())
    }

    async fn select_access_point(
        &self,
        access_point: &AccessPointData,
        password: Option<Redacted<String>>,
        connect_once: bool,
    ) -> anyhow::Result<()> {
        let conn = self.nm.inner().connection();
        let objects = self.objects().await?;
        let security = ap_security(&objects, access_point);

        if let Some(connection_path) = self.find_connection(&objects, &access_point.ssid).await? {
            let connection = connection_settings(conn, &connection_path).await?;

            let mut s = connection.get_settings().await?;
            let mut changed = false;
            let password = password
                .as_ref()
                .map(|password| password.as_inner().as_str())
                .filter(|_| security != ApSecurity::Open);

            if let Some(password) = password {
                if !security.supports_psk() {
                    anyhow::bail!(
                        "cannot store a passphrase for '{}': unsupported Wi-Fi security ({security:?})",
                        access_point.ssid
                    );
                }

                let sec = s.entry("802-11-wireless-security".to_string()).or_default();
                write_psk(sec, password)?;
                if !sec.contains_key("key-mgmt") {
                    sec.insert(
                        "key-mgmt".to_string(),
                        OwnedVariant::try_from(Variant::from(new_section_key_mgmt(security)))?,
                    );
                }
                changed = true;
            }
            if let Some(sec) = s.get_mut("802-11-wireless-security") {
                changed |= downgrade_sae_to_wpa_psk(sec, security)?;
            }

            let mut specific_object = object_path("/");
            if changed {
                let matches_pin = if let Some(pinned) = wireless_bssid(&s)? {
                    let selected = ap_bssid(&objects, access_point);
                    let matches = bssid_matches(&pinned, selected);
                    if !matches && password.is_some() {
                        anyhow::bail!(
                            "saved profile '{}' is pinned to BSSID {pinned}, but the selected access point is {}; refusing to change its security",
                            access_point.ssid,
                            selected.unwrap_or("unknown")
                        );
                    }
                    matches
                } else {
                    true
                };
                // If the pin doesn't match, let NM try the original profile without a rewrite.
                if matches_pin {
                    connection.update(s).await?;
                    specific_object = access_point.path.clone();
                }
            }

            self.nm
                .activate_connection(
                    connection_path,
                    access_point.device_path.clone(),
                    specific_object,
                )
                .await?;
        } else {
            debug!("Create new wifi connection: {}", access_point.ssid);

            let conn_settings = wifi_connection_settings(
                &access_point.ssid,
                password
                    .as_ref()
                    .map(|password| password.as_inner().as_str()),
                security,
                connect_once,
            )?;

            // "volatile" keeps the profile in memory only and NM deletes it on disconnect
            let persist = if connect_once { "volatile" } else { "disk" };
            debug!("Activating '{}' with persist={persist}", access_point.ssid);

            self.nm
                .add_and_activate_connection2(
                    conn_settings,
                    &access_point.device_path,
                    &access_point.path,
                    HashMap::from([("persist", Variant::Str(persist.into()))]),
                )
                .await?;
        }

        Ok(())
    }

    async fn toggle_vpn(&self, vpn: &Vpn) -> anyhow::Result<()> {
        let objects = self.objects().await?;
        let active = active_connections(&objects, manager(&objects)?)
            .into_iter()
            .find_map(|connection| match connection {
                ActiveConnectionInfo::Vpn { name, object_path } if name == vpn.name => {
                    Some(object_path)
                }
                _ => None,
            });

        if let Some(connection) = active {
            debug!("Deactivating VPN: {connection:?}");
            self.nm.deactivate_connection(connection).await?;
        } else {
            debug!("Activating VPN: {:?}", vpn.path);
            self.nm
                .activate_connection(vpn.path.clone(), object_path("/"), object_path("/"))
                .await?;
        }

        Ok(())
    }
}

/// The event a `PropertiesChanged` of NetworkManager stands for, if it touches
/// something `NetworkData` shows. A finished scan leaves the pending ones.
fn event(signal: &PropertiesChanged, pending_scans: &PendingScans) -> Option<Event> {
    let args = signal.args().ok()?;
    let changed = |shown: &[&str]| changes_any(&args, shown);

    match args.interface_name().as_str() {
        MANAGER
            if changed(&[
                "WirelessEnabled",
                "Connectivity",
                "ActiveConnections",
                "Devices",
            ]) =>
        {
            Some(Event::Changed)
        }
        SETTINGS if changed(&["Connections"]) => Some(Event::Changed),
        WIRELESS_DEVICE => {
            if changed(&["LastScan"])
                && let Some(path) = signal.message().header().path()
            {
                let path = OwnedObjectPath::from(path.to_owned());
                pending_scans.send_if_modified(|pending| pending.remove(&path));
            }

            changed(&["AccessPoints", "ActiveAccessPoint"]).then_some(Event::Changed)
        }
        ACCESS_POINT => {
            let strength = args
                .changed_properties()
                .get("Strength")?
                .downcast_ref::<u8>()
                .ok()?;
            let path = OwnedObjectPath::from(signal.message().header().path()?.to_owned());

            Some(Event::Patch(Patch::Strength(
                AccessPointOf::Path(path),
                strength,
            )))
        }
        _ => None,
    }
}

impl NetworkDbus<'static> {
    pub async fn new(
        conn: &zbus::Connection,
        pending_scans: PendingScans,
        profiles: Profiles,
    ) -> anyhow::Result<Self> {
        // Uncached: a property read after a change signal must not see the old value
        let nm = NetworkManagerProxy::builder(conn)
            .cache_properties(CacheProperties::No)
            .build()
            .await?;

        Ok(Self {
            nm,
            pending_scans,
            profiles,
        })
    }
}

impl NetworkDbus<'_> {
    async fn objects(&self) -> zbus::Result<ManagedObjects> {
        managed_objects(
            self.nm.inner().connection(),
            NETWORK_MANAGER,
            "/org/freedesktop",
        )
        .await
    }

    /// Whether a scan we asked for is running. Devices gone are no longer pending.
    fn scanning(&self, wireless_devices: &[WirelessDevice]) -> bool {
        self.pending_scans.send_if_modified(|pending| {
            let before = pending.len();
            pending.retain(|path| wireless_devices.iter().any(|device| device.path == path));
            pending.len() != before
        });

        !self.pending_scans.borrow().is_empty()
    }

    /// The first saved client profile for `ssid`, in NetworkManager's order,
    /// read afresh: a join must not create a duplicate of a profile the cache
    /// missed, nor go on when one can't be read.
    async fn find_connection(
        &self,
        objects: &ManagedObjects,
        ssid: &str,
    ) -> anyhow::Result<Option<OwnedObjectPath>> {
        let conn = self.nm.inner().connection();
        let paths = saved_connections(objects)?;
        let settings = try_join_all(paths.iter().map(|path| async move {
            connection_settings(conn, path).await?.get_settings().await
        }))
        .await?;

        Ok(paths
            .into_iter()
            .zip(settings)
            .find_map(|(path, settings)| {
                (client_connection_ssid(&settings).as_deref() == Some(ssid)).then_some(path)
            }))
    }

    /// Reads the saved profiles not read yet, concurrently, and forgets the
    /// ones gone.
    async fn read_profiles(&self, objects: &ManagedObjects) -> anyhow::Result<()> {
        let paths = saved_connections(objects)?;

        let (unread, started) = self.profiles.lock().unread(&paths);
        let read = join_all(unread.into_iter().map(|path| async move {
            let settings = async {
                connection_settings(self.nm.inner().connection(), &path)
                    .await?
                    .get_settings()
                    .await
            };
            (settings.await, path)
        }))
        .await;

        let mut profiles = self.profiles.lock();
        for (settings, path) in read {
            profiles.store(started, path, settings);
        }
        profiles.retain(&paths);

        Ok(())
    }

    /// The visible access points with a saved client profile, in the same
    /// order, then the VPNs by name.
    async fn known_connections(
        &self,
        objects: &ManagedObjects,
        wireless_access_points: &[AccessPointData],
    ) -> anyhow::Result<Vec<KnownConnection>> {
        self.read_profiles(objects).await?;

        let profiles = self.profiles.lock();
        let mut known_ssids = HashSet::new();
        let mut known_vpn = Vec::new();
        for (path, profile) in profiles.profiles() {
            match profile {
                Profile::Wifi(ssid) => {
                    known_ssids.insert(ssid.as_str());
                }
                Profile::Vpn(name) => known_vpn.push(Vpn {
                    name: name.clone(),
                    path: path.clone(),
                }),
                Profile::Other => {}
            }
        }
        known_vpn.sort_by(|a, b| a.name.cmp(&b.name).then_with(|| a.path.cmp(&b.path)));

        Ok(known_access_points(wireless_access_points, &known_ssids)
            .chain(known_vpn.into_iter().map(KnownConnection::Vpn))
            .collect())
    }

    /// Reads the stored passphrase for `ssid` from the profile of the connection
    /// currently carrying it. NetworkManager-only, so not part of
    /// [`super::NetworkBackend`].
    pub async fn read_psk(&self, ssid: &str) -> anyhow::Result<PskOutcome> {
        let Some((connection_path, all_settings)) = self.active_wifi_profile(ssid).await? else {
            // Only offered for the active connection: no profile means we can't
            // tell whether the network is open.
            debug!("read_psk: no active connection profile for SSID '{ssid}'");
            return Ok(PskOutcome::PasswordUnavailable);
        };

        let hidden = all_settings
            .get("802-11-wireless")
            .and_then(|wifi| property(wifi, "hidden"))
            .unwrap_or(false);

        // Open networks have no security section, and GetSecrets errors on a missing one.
        let Some(security_settings) = all_settings.get("802-11-wireless-security") else {
            return Ok(PskOutcome::Open { hidden });
        };

        // 802.1X, OWE and static WEP have no PSK a `WIFI:` URI can carry.
        let security = match property::<&str>(security_settings, "key-mgmt") {
            Some("wpa-psk") | Some("wpa-psk-sha256") => WifiSecurity::Wpa,
            Some("sae") => WifiSecurity::Sae,
            _ => return Ok(PskOutcome::Unsupported),
        };

        let secrets = connection_settings(self.nm.inner().connection(), &connection_path)
            .await?
            .get_secrets("802-11-wireless-security")
            .await
            .map_err(|e| anyhow::anyhow!("polkit/secret retrieval denied for '{ssid}': {e}"))?;

        let psk = secrets
            .get("802-11-wireless-security")
            .and_then(|security| property::<&str>(security, "psk"));

        Ok(match psk {
            Some(psk) if !psk.is_empty() => PskOutcome::Password {
                psk: Redacted::new(psk.to_owned()),
                security,
                hidden,
            },
            // Secured, but the secret is agent-owned or flagged not-saved.
            _ => PskOutcome::PasswordUnavailable,
        })
    }

    /// Settings profile behind the active connection carrying `ssid`.
    ///
    /// Starts from the active connection's own settings profile (its
    /// `Connection` property) instead of scanning saved profiles, so a stale
    /// profile that shares the SSID can't provide the password.
    async fn active_wifi_profile(
        &self,
        ssid: &str,
    ) -> anyhow::Result<Option<(OwnedObjectPath, ConnectionSettingsMap)>> {
        let objects = self.objects().await?;

        for active_path in list::<OwnedObjectPath>(manager(&objects)?, "ActiveConnections") {
            let Some(active) = interface(&objects, &active_path, ACTIVE_CONNECTION) else {
                continue;
            };
            if property::<&str>(active, "Type") != Some("802-11-wireless") {
                continue;
            }
            let Some(profile_path) = object_property(active, "Connection") else {
                continue;
            };

            let profile = connection_settings(self.nm.inner().connection(), &profile_path).await?;
            let Ok(settings) = profile.get_settings().await else {
                warn!("active_wifi_profile: failed to get settings for {profile_path}");
                continue;
            };

            if connection_ssid(&settings).as_deref() == Some(ssid) {
                return Ok(Some((profile_path, settings)));
            }
        }

        Ok(None)
    }
}

/// The saved profiles, in NetworkManager's order. Their absence is an error:
/// an empty list would read as no saved profile at all.
fn saved_connections(objects: &ManagedObjects) -> anyhow::Result<Vec<OwnedObjectPath>> {
    interface(objects, &object_path(SETTINGS_PATH), SETTINGS)
        .and_then(|settings| array(settings, "Connections"))
        .ok_or_else(|| anyhow::anyhow!("NetworkManager exposes no saved connections"))
}

fn manager(objects: &ManagedObjects) -> anyhow::Result<&Properties> {
    interface(objects, &object_path(MANAGER_PATH), MANAGER)
        .ok_or_else(|| anyhow::anyhow!("NetworkManager exposes no {MANAGER_PATH}"))
}

fn connectivity(state: u32) -> ConnectivityState {
    match state {
        1 => ConnectivityState::None,
        2 => ConnectivityState::Portal,
        3 => ConnectivityState::Loss,
        4 => ConnectivityState::Full,
        _ => ConnectivityState::Unknown,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DeviceType {
    Ethernet,
    Wifi,
    WireGuard,
    Other,
}

fn device_type(device: &Properties) -> DeviceType {
    match property::<u32>(device, "DeviceType") {
        Some(1) => DeviceType::Ethernet,
        Some(2) => DeviceType::Wifi,
        Some(29) => DeviceType::WireGuard,
        _ => DeviceType::Other,
    }
}

struct WirelessDevice<'a> {
    path: &'a OwnedObjectPath,
    wireless: &'a Properties,
}

fn wireless_devices<'a>(
    objects: &'a ManagedObjects,
    manager: &Properties,
) -> Vec<WirelessDevice<'a>> {
    list::<OwnedObjectPath>(manager, "Devices")
        .into_iter()
        .filter_map(|path| {
            let (path, interfaces) = objects.get_key_value(&path)?;
            let device = interfaces.get(DEVICE)?;
            let wireless = interfaces.get(WIRELESS_DEVICE)?;

            (device_type(device) == DeviceType::Wifi).then_some(WirelessDevice { path, wireless })
        })
        .collect()
}

fn access_points(
    objects: &ManagedObjects,
    wireless_devices: &[WirelessDevice],
) -> Vec<AccessPointData> {
    let mut access_points = Vec::new();
    for device in wireless_devices {
        let active = object_property(device.wireless, "ActiveAccessPoint");

        for path in list::<OwnedObjectPath>(device.wireless, "AccessPoints") {
            let Some(ap) = interface(objects, &path, ACCESS_POINT) else {
                continue;
            };
            let ssid = String::from_utf8_lossy(&list::<u8>(ap, "Ssid")).into_owned();
            // Hidden networks have no SSID to show
            let (false, Some(strength)) = (ssid.is_empty(), property::<u8>(ap, "Strength")) else {
                continue;
            };

            access_points.push(AccessPointData {
                ssid,
                strength,
                max_bitrate: property(ap, "MaxBitrate").unwrap_or_default(),
                frequency: property(ap, "Frequency").unwrap_or_default(),
                public: property::<u32>(ap, "Flags").unwrap_or_default() == 0,
                working: active.as_ref() == Some(&path),
                path,
                device_path: device.path.clone(),
            });
        }
    }

    best_per_ssid(access_points)
}

fn active_connections(objects: &ManagedObjects, manager: &Properties) -> Vec<ActiveConnectionInfo> {
    let mut info = Vec::new();
    for path in list::<OwnedObjectPath>(manager, "ActiveConnections") {
        let Some(connection) = interface(objects, &path, ACTIVE_CONNECTION) else {
            continue;
        };
        let Some(name) = property::<&str>(connection, "Id").map(str::to_owned) else {
            continue;
        };

        if property(connection, "Vpn").unwrap_or_default() {
            info.push(ActiveConnectionInfo::Vpn {
                name,
                object_path: path,
            });
            continue;
        }

        for device_path in list::<OwnedObjectPath>(connection, "Devices") {
            let Some(device) = interface(objects, &device_path, DEVICE) else {
                continue;
            };

            match device_type(device) {
                DeviceType::Ethernet => {
                    info.push(ActiveConnectionInfo::Wired { name: name.clone() })
                }
                DeviceType::Wifi => {
                    let access_point = interface(objects, &device_path, WIRELESS_DEVICE)
                        .and_then(|wireless| object_property(wireless, "ActiveAccessPoint"))
                        .and_then(|ap| interface(objects, &ap, ACCESS_POINT));

                    if let Some(access_point) = access_point {
                        info.push(ActiveConnectionInfo::WiFi {
                            name: String::from_utf8_lossy(&list::<u8>(access_point, "Ssid"))
                                .into_owned(),
                            strength: property(access_point, "Strength").unwrap_or_default(),
                            frequency: property(access_point, "Frequency").unwrap_or_default(),
                        });
                    }
                }
                DeviceType::WireGuard => info.push(ActiveConnectionInfo::Vpn {
                    name: name.clone(),
                    object_path: path.clone(),
                }),
                DeviceType::Other => {}
            }
        }
    }

    info.sort_by(|a, b| {
        fn key(conn: &ActiveConnectionInfo) -> (u8, &str) {
            match conn {
                ActiveConnectionInfo::Vpn { name, .. } => (0, name),
                ActiveConnectionInfo::Wired { name, .. } => (1, name),
                ActiveConnectionInfo::WiFi { name, .. } => (2, name),
            }
        }
        key(a).cmp(&key(b))
    });

    info
}

async fn connection_settings<'a>(
    conn: &zbus::Connection,
    path: &'a OwnedObjectPath,
) -> zbus::Result<ConnectionSettingsProxy<'a>> {
    ConnectionSettingsProxy::builder(conn)
        .path(path)
        .cache_properties(CacheProperties::No)
        .build()
        .await
}

type ConnectionSettingsMap = HashMap<String, Properties>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ApSecurity {
    Open,
    Wpa2,
    Wpa3,
    Mixed,
    Owe,
    Enterprise,
    Wep,
    Unknown,
}

impl ApSecurity {
    fn supports_psk(self) -> bool {
        matches!(self, Self::Open | Self::Wpa2 | Self::Wpa3 | Self::Mixed)
    }
}

const AP_SEC_KEY_MGMT_PSK: u32 = 0x0000_0100;
const AP_SEC_KEY_MGMT_802_1X: u32 = 0x0000_0200;
const AP_SEC_KEY_MGMT_SAE: u32 = 0x0000_0400;
const AP_SEC_KEY_MGMT_OWE: u32 = 0x0000_0800;
const AP_SEC_KEY_MGMT_OWE_TM: u32 = 0x0000_1000;
const AP_SEC_KEY_MGMT_EAP_SUITE_B_192: u32 = 0x0000_2000;
const AP_FLAG_PRIVACY: u32 = 0x0000_0001;

/// The security of the access point, from its flags as the read saw them.
fn ap_security(objects: &ManagedObjects, access_point: &AccessPointData) -> ApSecurity {
    let flags = interface(objects, &access_point.path, ACCESS_POINT).and_then(|ap| {
        Some((
            property::<u32>(ap, "Flags")?,
            property::<u32>(ap, "RsnFlags")?,
            property::<u32>(ap, "WpaFlags")?,
        ))
    });
    let Some((flags, rsn_flags, wpa_flags)) = flags else {
        debug!("Could not read security flags for '{}'", access_point.ssid);
        return ApSecurity::Unknown;
    };

    let security = ap_security_from_flags(flags, rsn_flags, wpa_flags);
    debug!(
        "AP '{}' security: flags={flags:#x} rsn={rsn_flags:#x} wpa={wpa_flags:#x} -> {security:?}",
        access_point.ssid
    );
    security
}

fn ap_bssid<'a>(objects: &'a ManagedObjects, access_point: &AccessPointData) -> Option<&'a str> {
    property(
        interface(objects, &access_point.path, ACCESS_POINT)?,
        "HwAddress",
    )
}

fn ap_security_from_flags(flags: u32, rsn_flags: u32, wpa_flags: u32) -> ApSecurity {
    let sec = rsn_flags | wpa_flags;

    if sec & (AP_SEC_KEY_MGMT_OWE | AP_SEC_KEY_MGMT_OWE_TM) != 0 {
        return ApSecurity::Owe;
    }
    if sec & (AP_SEC_KEY_MGMT_802_1X | AP_SEC_KEY_MGMT_EAP_SUITE_B_192) != 0 {
        return ApSecurity::Enterprise;
    }

    let psk = sec & AP_SEC_KEY_MGMT_PSK != 0;
    let sae = sec & AP_SEC_KEY_MGMT_SAE != 0;

    match (psk, sae) {
        (true, true) => ApSecurity::Mixed,
        (true, false) => ApSecurity::Wpa2,
        (false, true) => ApSecurity::Wpa3,
        (false, false) => match (sec == 0, flags & AP_FLAG_PRIVACY != 0) {
            (true, false) => ApSecurity::Open,
            // NM treats privacy with no WPA/RSN flags as WEP (static or dynamic).
            (true, true) => ApSecurity::Wep,
            (false, _) => ApSecurity::Unknown,
        },
    }
}

fn wifi_connection_settings(
    ssid: &str,
    password: Option<&str>,
    security: ApSecurity,
    connect_once: bool,
) -> anyhow::Result<ConnectionSettingsMap> {
    match (security, password.is_some()) {
        (ApSecurity::Open, _) | (ApSecurity::Owe, false) => {}
        (ApSecurity::Wpa2 | ApSecurity::Wpa3 | ApSecurity::Mixed, true) => {}
        (ApSecurity::Wpa2 | ApSecurity::Wpa3 | ApSecurity::Mixed, false) => {
            anyhow::bail!("a passphrase is required to create a profile for '{ssid}'");
        }
        _ => anyhow::bail!(
            "unsupported Wi-Fi security for '{ssid}': {security:?} networks need credentials ashell cannot provide"
        ),
    }

    let owned = |v: Variant<'_>| -> anyhow::Result<OwnedVariant> { Ok(v.try_to_owned()?) };

    let connection = HashMap::from([
        ("id".to_string(), owned(Variant::from(ssid))?),
        ("type".to_string(), owned(Variant::from("802-11-wireless"))?),
        (
            "autoconnect".to_string(),
            owned(Variant::from(!connect_once))?,
        ),
    ]);

    let wireless = HashMap::from([(
        "ssid".to_string(),
        owned(Variant::Array(ssid.as_bytes().into()))?,
    )]);

    let mut settings = HashMap::from([
        ("connection".to_string(), connection),
        ("802-11-wireless".to_string(), wireless),
    ]);

    if let Some(password) = password.filter(|_| security != ApSecurity::Open) {
        let mut security_section = HashMap::new();
        security_section.insert("psk".to_string(), owned(Variant::from(password))?);
        security_section.insert("psk-flags".to_string(), owned(Variant::from(0u32))?);
        settings.insert("802-11-wireless-security".to_string(), security_section);
    }

    Ok(settings)
}

/// `sae` cannot authenticate to a WPA2-only AP. The reverse is not done: NM
/// makes `wpa-psk` work with SAE-capable APs when PMF allows.
fn downgrade_sae_to_wpa_psk(sec: &mut Properties, security: ApSecurity) -> anyhow::Result<bool> {
    if security != ApSecurity::Wpa2 || property::<&str>(sec, "key-mgmt") != Some("sae") {
        return Ok(false);
    }

    sec.insert(
        "key-mgmt".to_string(),
        OwnedVariant::try_from(Variant::from("wpa-psk"))?,
    );
    Ok(true)
}

fn new_section_key_mgmt(security: ApSecurity) -> &'static str {
    if security == ApSecurity::Wpa3 {
        "sae"
    } else {
        "wpa-psk"
    }
}

fn write_psk(sec: &mut Properties, password: &str) -> anyhow::Result<()> {
    sec.insert(
        "psk".to_string(),
        OwnedVariant::try_from(Variant::from(password))?,
    );
    if !sec.contains_key("psk-flags") {
        sec.insert(
            "psk-flags".to_string(),
            OwnedVariant::try_from(Variant::from(0u32))?,
        );
    }
    Ok(())
}

fn bssid_matches(pinned: &str, selected: Option<&str>) -> bool {
    selected.is_some_and(|bssid| bssid.eq_ignore_ascii_case(pinned))
}

fn wireless_bssid(settings: &ConnectionSettingsMap) -> anyhow::Result<Option<String>> {
    let Some(wifi) = settings
        .get("802-11-wireless")
        .filter(|wifi| wifi.contains_key("bssid"))
    else {
        return Ok(None);
    };
    if let [a, b, c, d, e, f] = list::<u8>(wifi, "bssid")[..] {
        return Ok(Some(format!(
            "{a:02X}:{b:02X}:{c:02X}:{d:02X}:{e:02X}:{f:02X}"
        )));
    }
    anyhow::bail!("invalid BSSID in saved Wi-Fi profile")
}

fn connection_ssid(settings: &ConnectionSettingsMap) -> Option<String> {
    let ssid = array::<u8>(settings.get("802-11-wireless")?, "ssid")?;
    Some(String::from_utf8_lossy(&ssid).into_owned())
}

/// SSID of a profile that joins networks as a client. Hotspot (`ap`), ad-hoc
/// and mesh profiles share the SSID field but can't join a scanned network.
fn client_connection_ssid(settings: &ConnectionSettingsMap) -> Option<String> {
    let mode = settings
        .get("802-11-wireless")
        .and_then(|wifi| property::<&str>(wifi, "mode"));

    match mode {
        None | Some("infrastructure") => connection_ssid(settings),
        _ => None,
    }
}

#[proxy(
    interface = "org.freedesktop.NetworkManager",
    default_service = "org.freedesktop.NetworkManager",
    default_path = "/org/freedesktop/NetworkManager"
)]
pub trait NetworkManager {
    fn activate_connection(
        &self,
        connection: OwnedObjectPath,
        device: OwnedObjectPath,
        specific_object: OwnedObjectPath,
    ) -> Result<OwnedObjectPath>;

    /// Supersedes `AddAndActivateConnection` with an options dictionary
    /// (`persist`, `bind-activation`). Requires NetworkManager 1.16 or newer.
    #[zbus(name = "AddAndActivateConnection2")]
    fn add_and_activate_connection2(
        &self,
        connection: ConnectionSettingsMap,
        device: &ObjectPath<'_>,
        specific_object: &ObjectPath<'_>,
        options: HashMap<&str, Variant<'_>>,
    ) -> Result<(
        OwnedObjectPath,
        OwnedObjectPath,
        HashMap<String, OwnedVariant>,
    )>;

    fn deactivate_connection(&self, connection: OwnedObjectPath) -> Result<()>;

    #[zbus(property)]
    fn wireless_enabled(&self) -> Result<bool>;

    #[zbus(property)]
    fn set_wireless_enabled(&self, value: bool) -> Result<()>;
}

#[proxy(
    default_service = "org.freedesktop.NetworkManager",
    default_path = "/org/freedesktop/NetworkManager/Device/Wireless",
    interface = "org.freedesktop.NetworkManager.Device.Wireless"
)]
pub trait WirelessDevice {
    fn request_scan(&self, options: HashMap<String, OwnedVariant>) -> Result<()>;
}

#[proxy(
    default_service = "org.freedesktop.NetworkManager",
    default_path = "/org/freedesktop/NetworkManager/Settings/Connection",
    interface = "org.freedesktop.NetworkManager.Settings.Connection"
)]
trait ConnectionSettings {
    fn update(&self, settings: HashMap<String, HashMap<String, OwnedVariant>>) -> Result<()>;

    fn get_settings(&self) -> Result<HashMap<String, HashMap<String, OwnedVariant>>>;

    /// Gated by polkit rather than `*-flags`, so it returns secrets even when `psk-flags` is set.
    fn get_secrets(
        &self,
        setting_name: &str,
    ) -> Result<HashMap<String, HashMap<String, OwnedVariant>>>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ops::Deref;

    fn wifi_settings(mode: Option<&str>) -> ConnectionSettingsMap {
        let mut wifi = HashMap::from([(
            "ssid".to_string(),
            OwnedVariant::try_from(Variant::from(b"MyNet".to_vec())).unwrap(),
        )]);
        if let Some(mode) = mode {
            wifi.insert(
                "mode".to_string(),
                OwnedVariant::try_from(Variant::from(mode)).unwrap(),
            );
        }
        HashMap::from([("802-11-wireless".to_string(), wifi)])
    }

    #[test]
    fn client_profiles_match_by_ssid() {
        assert_eq!(
            client_connection_ssid(&wifi_settings(None)).as_deref(),
            Some("MyNet")
        );
        assert_eq!(
            client_connection_ssid(&wifi_settings(Some("infrastructure"))).as_deref(),
            Some("MyNet")
        );
    }

    #[test]
    fn non_client_profiles_are_skipped() {
        for mode in ["ap", "adhoc", "mesh"] {
            assert_eq!(client_connection_ssid(&wifi_settings(Some(mode))), None);
        }
    }

    #[test]
    fn an_ssid_that_is_not_a_byte_array_is_no_ssid() {
        let settings = HashMap::from([(
            "802-11-wireless".to_string(),
            HashMap::from([(
                "ssid".to_string(),
                OwnedVariant::try_from(Variant::from("MyNet")).unwrap(),
            )]),
        )]);

        assert_eq!(connection_ssid(&settings), None);
        assert_eq!(
            connection_ssid(&wifi_settings(None)).as_deref(),
            Some("MyNet")
        );
    }

    fn path(path: &str) -> OwnedObjectPath {
        OwnedObjectPath::try_from(path).unwrap()
    }

    fn settings_error() -> zbus::Error {
        zbus::Error::Failure("denied".to_string())
    }

    #[test]
    fn settings_fetched_before_an_invalidation_are_not_stored() {
        let mut cache = ProfileCache::default();
        let paths = [path("/s/1"), path("/s/2")];

        let (unread, started) = cache.unread(&paths);
        assert_eq!(unread.len(), 2);

        cache.invalidate(&paths[0]);
        cache.store(started, paths[0].clone(), Ok(wifi_settings(None)));
        cache.store(started, paths[1].clone(), Ok(wifi_settings(None)));
        assert_eq!(cache.profiles().count(), 0);

        let (unread, started) = cache.unread(&paths);
        assert_eq!(unread.len(), 2);
        cache.store(started, paths[0].clone(), Ok(wifi_settings(None)));
        assert_eq!(cache.profiles().count(), 1);
        assert_eq!(cache.unread(&paths).0, [paths[1].clone()]);
    }

    #[test]
    fn a_clear_drops_what_a_fetch_in_flight_read() {
        let mut cache = ProfileCache::default();
        let paths = [path("/s/1")];

        let (_, started) = cache.unread(&paths);
        cache.clear();
        cache.store(started, paths[0].clone(), Ok(wifi_settings(None)));

        assert_eq!(cache.profiles().count(), 0);
    }

    #[test]
    fn a_failed_profile_is_not_read_again_until_updated() {
        let mut cache = ProfileCache::default();
        let paths = [path("/s/1")];

        let (_, started) = cache.unread(&paths);
        cache.store(started, paths[0].clone(), Err(settings_error()));
        assert!(cache.unread(&paths).0.is_empty());
        assert_eq!(cache.profiles().count(), 0);
        assert!(cache.warned.contains(&paths[0]));

        cache.invalidate(&paths[0]);
        let (unread, started) = cache.unread(&paths);
        assert_eq!(unread, paths);
        cache.store(started, paths[0].clone(), Err(settings_error()));
        // Still warned about: the second failure only logs at debug level
        assert!(cache.warned.contains(&paths[0]));

        cache.retain(&[]);
        assert!(cache.warned.is_empty());
        assert!(cache.entries.is_empty());
    }

    #[test]
    fn vpn_profiles_are_named_by_their_id() {
        let settings = HashMap::from([
            ("vpn".to_string(), Properties::new()),
            (
                "connection".to_string(),
                HashMap::from([(
                    "id".to_string(),
                    OwnedVariant::try_from(Variant::from("Office")).unwrap(),
                )]),
            ),
        ]);

        assert!(matches!(Profile::new(&settings), Profile::Vpn(name) if name == "Office"));
    }

    fn field<'a>(
        s: &'a ConnectionSettingsMap,
        section: &str,
        key: &str,
    ) -> Option<&'a OwnedVariant> {
        s.get(section)?.get(key)
    }

    fn str_field(s: &ConnectionSettingsMap, section: &str, key: &str) -> Option<String> {
        match field(s, section, key)?.deref() {
            Variant::Str(v) => Some(v.to_string()),
            _ => None,
        }
    }

    #[test]
    fn security_from_flags_maps_key_mgmt() {
        assert_eq!(ap_security_from_flags(0, 0, 0), ApSecurity::Open);
        assert_eq!(
            ap_security_from_flags(AP_FLAG_PRIVACY, AP_SEC_KEY_MGMT_PSK, 0),
            ApSecurity::Wpa2
        );
        assert_eq!(
            ap_security_from_flags(AP_FLAG_PRIVACY, AP_SEC_KEY_MGMT_SAE, 0),
            ApSecurity::Wpa3
        );
        assert_eq!(
            ap_security_from_flags(
                AP_FLAG_PRIVACY,
                AP_SEC_KEY_MGMT_PSK | AP_SEC_KEY_MGMT_SAE,
                0
            ),
            ApSecurity::Mixed
        );
        assert_eq!(
            ap_security_from_flags(AP_FLAG_PRIVACY, AP_SEC_KEY_MGMT_OWE, 0),
            ApSecurity::Owe
        );
        assert_eq!(
            ap_security_from_flags(AP_FLAG_PRIVACY, AP_SEC_KEY_MGMT_OWE_TM, 0),
            ApSecurity::Owe
        );
        assert_eq!(
            ap_security_from_flags(AP_FLAG_PRIVACY, AP_SEC_KEY_MGMT_802_1X, 0),
            ApSecurity::Enterprise
        );
        assert_eq!(
            ap_security_from_flags(AP_FLAG_PRIVACY, AP_SEC_KEY_MGMT_EAP_SUITE_B_192, 0),
            ApSecurity::Enterprise
        );
        // Real WEP APs report only the privacy flag, with no WPA/RSN flags.
        assert_eq!(
            ap_security_from_flags(AP_FLAG_PRIVACY, 0, 0),
            ApSecurity::Wep
        );
        assert_eq!(
            ap_security_from_flags(AP_FLAG_PRIVACY, 0x8, 0),
            ApSecurity::Unknown
        );
        assert_eq!(ap_security_from_flags(0, 0x8, 0), ApSecurity::Unknown);
    }

    #[test]
    fn security_from_flags_uses_networkmanager_values() {
        // Real RsnFlags: WPA3-only 1160, mixed 1416, WPA2-only 392.
        assert_eq!(ap_security_from_flags(1, 1160, 0), ApSecurity::Wpa3);
        assert_eq!(ap_security_from_flags(3, 1416, 0), ApSecurity::Mixed);
        assert_eq!(ap_security_from_flags(1, 392, 0), ApSecurity::Wpa2);
    }

    #[test]
    fn open_profile_has_no_security_section() {
        let s = wifi_connection_settings("Cafe", None, ApSecurity::Open, false).unwrap();

        assert!(!s.contains_key("802-11-wireless-security"));
        assert!(field(&s, "802-11-wireless", "mode").is_none());
        assert_eq!(str_field(&s, "connection", "id").as_deref(), Some("Cafe"));
        assert_eq!(
            str_field(&s, "connection", "type").as_deref(),
            Some("802-11-wireless")
        );
        assert!(matches!(
            field(&s, "connection", "autoconnect").map(|v| v.deref()),
            Some(Variant::Bool(true))
        ));
    }

    #[test]
    fn wpa2_profile_sets_psk_without_key_mgmt() {
        let s = wifi_connection_settings("Home", Some("secret"), ApSecurity::Wpa2, false).unwrap();

        assert_eq!(
            str_field(&s, "802-11-wireless-security", "psk").as_deref(),
            Some("secret")
        );
        assert_eq!(str_field(&s, "802-11-wireless-security", "key-mgmt"), None);
        assert!(matches!(
            field(&s, "802-11-wireless-security", "psk-flags").map(|v| v.deref()),
            Some(Variant::U32(0))
        ));
    }

    #[test]
    fn wpa3_profile_sets_psk_without_key_mgmt() {
        let s = wifi_connection_settings("Home", Some("secret"), ApSecurity::Wpa3, false).unwrap();

        assert_eq!(
            str_field(&s, "802-11-wireless-security", "psk").as_deref(),
            Some("secret")
        );
        assert_eq!(str_field(&s, "802-11-wireless-security", "key-mgmt"), None);
    }

    #[test]
    fn mixed_profile_sets_psk_without_key_mgmt() {
        let s = wifi_connection_settings("Home", Some("secret"), ApSecurity::Mixed, false).unwrap();

        assert_eq!(
            str_field(&s, "802-11-wireless-security", "psk").as_deref(),
            Some("secret")
        );
        assert_eq!(str_field(&s, "802-11-wireless-security", "key-mgmt"), None);
    }

    #[test]
    fn connect_once_disables_autoconnect() {
        let s = wifi_connection_settings("Guest", None, ApSecurity::Open, true).unwrap();

        assert!(matches!(
            field(&s, "connection", "autoconnect").map(|v| v.deref()),
            Some(Variant::Bool(false))
        ));
    }

    #[test]
    fn unsupported_security_is_rejected() {
        for security in [
            ApSecurity::Owe,
            ApSecurity::Enterprise,
            ApSecurity::Wep,
            ApSecurity::Unknown,
        ] {
            assert!(wifi_connection_settings("X", Some("pw"), security, false).is_err());
        }
        for security in [ApSecurity::Enterprise, ApSecurity::Wep, ApSecurity::Unknown] {
            assert!(wifi_connection_settings("X", None, security, false).is_err());
        }
    }

    #[test]
    fn secured_profile_without_password_is_rejected() {
        for security in [ApSecurity::Wpa2, ApSecurity::Wpa3, ApSecurity::Mixed] {
            assert!(wifi_connection_settings("X", None, security, false).is_err());
        }
    }

    #[test]
    fn owe_profile_without_password_is_left_to_networkmanager() {
        let s = wifi_connection_settings("X", None, ApSecurity::Owe, false).unwrap();

        assert!(!s.contains_key("802-11-wireless-security"));
    }

    fn sec_with_key_mgmt(value: &str) -> HashMap<String, OwnedVariant> {
        HashMap::from([(
            "key-mgmt".to_string(),
            OwnedVariant::try_from(Variant::from(value)).unwrap(),
        )])
    }

    fn section_str(sec: &HashMap<String, OwnedVariant>, key: &str) -> Option<String> {
        match sec.get(key).map(|v| v.deref()) {
            Some(Variant::Str(value)) => Some(value.to_string()),
            _ => None,
        }
    }

    fn section_u32(sec: &HashMap<String, OwnedVariant>, key: &str) -> Option<u32> {
        match sec.get(key).map(|v| v.deref()) {
            Some(Variant::U32(value)) => Some(*value),
            _ => None,
        }
    }

    #[test]
    fn downgrades_sae_for_a_wpa2_only_ap() {
        let mut sec = sec_with_key_mgmt("sae");
        assert!(downgrade_sae_to_wpa_psk(&mut sec, ApSecurity::Wpa2).unwrap());
        assert_eq!(section_str(&sec, "key-mgmt").as_deref(), Some("wpa-psk"));
    }

    #[test]
    fn does_not_rewrite_wpa_psk_to_sae() {
        // NM supports SAE through wpa-psk; sae + pmf=optional fails Update.
        let mut sec = sec_with_key_mgmt("wpa-psk");
        assert!(!downgrade_sae_to_wpa_psk(&mut sec, ApSecurity::Wpa3).unwrap());
        assert_eq!(section_str(&sec, "key-mgmt").as_deref(), Some("wpa-psk"));
    }

    #[test]
    fn keeps_key_mgmt_that_works_with_the_ap() {
        for (key_mgmt, security) in [
            ("sae", ApSecurity::Wpa3),
            ("sae", ApSecurity::Mixed),
            ("wpa-psk", ApSecurity::Wpa2),
            ("wpa-psk", ApSecurity::Wpa3),
            ("wpa-psk", ApSecurity::Mixed),
        ] {
            let mut sec = sec_with_key_mgmt(key_mgmt);
            assert!(!downgrade_sae_to_wpa_psk(&mut sec, security).unwrap());
            assert_eq!(section_str(&sec, "key-mgmt").as_deref(), Some(key_mgmt));
        }
    }

    #[test]
    fn downgrade_never_touches_non_psk_security() {
        for security in [
            ApSecurity::Open,
            ApSecurity::Owe,
            ApSecurity::Enterprise,
            ApSecurity::Wep,
            ApSecurity::Unknown,
        ] {
            let mut sec = sec_with_key_mgmt("sae");
            assert!(!downgrade_sae_to_wpa_psk(&mut sec, security).unwrap());
            assert_eq!(section_str(&sec, "key-mgmt").as_deref(), Some("sae"));
        }
    }

    #[test]
    fn downgrade_does_not_add_a_missing_key_mgmt() {
        let mut sec = HashMap::new();
        assert!(!downgrade_sae_to_wpa_psk(&mut sec, ApSecurity::Wpa2).unwrap());
        assert!(!sec.contains_key("key-mgmt"));
    }

    #[test]
    fn new_section_uses_psk_only_for_sae_aps() {
        assert_eq!(new_section_key_mgmt(ApSecurity::Wpa2), "wpa-psk");
        assert_eq!(new_section_key_mgmt(ApSecurity::Mixed), "wpa-psk");
        assert_eq!(new_section_key_mgmt(ApSecurity::Wpa3), "sae");
    }

    #[test]
    fn write_psk_preserves_existing_secret_flags() {
        for flags in [1u32, 2u32] {
            let mut sec = HashMap::from([(
                "psk-flags".to_string(),
                OwnedVariant::try_from(Variant::from(flags)).unwrap(),
            )]);
            write_psk(&mut sec, "secret").unwrap();
            assert_eq!(section_u32(&sec, "psk-flags"), Some(flags));
            assert_eq!(section_str(&sec, "psk").as_deref(), Some("secret"));
        }
    }

    #[test]
    fn write_psk_defaults_new_section_to_system_storage() {
        let mut sec = HashMap::new();
        write_psk(&mut sec, "secret").unwrap();
        assert_eq!(section_u32(&sec, "psk-flags"), Some(0));
    }

    #[test]
    fn reads_a_profile_bssid() {
        let mut s = wifi_settings(None);
        assert_eq!(wireless_bssid(&s).unwrap(), None);

        let bytes: &[u8] = &[0xAA, 0xBB, 0x0C, 0xDD, 0xEE, 0xFF];
        s.get_mut("802-11-wireless").unwrap().insert(
            "bssid".to_string(),
            OwnedVariant::try_from(Variant::Array(bytes.into())).unwrap(),
        );
        let pinned = wireless_bssid(&s).unwrap().unwrap();
        assert!(bssid_matches(&pinned, Some("aa:bb:0c:dd:ee:ff")));
        assert!(!bssid_matches(&pinned, Some("11:22:33:44:55:66")));
        assert!(!bssid_matches(&pinned, None));
    }

    #[test]
    fn rejects_an_invalid_profile_bssid() {
        let mut s = wifi_settings(None);
        let bytes: &[u8] = &[0xAA, 0xBB];
        s.get_mut("802-11-wireless").unwrap().insert(
            "bssid".to_string(),
            OwnedVariant::try_from(Variant::Array(bytes.into())).unwrap(),
        );
        assert!(wireless_bssid(&s).is_err());
    }
}
