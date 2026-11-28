use crate::services::network::{
    NetworkBackend, NetworkData, NetworkEvent, PskOutcome, Redacted, WifiSecurity,
};
use ashell_services::rfkill;

use super::{AccessPointData, ActiveConnectionInfo, KnownConnection, Vpn};
use iced::futures::{
    Stream, StreamExt,
    stream::{self, select_all},
};
use itertools::Itertools;
use log::{debug, warn};
use std::{collections::HashMap, ops::Deref};
use zbus::{ObjectPath, OwnedObjectPath, OwnedVariant, Result, Variant, proxy};

pub struct NetworkDbus<'a>(NetworkManagerProxy<'a>);

impl super::NetworkBackend for NetworkDbus<'_> {
    async fn initialize_data(&self) -> anyhow::Result<super::NetworkData> {
        let nm = self;

        // airplane mode
        let bluetooth_soft_blocked = rfkill::bluetooth_soft_blocked().await;

        let wifi_present = nm.wifi_device_present().await?;

        let wifi_enabled = nm.wireless_enabled().await.unwrap_or_default();
        debug!("Wifi enabled: {wifi_enabled}");

        let airplane_mode = bluetooth_soft_blocked && !wifi_enabled;
        debug!("Airplane mode: {airplane_mode}");

        let active_connections = nm.active_connections_info().await?;
        debug!("Active connections: {active_connections:?}");

        let wireless_access_points = nm.wireless_access_points().await?;
        debug!("Wireless access points: {wireless_access_points:?}");

        let known_connections = nm
            .known_connections_internal(&wireless_access_points)
            .await?;
        debug!("Known connections: {known_connections:?}");

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

    async fn set_airplane_mode(&self, enable: bool) -> anyhow::Result<()> {
        if let Err(e) = rfkill::set_bluetooth_soft_block(enable).await {
            warn!("Failed to set bluetooth rfkill: {e:#}");
        } else {
            debug!("Bluetooth rfkill set successfully");
        }

        let nm = NetworkDbus::new(self.0.inner().connection()).await?;
        nm.set_wireless_enabled(!enable).await?;

        Ok(())
    }

    async fn scan_nearby_wifi(&self) -> anyhow::Result<()> {
        self.scan_nearby_wifi_with_devices().await?;
        Ok(())
    }

    async fn set_wifi_enabled(&self, enable: bool) -> anyhow::Result<()> {
        self.set_wireless_enabled(enable).await?;
        Ok(())
    }

    async fn select_access_point(
        &self,
        access_point: &AccessPointData,
        password: Option<String>,
        connect_once: bool,
    ) -> anyhow::Result<()> {
        let conn = self.0.inner().connection();
        let settings = NetworkSettingsDbus::new(conn).await?;
        let security = self.ap_security(access_point).await;

        if let Some(connection_path) = settings.find_connection(&access_point.ssid).await? {
            let connection = ConnectionSettingsProxy::builder(conn)
                .path(connection_path.clone())
                .build()
                .await?;

            let mut s = connection.get_settings().await?;
            let mut changed = false;
            let password = password.as_deref().filter(|_| security != ApSecurity::Open);

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

            let mut specific_object = OwnedObjectPath::try_from("/")?;
            if changed {
                let matches_pin = if let Some(pinned) = wireless_bssid(&s)? {
                    let selected = self.ap_bssid(access_point).await;
                    let matches = bssid_matches(&pinned, selected.as_deref());
                    if !matches && password.is_some() {
                        anyhow::bail!(
                            "saved profile '{}' is pinned to BSSID {pinned}, but the selected access point is {}; refusing to change its security",
                            access_point.ssid,
                            selected.as_deref().unwrap_or("unknown")
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

            self.activate_connection(
                connection_path,
                access_point.device_path.to_owned(),
                specific_object,
            )
            .await?;
        } else {
            let name = access_point.ssid.clone();
            debug!("Create new wifi connection: {name}");

            let conn_settings = wifi_connection_settings(
                &access_point.ssid,
                password.as_deref(),
                security,
                connect_once,
            )?;

            // "volatile" keeps the profile in memory only and NM deletes it on disconnect
            let persist = if connect_once { "volatile" } else { "disk" };
            debug!("Activating '{}' with persist={persist}", access_point.ssid);

            self.add_and_activate_connection2(
                conn_settings,
                &access_point.device_path,
                &access_point.path,
                HashMap::from([("persist", Variant::Str(persist.into()))]),
            )
            .await?;
        }

        Ok(())
    }

    async fn set_vpn(
        &self,
        connection: OwnedObjectPath,
        enable: bool,
    ) -> anyhow::Result<Vec<KnownConnection>> {
        if enable {
            debug!("Activating VPN: {connection:?}");
            self.activate_connection(
                connection,
                OwnedObjectPath::try_from("/").expect("D-Bus root path is always valid"),
                OwnedObjectPath::try_from("/").expect("D-Bus root path is always valid"),
            )
            .await?;
        } else {
            debug!("Deactivating VPN: {connection:?}");
            self.deactivate_connection(connection).await?;
        }

        let known_connections = self.known_connections().await?;
        Ok(known_connections)
    }

    async fn known_connections(&self) -> anyhow::Result<Vec<KnownConnection>> {
        let wireless_access_points = self.wireless_access_points().await?;
        self.known_connections_internal(&wireless_access_points)
            .await
    }
}

impl<'a> Deref for NetworkDbus<'a> {
    type Target = NetworkManagerProxy<'a>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl NetworkDbus<'_> {
    pub async fn new(conn: &zbus::Connection) -> anyhow::Result<Self> {
        let nm = NetworkManagerProxy::new(conn).await?;

        Ok(Self(nm))
    }

    async fn ap_security(&self, access_point: &AccessPointData) -> ApSecurity {
        let conn = self.0.inner().connection();
        let Ok(ap) = AccessPointProxy::builder(conn)
            .path(&access_point.path)
            .build()
            .await
        else {
            return ApSecurity::Unknown;
        };

        let (flags, rsn_flags, wpa_flags) =
            match iced::futures::try_join!(ap.flags(), ap.rsn_flags(), ap.wpa_flags()) {
                Ok(flags) => flags,
                Err(err) => {
                    debug!(
                        "Could not read security flags for '{}': {err}",
                        access_point.ssid
                    );
                    return ApSecurity::Unknown;
                }
            };

        let security = ap_security_from_flags(flags, rsn_flags, wpa_flags);
        debug!(
            "AP '{}' security: flags={flags:#x} rsn={rsn_flags:#x} wpa={wpa_flags:#x} -> {security:?}",
            access_point.ssid
        );
        security
    }

    async fn ap_bssid(&self, access_point: &AccessPointData) -> Option<String> {
        let conn = self.0.inner().connection();
        let ap = AccessPointProxy::builder(conn)
            .path(&access_point.path)
            .build()
            .await
            .ok()?;
        ap.hw_address().await.ok()
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

        let connection = ConnectionSettingsProxy::builder(self.0.inner().connection())
            .path(connection_path)
            .build()
            .await?;

        let hidden = all_settings
            .get("802-11-wireless")
            .and_then(|w| w.get("hidden"))
            .and_then(|v| match v.deref() {
                Variant::Bool(b) => Some(*b),
                _ => None,
            })
            .unwrap_or(false);

        // Open networks have no security section, and GetSecrets errors on a missing one.
        let Some(security_settings) = all_settings.get("802-11-wireless-security") else {
            return Ok(PskOutcome::Open { hidden });
        };

        let key_mgmt = security_settings
            .get("key-mgmt")
            .and_then(|v| match v.deref() {
                Variant::Str(s) => Some(s.to_string()),
                _ => None,
            });

        // 802.1X, OWE and static WEP have no PSK a `WIFI:` URI can carry.
        let security = match key_mgmt.as_deref() {
            Some("wpa-psk") | Some("wpa-psk-sha256") => WifiSecurity::Wpa,
            Some("sae") => WifiSecurity::Sae,
            _ => return Ok(PskOutcome::Unsupported),
        };

        let secrets = connection
            .get_secrets("802-11-wireless-security")
            .await
            .map_err(|e| anyhow::anyhow!("polkit/secret retrieval denied for '{ssid}': {e}"))?;

        let psk = secrets
            .get("802-11-wireless-security")
            .and_then(|sec| sec.get("psk"))
            .and_then(|v| match v.deref() {
                Variant::Str(v) => Some(v.to_string()),
                _ => None,
            });

        Ok(match psk {
            Some(psk) if !psk.is_empty() => PskOutcome::Password {
                psk: Redacted::new(psk),
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
        for active_path in self.active_connections().await? {
            let active = ActiveConnectionProxy::builder(self.0.inner().connection())
                .path(active_path)
                .build()
                .await?;

            if active.connection_type().await.ok().as_deref() != Some("802-11-wireless") {
                continue;
            }

            let Ok(profile_path) = active.connection().await else {
                continue;
            };

            let profile = ConnectionSettingsProxy::builder(self.0.inner().connection())
                .path(profile_path.clone())
                .build()
                .await?;

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

    pub async fn scan_nearby_wifi_with_devices(&self) -> anyhow::Result<Vec<OwnedObjectPath>> {
        let device_paths = self.wireless_devices().await?;
        let mut requested_devices = Vec::with_capacity(device_paths.len());

        for device_path in device_paths {
            let device_path_str = device_path.to_string();
            let device = WirelessDeviceProxy::builder(self.0.inner().connection())
                .path(device_path.clone())
                .build()
                .await?;

            match device.request_scan(HashMap::new()).await {
                Ok(()) => requested_devices.push(device_path),
                Err(e) => warn!("Scan request failed for device {device_path_str}: {e}"),
            }
        }

        Ok(requested_devices)
    }

    pub async fn subscribe_events(
        &self,
    ) -> anyhow::Result<impl Stream<Item = super::NetworkEvent>> {
        let nm = self;
        let conn = self.0.inner().connection();
        let settings = NetworkSettingsDbus::new(conn).await?;

        let wireless_enabled = nm
            .receive_wireless_enabled_changed()
            .await
            .then(|v| async move {
                let value = v.get().await.unwrap_or_default();

                debug!("WiFi enabled changed: {value}");
                NetworkEvent::WiFiEnabled(value)
            })
            .boxed();

        let connectivity_changed = nm
            .receive_connectivity_changed()
            .await
            .then(|val| async move {
                let value = val.get().await.unwrap_or_default().into();

                debug!("Connectivity changed: {value:?}");
                NetworkEvent::Connectivity(value)
            })
            .boxed();

        let active_connections_changes = nm
            .receive_active_connections_changed()
            .await
            .then({
                let conn = conn.clone();
                move |_| {
                    let conn = conn.clone();
                    async move {
                        let value = match NetworkDbus::new(&conn).await {
                            Ok(nm) => nm.active_connections_info().await.unwrap_or_default(),
                            Err(e) => {
                                warn!("Failed to create NetworkDbus proxy: {e}");
                                Vec::new()
                            }
                        };

                        debug!("Active connections changed: {value:?}");
                        NetworkEvent::ActiveConnections(value)
                    }
                }
            })
            .boxed();

        let devices = nm.wireless_devices().await.unwrap_or_default();

        let wireless_devices_changed = nm
            .receive_devices_changed()
            .await
            .then({
                let conn = conn.clone();
                let devices = devices.clone();
                move |_| {
                    let conn = conn.clone();
                    let devices = devices.clone();
                    async move {
                        let nm = match NetworkDbus::new(&conn).await {
                            Ok(nm) => nm,
                            Err(e) => {
                                warn!("Failed to create NetworkDbus proxy: {e}");
                                return Vec::new();
                            }
                        };

                        let current_devices = nm.wireless_devices().await.unwrap_or_default();
                        if current_devices != devices {
                            let wifi_present = nm.wifi_device_present().await.unwrap_or_default();
                            let wireless_access_points =
                                nm.wireless_access_points().await.unwrap_or_default();

                            debug!(
                                "Wireless device changed: wifi present {wifi_present:?}, wireless_access_points {wireless_access_points:?}",
                            );
                            // WirelessDevice ends the events loop, so known connections go first
                            nm.refreshed_known_connections(&wireless_access_points)
                                .await
                                .into_iter()
                                .chain([NetworkEvent::WirelessDevice {
                                    wifi_present,
                                    wireless_access_points,
                                }])
                                .collect()
                        } else {
                            Vec::new()
                        }
                    }
                }
            })
            .flat_map(stream::iter)
            .boxed();

        // When devices list change I need to update the wireless device state changes
        let wireless_ac = nm.wireless_access_points().await?;

        let mut device_state_changes = Vec::with_capacity(wireless_ac.len());
        for ac in wireless_ac.iter() {
            let dp = DeviceProxy::builder(conn)
                .path(ac.device_path.clone())
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
                        let ssid = ac.ssid.clone();

                        debug!("Request password for ssid {ssid}");
                        NetworkEvent::RequestPasswordForSSID(ssid)
                    }),
            );
        }

        // Set up access point change listeners on wireless devices
        let mut ac_changes = Vec::with_capacity(devices.len());
        for device_path in devices.iter() {
            let dp = WirelessDeviceProxy::builder(conn)
                .path(device_path.clone())
                .build()
                .await?;

            ac_changes.push(
                dp.receive_access_points_changed()
                    .await
                    .then({
                        let conn = conn.clone();
                        move |_| {
                            let conn = conn.clone();
                            async move {
                                let nm = match NetworkDbus::new(&conn).await {
                                    Ok(nm) => nm,
                                    Err(e) => {
                                        warn!("Failed to create NetworkDbus proxy: {e}");
                                        return vec![NetworkEvent::WirelessAccessPoint(Vec::new())];
                                    }
                                };
                                let wireless_access_point =
                                    nm.wireless_access_points().await.unwrap_or_default();
                                debug!(
                                    "access_points_changed event received, count: {}",
                                    wireless_access_point.len()
                                );

                                let known_connections =
                                    nm.refreshed_known_connections(&wireless_access_point).await;
                                [NetworkEvent::WirelessAccessPoint(wireless_access_point)]
                                    .into_iter()
                                    .chain(known_connections)
                                    .collect::<Vec<_>>()
                            }
                        }
                    })
                    .flat_map(stream::iter)
                    .boxed(),
            );
        }

        // When devices list change I need to update the wireless strength changes
        let mut strength_changes = Vec::with_capacity(wireless_ac.len());
        for ap in wireless_ac {
            let ssid = ap.ssid.clone();
            let path = ap.path.clone();
            let app = AccessPointProxy::builder(conn)
                .path(ap.path.clone())
                .build()
                .await?;

            strength_changes.push(
                app.receive_strength_changed()
                    .await
                    .then(move |val| {
                        let ssid = ssid.clone();
                        let path = path.clone();
                        async move {
                            let value = val.get().await.unwrap_or_default();
                            debug!("Strength changed value: {}, {}", ssid, value);
                            NetworkEvent::Strength((Some(path.clone()), ssid.clone(), value))
                        }
                    })
                    .boxed(),
            );
        }
        let strength_changes = select_all(strength_changes).boxed();

        let access_points = select_all(ac_changes).boxed();

        // Set up LastScan change listeners on wireless devices to detect scan completion
        let mut last_scan_changes = Vec::with_capacity(devices.len());
        for device_path in devices.iter() {
            let dp = WirelessDeviceProxy::builder(conn)
                .path(device_path.clone())
                .build()
                .await?;

            last_scan_changes.push(
                dp.receive_last_scan_changed()
                    .await
                    .then({
                        let device_path = device_path.clone();
                        move |_| {
                            let device_path = device_path.clone();
                            async move { NetworkEvent::ScanCompleted(device_path) }
                        }
                    })
                    .boxed(),
            );
        }
        let last_scan_changes = select_all(last_scan_changes).boxed();

        let known_connections = settings
            .receive_connections_changed()
            .await
            .then({
                let conn = conn.clone();
                move |_| {
                    let conn = conn.clone();
                    async move {
                        let known_connections = match NetworkDbus::new(&conn).await {
                            Ok(nm) => nm.known_connections().await.unwrap_or_default(),
                            Err(e) => {
                                warn!("Failed to create NetworkDbus proxy: {e}");
                                Vec::new()
                            }
                        };

                        debug!("Known connections changed");
                        NetworkEvent::KnownConnections(known_connections)
                    }
                }
            })
            .boxed();

        let events = select_all(vec![
            wireless_enabled,
            wireless_devices_changed,
            connectivity_changed,
            active_connections_changes,
            access_points,
            strength_changes,
            last_scan_changes,
            known_connections,
        ]);

        Ok(events)
    }

    pub async fn connectivity(&self) -> Result<ConnectivityState> {
        self.0.connectivity().await.map(ConnectivityState::from)
    }

    pub async fn wifi_device_present(&self) -> anyhow::Result<bool> {
        let devices = self.devices().await?;
        for d in devices {
            let device = DeviceProxy::builder(self.0.inner().connection())
                .path(d)
                .build()
                .await?;

            if matches!(
                device.device_type().await.map(DeviceType::from),
                Ok(DeviceType::Wifi)
            ) {
                return Ok(true);
            }
        }

        Ok(false)
    }

    pub async fn active_connections(&self) -> anyhow::Result<Vec<OwnedObjectPath>> {
        let connections = self.0.active_connections().await?;

        Ok(connections)
    }

    pub async fn active_connections_info(&self) -> anyhow::Result<Vec<ActiveConnectionInfo>> {
        let active_connections = self.active_connections().await?;
        let mut ac_proxies: Vec<ActiveConnectionProxy> =
            Vec::with_capacity(active_connections.len());
        for active_connection in &active_connections {
            let active_connection = ActiveConnectionProxy::builder(self.0.inner().connection())
                .path(active_connection)
                .build()
                .await?;
            ac_proxies.push(active_connection);
        }

        let mut info = Vec::<ActiveConnectionInfo>::with_capacity(active_connections.len());
        for connection in ac_proxies {
            if connection.vpn().await.unwrap_or_default() {
                info.push(ActiveConnectionInfo::Vpn {
                    name: connection.id().await?,
                    object_path: connection.inner().path().to_owned().into(),
                });
                continue;
            }
            for device in connection.devices().await.unwrap_or_default() {
                let device = DeviceProxy::builder(self.0.inner().connection())
                    .path(device)
                    .build()
                    .await?;

                match device.device_type().await.map(DeviceType::from).ok() {
                    Some(DeviceType::Ethernet) => {
                        info.push(ActiveConnectionInfo::Wired {
                            name: connection.id().await?,
                        });
                    }
                    Some(DeviceType::Wifi) => {
                        let wireless_device =
                            WirelessDeviceProxy::builder(self.0.inner().connection())
                                .path(device.0.path())
                                .build()
                                .await?;

                        if let Ok(access_point) = wireless_device.active_access_point().await {
                            let access_point =
                                AccessPointProxy::builder(self.0.inner().connection())
                                    .path(access_point)
                                    .build()
                                    .await?;

                            let freq = access_point.frequency().await.unwrap_or_default();
                            info.push(ActiveConnectionInfo::WiFi {
                                name: String::from_utf8_lossy(&access_point.ssid().await?)
                                    .into_owned(),
                                strength: access_point.strength().await.unwrap_or_default(),
                                frequency: freq,
                            });
                        }
                    }
                    Some(DeviceType::WireGuard) => {
                        info.push(ActiveConnectionInfo::Vpn {
                            name: connection.id().await?,
                            object_path: connection.inner().path().to_owned().into(),
                        });
                    }
                    _ => {}
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

        Ok(info)
    }
}

type ConnectionSettingsMap = HashMap<String, HashMap<String, OwnedVariant>>;

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

    if !psk && !sae {
        return match (sec == 0, flags & AP_FLAG_PRIVACY != 0) {
            (true, false) => ApSecurity::Open,
            // NM treats privacy with no WPA/RSN flags as WEP (static or dynamic).
            (true, true) => ApSecurity::Wep,
            (false, _) => ApSecurity::Unknown,
        };
    }

    match (psk, sae) {
        (true, true) => ApSecurity::Mixed,
        (true, false) => ApSecurity::Wpa2,
        (false, true) => ApSecurity::Wpa3,
        (false, false) => unreachable!("handled above"),
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
fn downgrade_sae_to_wpa_psk(
    sec: &mut HashMap<String, OwnedVariant>,
    security: ApSecurity,
) -> anyhow::Result<bool> {
    if security != ApSecurity::Wpa2 {
        return Ok(false);
    }

    let is_sae = matches!(
        sec.get("key-mgmt").map(|v| v.deref()),
        Some(Variant::Str(key_mgmt)) if key_mgmt == "sae"
    );
    if !is_sae {
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

fn write_psk(sec: &mut HashMap<String, OwnedVariant>, password: &str) -> anyhow::Result<()> {
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
    let Some(bssid) = settings
        .get("802-11-wireless")
        .and_then(|wifi| wifi.get("bssid"))
    else {
        return Ok(None);
    };
    if let Variant::Array(bytes) = bssid.deref()
        && let [
            Variant::U8(a),
            Variant::U8(b),
            Variant::U8(c),
            Variant::U8(d),
            Variant::U8(e),
            Variant::U8(f),
        ] = bytes.as_ref()
    {
        return Ok(Some(format!(
            "{a:02X}:{b:02X}:{c:02X}:{d:02X}:{e:02X}:{f:02X}"
        )));
    }
    anyhow::bail!("invalid BSSID in saved Wi-Fi profile")
}

fn connection_ssid(settings: &ConnectionSettingsMap) -> Option<String> {
    let bytes: Vec<u8> = match settings.get("802-11-wireless")?.get("ssid")?.deref() {
        Variant::Array(bytes) => bytes
            .iter()
            .map(|b| match b {
                Variant::U8(b) => Some(*b),
                _ => None,
            })
            .collect::<Option<Vec<u8>>>()?,
        _ => return None,
    };

    Some(String::from_utf8_lossy(&bytes).into_owned())
}

/// SSID of a profile that joins networks as a client. Hotspot (`ap`), ad-hoc
/// and mesh profiles share the SSID field but can't join a scanned network.
fn client_connection_ssid(settings: &ConnectionSettingsMap) -> Option<String> {
    let mode = settings
        .get("802-11-wireless")?
        .get("mode")
        .and_then(|v| match v.deref() {
            Variant::Str(s) => Some(s.to_string()),
            _ => None,
        });

    match mode.as_deref() {
        None | Some("infrastructure") => connection_ssid(settings),
        _ => None,
    }
}

fn connection_id(settings: &HashMap<String, HashMap<String, OwnedVariant>>) -> Option<String> {
    settings
        .get("connection")?
        .get("id")
        .and_then(|v| match v.deref() {
            Variant::Str(v) => Some(v.to_string()),
            _ => None,
        })
}

impl NetworkDbus<'_> {
    /// Known Wi-Fi connections only include visible access points, so they
    /// must be recomputed whenever the access point list changes.
    async fn refreshed_known_connections(
        &self,
        wireless_access_points: &[AccessPointData],
    ) -> Option<NetworkEvent> {
        match self
            .known_connections_internal(wireless_access_points)
            .await
        {
            Ok(known_connections) => Some(NetworkEvent::KnownConnections(known_connections)),
            Err(e) => {
                warn!("Failed to refresh known connections: {e}");
                None
            }
        }
    }

    pub async fn known_connections_internal(
        &self,
        wireless_access_points: &[AccessPointData],
    ) -> anyhow::Result<Vec<KnownConnection>> {
        let settings = NetworkSettingsDbus::new(self.0.inner().connection()).await?;

        let known_connections = settings.know_connections().await?;

        let mut known_ssid = Vec::with_capacity(known_connections.len());
        let mut known_vpn = Vec::new();
        for c in known_connections {
            let cs = ConnectionSettingsProxy::builder(self.0.inner().connection())
                .path(c.clone())
                .build()
                .await?;
            let Ok(s) = cs.get_settings().await else {
                warn!("Failed to get settings for connection {c}");
                continue;
            };

            let wifi = s.get("802-11-wireless");

            if wifi.is_some() {
                if let Some(cur_ssid) = client_connection_ssid(&s) {
                    known_ssid.push(cur_ssid);
                }
            } else if (s.contains_key("vpn") || s.contains_key("wireguard"))
                && let Some(id) = connection_id(&s)
            {
                known_vpn.push(Vpn { name: id, path: c });
            }
        }
        let known_connections: Vec<_> = wireless_access_points
            .iter()
            .filter_map(|a| {
                if known_ssid.contains(&a.ssid) {
                    Some(KnownConnection::AccessPoint(a.clone()))
                } else {
                    None
                }
            })
            .chain(known_vpn.into_iter().map(KnownConnection::Vpn))
            .collect();

        Ok(known_connections)
    }

    pub async fn wireless_devices(&self) -> anyhow::Result<Vec<OwnedObjectPath>> {
        let devices = self.devices().await?;
        let mut wireless_devices = Vec::new();
        for d in devices {
            let device = DeviceProxy::builder(self.0.inner().connection())
                .path(&d)
                .build()
                .await?;

            if matches!(
                device.device_type().await.map(DeviceType::from),
                Ok(DeviceType::Wifi)
            ) {
                wireless_devices.push(d);
            }
        }

        Ok(wireless_devices)
    }

    pub async fn wireless_access_points(&self) -> anyhow::Result<Vec<AccessPointData>> {
        let wireless_devices = self.wireless_devices().await?;
        let wireless_access_point_futures: Vec<_> = wireless_devices
            .into_iter()
            .map(|path| async move {
                let device = DeviceProxy::builder(self.0.inner().connection())
                    .path(&path)
                    .build()
                    .await?;
                let wireless_device = WirelessDeviceProxy::builder(self.0.inner().connection())
                    .path(&path)
                    .build()
                    .await?;
                // disable scan for now for performance reason
                // wireless_device.request_scan(HashMap::new()).await?;
                // let mut scan_changed = wireless_device.receive_last_scan_changed().await;
                // if let Some(t) = scan_changed.next().await {
                //     if let Ok(-1) = t.get().await {
                //         return Ok(Default::default());
                //     }
                // }
                let access_points = wireless_device.get_access_points().await?;
                let state: DeviceState = device
                    .cached_state()
                    .unwrap_or_default()
                    .map_or(DeviceState::Unknown, DeviceState::from);

                // Sort by strength and remove duplicates
                let mut aps = HashMap::<String, AccessPointData>::new();
                for ap in access_points {
                    let ap = AccessPointProxy::builder(self.0.inner().connection())
                        .path(ap)
                        .build()
                        .await?;

                    let ssid = String::from_utf8_lossy(&ap.ssid().await?).into_owned();
                    let public = ap.flags().await.unwrap_or_default() == 0;
                    let strength = ap.strength().await?;
                    let max_bitrate = ap.max_bitrate().await.unwrap_or_default();
                    let frequency = ap.frequency().await.unwrap_or_default();
                    if let Some(access_point) = aps.get(&ssid)
                        && AccessPointData::is_better(
                            access_point.max_bitrate,
                            access_point.frequency,
                            access_point.strength,
                            max_bitrate,
                            frequency,
                            strength,
                        )
                    {
                        continue;
                    }

                    aps.insert(
                        ssid.clone(),
                        AccessPointData {
                            ssid,
                            strength,
                            max_bitrate,
                            frequency,
                            state,
                            public,
                            working: false,
                            path: ap.inner().path().clone().into(),
                            device_path: device.0.path().clone().into(),
                        },
                    );
                }

                let aps = aps
                    .into_values()
                    .sorted_by(|a, b| b.strength.cmp(&a.strength))
                    .collect();

                Ok(aps)
            })
            .collect();

        let mut wireless_access_points = Vec::with_capacity(wireless_access_point_futures.len());
        for f in wireless_access_point_futures {
            let mut access_points: anyhow::Result<Vec<AccessPointData>> = f.await;
            if let Ok(access_points) = &mut access_points {
                wireless_access_points.append(access_points);
            }
        }

        let wireless_access_points = wireless_access_points
            .into_iter()
            .fold(HashMap::<String, AccessPointData>::new(), |mut acc, ap| {
                if let Some(existing) = acc.get(&ap.ssid)
                    && AccessPointData::is_better(
                        existing.max_bitrate,
                        existing.frequency,
                        existing.strength,
                        ap.max_bitrate,
                        ap.frequency,
                        ap.strength,
                    )
                {
                    return acc;
                }

                acc.insert(ap.ssid.clone(), ap);
                acc
            })
            .into_values()
            .sorted_by(|a, b| b.strength.cmp(&a.strength))
            .collect();

        Ok(wireless_access_points)
    }
}

pub struct NetworkSettingsDbus<'a>(SettingsProxy<'a>);

impl<'a> Deref for NetworkSettingsDbus<'a> {
    type Target = SettingsProxy<'a>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl NetworkSettingsDbus<'_> {
    pub async fn new(conn: &zbus::Connection) -> anyhow::Result<Self> {
        let settings = SettingsProxy::new(conn).await?;

        Ok(Self(settings))
    }

    pub async fn know_connections(&self) -> anyhow::Result<Vec<OwnedObjectPath>> {
        Ok(self.list_connections().await?)
    }

    /// Saved client Wi-Fi profile for `ssid`, matched by the profile's SSID since its
    /// name (`connection.id`) can be anything.
    pub async fn find_connection(&self, ssid: &str) -> anyhow::Result<Option<OwnedObjectPath>> {
        let connections = self.list_connections().await?;

        for connection in connections {
            let connection = ConnectionSettingsProxy::builder(self.inner().connection())
                .path(connection)
                .build()
                .await?;

            let s = connection.get_settings().await?;
            if client_connection_ssid(&s).as_deref() == Some(ssid) {
                return Ok(Some(connection.inner().path().to_owned().into()));
            }
        }

        Ok(None)
    }
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceType {
    Ethernet,
    Wifi,
    Bluetooth,
    TunTap,
    WireGuard,
    Generic,
    Other,
    #[default]
    Unknown,
}

impl From<u32> for DeviceType {
    fn from(device_type: u32) -> DeviceType {
        match device_type {
            1 => DeviceType::Ethernet,
            2 => DeviceType::Wifi,
            5 => DeviceType::Bluetooth,
            14 => DeviceType::Generic,
            16 => DeviceType::TunTap,
            29 => DeviceType::WireGuard,
            3..=32 => DeviceType::Other,
            _ => DeviceType::Unknown,
        }
    }
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectivityState {
    None,
    Portal,
    Loss,
    Full,
    #[default]
    Unknown,
}

impl From<u32> for ConnectivityState {
    fn from(state: u32) -> ConnectivityState {
        match state {
            1 => ConnectivityState::None,
            2 => ConnectivityState::Portal,
            3 => ConnectivityState::Loss,
            4 => ConnectivityState::Full,
            _ => ConnectivityState::Unknown,
        }
    }
}

// Used by iwd
impl From<String> for ConnectivityState {
    fn from(state: String) -> ConnectivityState {
        match state.as_str() {
            "inactive" | "disconnected" => ConnectivityState::None,
            "portal" => ConnectivityState::Portal,
            "failed" => ConnectivityState::Loss,
            "connected" => ConnectivityState::Full,
            _ => ConnectivityState::Unknown, // scanning, connecting
        }
    }
}

impl From<Vec<ConnectivityState>> for ConnectivityState {
    fn from(states: Vec<ConnectivityState>) -> ConnectivityState {
        if states.is_empty() {
            return ConnectivityState::Unknown;
        }

        let mut state = states[0];
        for s in states.iter().skip(1) {
            if Into::<u32>::into(*s) >= state.into() {
                state = *s;
            }
        }

        state
    }
}

impl From<ConnectivityState> for u32 {
    #[inline]
    fn from(val: ConnectivityState) -> Self {
        match val {
            ConnectivityState::None => 1,
            ConnectivityState::Portal => 2,
            ConnectivityState::Loss => 3,
            ConnectivityState::Full => 4,
            _ => 0,
        }
    }
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceState {
    Unmanaged,
    Unavailable,
    Disconnected,
    Prepare,
    Config,
    NeedAuth,
    IpConfig,
    IpCheck,
    Secondaries,
    Activated,
    Deactivating,
    Failed,
    #[default]
    Unknown,
}

impl From<u32> for DeviceState {
    fn from(device_state: u32) -> Self {
        match device_state {
            10 => DeviceState::Unmanaged,
            20 => DeviceState::Unavailable,
            30 => DeviceState::Disconnected,
            40 => DeviceState::Prepare,
            50 => DeviceState::Config,
            60 => DeviceState::NeedAuth,
            70 => DeviceState::IpConfig,
            80 => DeviceState::IpCheck,
            90 => DeviceState::Secondaries,
            100 => DeviceState::Activated,
            110 => DeviceState::Deactivating,
            120 => DeviceState::Failed,
            _ => DeviceState::Unknown,
        }
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
    fn active_connections(&self) -> Result<Vec<OwnedObjectPath>>;

    #[zbus(property)]
    fn devices(&self) -> Result<Vec<OwnedObjectPath>>;

    #[zbus(property)]
    fn wireless_enabled(&self) -> Result<bool>;

    #[zbus(property)]
    fn set_wireless_enabled(&self, value: bool) -> Result<()>;

    #[zbus(property)]
    fn connectivity(&self) -> Result<u32>;
}

#[proxy(
    default_service = "org.freedesktop.NetworkManager",
    default_path = "/org/freedesktop/NetworkManager/Connection/Active",
    interface = "org.freedesktop.NetworkManager.Connection.Active"
)]
trait ActiveConnection {
    #[zbus(property)]
    fn id(&self) -> Result<String>;

    #[zbus(property)]
    fn uuid(&self) -> Result<String>;

    #[zbus(property)]
    fn connection(&self) -> Result<OwnedObjectPath>;

    #[zbus(property, name = "Type")]
    fn connection_type(&self) -> Result<String>;

    #[zbus(property)]
    fn state(&self) -> Result<u32>;

    #[zbus(property)]
    fn vpn(&self) -> Result<bool>;

    #[zbus(property)]
    fn devices(&self) -> Result<Vec<OwnedObjectPath>>;
}

#[proxy(
    default_service = "org.freedesktop.NetworkManager",
    default_path = "/org/freedesktop/NetworkManager/Device",
    interface = "org.freedesktop.NetworkManager.Device"
)]
pub trait Device {
    #[zbus(property)]
    fn device_type(&self) -> Result<u32>;

    #[zbus(property)]
    fn available_connections(&self) -> Result<Vec<OwnedObjectPath>>;

    #[zbus(property)]
    fn active_connection(&self) -> Result<OwnedObjectPath>;

    #[zbus(property)]
    fn state(&self) -> Result<u32>;
}

#[proxy(
    default_service = "org.freedesktop.NetworkManager",
    default_path = "/org/freedesktop/NetworkManager/Device/Wireless",
    interface = "org.freedesktop.NetworkManager.Device.Wireless"
)]
pub trait WirelessDevice {
    /// `GetAccessPoints` method
    fn get_access_points(&self) -> zbus::Result<Vec<zbus::OwnedObjectPath>>;

    #[zbus(property)]
    fn active_access_point(&self) -> Result<OwnedObjectPath>;

    #[zbus(property)]
    fn access_points(&self) -> Result<Vec<OwnedObjectPath>>;

    #[zbus(property)]
    fn last_scan(&self) -> zbus::Result<i64>;

    fn request_scan(&self, options: HashMap<String, OwnedVariant>) -> Result<()>;
}

#[proxy(
    default_service = "org.freedesktop.NetworkManager",
    default_path = "/org/freedesktop/NetworkManager/AccessPoint",
    interface = "org.freedesktop.NetworkManager.AccessPoint"
)]
pub trait AccessPoint {
    #[zbus(property)]
    fn ssid(&self) -> Result<Vec<u8>>;

    #[zbus(property)]
    fn max_bitrate(&self) -> Result<u32>;

    #[zbus(property)]
    fn frequency(&self) -> Result<u32>;

    #[zbus(property)]
    fn strength(&self) -> Result<u8>;

    #[zbus(property)]
    fn flags(&self) -> Result<u32>;

    #[zbus(property)]
    fn hw_address(&self) -> Result<String>;

    #[zbus(property)]
    fn rsn_flags(&self) -> Result<u32>;

    #[zbus(property)]
    fn wpa_flags(&self) -> Result<u32>;
}

#[proxy(
    default_service = "org.freedesktop.NetworkManager",
    default_path = "/org/freedesktop/NetworkManager/Settings",
    interface = "org.freedesktop.NetworkManager.Settings"
)]
pub trait Settings {
    fn add_connection(
        &self,
        connection: HashMap<String, HashMap<String, OwnedVariant>>,
    ) -> Result<OwnedObjectPath>;

    #[zbus(property)]
    fn connections(&self) -> Result<Vec<OwnedObjectPath>>;

    fn load_connections(&self, filenames: &[&str]) -> Result<(bool, Vec<String>)>;

    fn list_connections(&self) -> zbus::Result<Vec<OwnedObjectPath>>;
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
        assert!(sec.get("key-mgmt").is_none());
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
