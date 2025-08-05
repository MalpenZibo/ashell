use crate::services::{
    bluetooth::BluetoothService,
    network::{NetworkBackend, NetworkData, NetworkEvent},
};

use super::{AccessPoint, ActiveConnectionInfo, KnownConnection, Vpn};
use iced::futures::{Stream, StreamExt, stream::select_all};
use itertools::Itertools;
use log::{debug, warn};
use std::{collections::HashMap, ops::Deref};
use tokio::process::Command;
use zbus::{
    Result, proxy,
    zvariant::{self, ObjectPath, OwnedObjectPath, OwnedValue, Value},
};

pub struct NetworkDbus<'a>(NetworkManagerProxy<'a>);

impl super::NetworkBackend for NetworkDbus<'_> {
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
        let rfkill_res = Command::new("/usr/sbin/rfkill")
            .arg(if enable { "block" } else { "unblock" })
            .arg("bluetooth")
            .output()
            .await;

        if let Err(e) = rfkill_res {
            debug!("Failed to set bluetooth rfkill: {e}");
        } else {
            debug!("Bluetooth rfkill set successfully");
        }

        let nm = NetworkDbus::new(self.0.inner().connection()).await?;
        nm.set_wireless_enabled(!enable).await?;

        Ok(())
    }

    async fn scan_nearby_wifi(&self) -> anyhow::Result<()> {
        for device_path in self
            .wireless_access_points()
            .await?
            .iter()
            .map(|ap| ap.path.clone())
        {
            let device = WirelessDeviceProxy::builder(self.0.inner().connection())
                .path(device_path)?
                .build()
                .await?;

            device.request_scan(HashMap::new()).await?;
        }

        Ok(())
    }

    async fn set_wifi_enabled(&self, enable: bool) -> anyhow::Result<()> {
        self.set_wireless_enabled(enable).await?;
        Ok(())
    }

    async fn select_access_point(
        &mut self,
        access_point: &AccessPoint,
        password: Option<String>,
    ) -> anyhow::Result<()> {
        let settings = NetworkSettingsDbus::new(self.0.inner().connection()).await?;
        let connection = settings.find_connection(&access_point.ssid).await?;

        if let Some(connection) = connection.as_ref() {
            if let Some(password) = password {
                let connection = ConnectionSettingsProxy::builder(self.0.inner().connection())
                    .path(connection)?
                    .build()
                    .await?;

                let mut s = connection.get_settings().await?;
                if let Some(wifi_settings) = s.get_mut("802-11-wireless-security") {
                    let new_password = zvariant::Value::from(password.clone()).try_to_owned()?;
                    wifi_settings.insert("psk".to_string(), new_password);
                }

                connection.update(s).await?;
            }

            self.activate_connection(
                connection.clone(),
                access_point.device_path.to_owned(),
                OwnedObjectPath::try_from("/")?,
            )
            .await?;
        } else {
            let name = access_point.ssid.clone();
            debug!("Create new wifi connection: {name}");

            let mut conn_settings: HashMap<&str, HashMap<&str, zvariant::Value>> = HashMap::from([
                (
                    "802-11-wireless",
                    HashMap::from([("ssid", Value::Array(name.as_bytes().into()))]),
                ),
                (
                    "connection",
                    HashMap::from([
                        ("id", Value::Str(name.into())),
                        ("type", Value::Str("802-11-wireless".into())),
                    ]),
                ),
            ]);

            if let Some(pass) = password {
                conn_settings.insert(
                    "802-11-wireless-security",
                    HashMap::from([
                        ("psk", Value::Str(pass.into())),
                        ("key-mgmt", Value::Str("wpa-psk".into())),
                    ]),
                );
            }

            self.add_and_activate_connection(
                conn_settings,
                &access_point.device_path,
                &access_point.path,
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
                OwnedObjectPath::try_from("/").unwrap(),
                OwnedObjectPath::try_from("/").unwrap(),
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
                        let nm = NetworkDbus::new(&conn).await.unwrap();
                        let value = nm.active_connections_info().await.unwrap_or_default();

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
            .filter_map({
                let conn = conn.clone();
                let devices = devices.clone();
                move |_| {
                    let conn = conn.clone();
                    let devices = devices.clone();
                    async move {
                        let nm = NetworkDbus::new(&conn).await.unwrap();

                        let current_devices = nm.wireless_devices().await.unwrap_or_default();
                        if current_devices != devices {
                            let wifi_present = nm.wifi_device_present().await.unwrap_or_default();
                            let wireless_access_points =
                                nm.wireless_access_points().await.unwrap_or_default();

                            debug!(
                                "Wireless device changed: wifi present {wifi_present:?}, wireless_access_points {wireless_access_points:?}",
                            );
                            Some(NetworkEvent::WirelessDevice {
                                wifi_present,
                                wireless_access_points,
                            })
                        } else {
                            None
                        }
                    }
                }
            })
            .boxed();

        // When devices list change I need to update the wireless device state changes
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
                        let ssid = ac.ssid.clone();

                        debug!("Request password for ssid {ssid}");
                        NetworkEvent::RequestPasswordForSSID(ssid)
                    }),
            );
        }

        // When devices list change I need to update the access points changes
        let mut ac_changes = Vec::with_capacity(wireless_ac.len());
        for ac in wireless_ac.iter() {
            let dp = WirelessDeviceProxy::builder(conn)
                .path(ac.device_path.clone())?
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
                                let nm = NetworkDbus::new(&conn).await.unwrap();
                                let wireless_access_point =
                                    nm.wireless_access_points().await.unwrap_or_default();
                                debug!("access_points_changed {wireless_access_point:?}");

                                NetworkEvent::WirelessAccessPoint(wireless_access_point)
                            }
                        }
                    })
                    .boxed(),
            );
        }

        // When devices list change I need to update the wireless strength changes
        let mut strength_changes = Vec::with_capacity(wireless_ac.len());
        for ap in wireless_ac {
            let ssid = ap.ssid.clone();
            let app = AccessPointProxy::builder(conn)
                .path(ap.path.clone())?
                .build()
                .await?;

            strength_changes.push(
                app.receive_strength_changed()
                    .await
                    .then(move |val| {
                        let ssid = ssid.clone();
                        async move {
                            let value = val.get().await.unwrap_or_default();
                            debug!("Strength changed value: {}, {}", &ssid, value);
                            NetworkEvent::Strength((ssid.clone(), value))
                        }
                    })
                    .boxed(),
            );
        }
        let strength_changes = select_all(strength_changes).boxed();

        let access_points = select_all(ac_changes).boxed();

        let known_connections = settings
            .receive_connections_changed()
            .await
            .then({
                let conn = conn.clone();
                move |_| {
                    let conn = conn.clone();
                    async move {
                        let nm = NetworkDbus::new(&conn).await.unwrap();
                        let known_connections = nm.known_connections().await.unwrap_or_default();

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
                .path(d)?
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
                .path(active_connection)?
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
                    .path(device)?
                    .build()
                    .await?;

                match device.device_type().await.map(DeviceType::from).ok() {
                    Some(DeviceType::Ethernet) => {
                        let wired_device = WiredDeviceProxy::builder(self.0.inner().connection())
                            .path(device.0.path())?
                            .build()
                            .await?;

                        info.push(ActiveConnectionInfo::Wired {
                            name: connection.id().await?,
                            speed: wired_device.speed().await?,
                        });
                    }
                    Some(DeviceType::Wifi) => {
                        let wireless_device =
                            WirelessDeviceProxy::builder(self.0.inner().connection())
                                .path(device.0.path())?
                                .build()
                                .await?;

                        if let Ok(access_point) = wireless_device.active_access_point().await {
                            let access_point =
                                AccessPointProxy::builder(self.0.inner().connection())
                                    .path(access_point)?
                                    .build()
                                    .await?;

                            info.push(ActiveConnectionInfo::WiFi {
                                id: connection.id().await?,
                                name: String::from_utf8_lossy(&access_point.ssid().await?)
                                    .into_owned(),
                                strength: access_point.strength().await.unwrap_or_default(),
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
            let helper = |conn: &ActiveConnectionInfo| match conn {
                ActiveConnectionInfo::Vpn { name, .. } => format!("0{name}"),
                ActiveConnectionInfo::Wired { name, .. } => format!("1{name}"),
                ActiveConnectionInfo::WiFi { name, .. } => format!("2{name}"),
            };
            helper(a).cmp(&helper(b))
        });

        Ok(info)
    }

    pub async fn known_connections_internal(
        &self,
        wireless_access_points: &[AccessPoint],
    ) -> anyhow::Result<Vec<KnownConnection>> {
        let settings = NetworkSettingsDbus::new(self.0.inner().connection()).await?;

        let known_connections = settings.know_connections().await?;

        let mut known_ssid = Vec::with_capacity(known_connections.len());
        let mut known_vpn = Vec::new();
        for c in known_connections {
            let cs = ConnectionSettingsProxy::builder(self.0.inner().connection())
                .path(c.clone())?
                .build()
                .await?;
            let Ok(s) = cs.get_settings().await else {
                warn!("Failed to get settings for connection {c}");
                continue;
            };

            let wifi = s.get("802-11-wireless");

            if wifi.is_some() {
                let ssid = s
                    .get("connection")
                    .and_then(|c| c.get("id"))
                    .map(|s| match s.deref() {
                        Value::Str(v) => v.to_string(),
                        _ => "".to_string(),
                    });

                if let Some(cur_ssid) = ssid {
                    known_ssid.push(cur_ssid);
                }
            } else if s.contains_key("vpn") {
                let id = s
                    .get("connection")
                    .and_then(|c| c.get("id"))
                    .map(|v| match v.deref() {
                        Value::Str(v) => v.to_string(),
                        _ => "".to_string(),
                    });

                if let Some(id) = id {
                    known_vpn.push(Vpn { name: id, path: c });
                }
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
                .path(&d)?
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

    pub async fn wireless_access_points(&self) -> anyhow::Result<Vec<AccessPoint>> {
        let wireless_devices = self.wireless_devices().await?;
        let wireless_access_point_futures: Vec<_> = wireless_devices
            .into_iter()
            .map(|path| async move {
                let device = DeviceProxy::builder(self.0.inner().connection())
                    .path(&path)?
                    .build()
                    .await?;
                let wireless_device = WirelessDeviceProxy::builder(self.0.inner().connection())
                    .path(&path)?
                    .build()
                    .await?;
                wireless_device.request_scan(HashMap::new()).await?;
                let mut scan_changed = wireless_device.receive_last_scan_changed().await;
                if let Some(t) = scan_changed.next().await {
                    if let Ok(-1) = t.get().await {
                        return Ok(Default::default());
                    }
                }
                let access_points = wireless_device.get_access_points().await?;
                let state: DeviceState = device
                    .cached_state()
                    .unwrap_or_default()
                    .map(DeviceState::from)
                    .unwrap_or_else(|| DeviceState::Unknown);

                // Sort by strength and remove duplicates
                let mut aps = HashMap::<String, AccessPoint>::new();
                for ap in access_points {
                    let ap = AccessPointProxy::builder(self.0.inner().connection())
                        .path(ap)?
                        .build()
                        .await?;

                    let ssid = String::from_utf8_lossy(&ap.ssid().await?.clone()).into_owned();
                    let public = ap.flags().await.unwrap_or_default() == 0;
                    let strength = ap.strength().await?;
                    if let Some(access_point) = aps.get(&ssid) {
                        if access_point.strength > strength {
                            continue;
                        }
                    }

                    aps.insert(
                        ssid.clone(),
                        AccessPoint {
                            ssid,
                            strength,
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
            let mut access_points: anyhow::Result<Vec<AccessPoint>> = f.await;
            if let Ok(access_points) = &mut access_points {
                wireless_access_points.append(access_points);
            }
        }

        wireless_access_points.sort_by(|a, b| b.strength.cmp(&a.strength));

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

    pub async fn find_connection(&self, name: &str) -> anyhow::Result<Option<OwnedObjectPath>> {
        let connections = self.list_connections().await?;

        for connection in connections {
            let connection = ConnectionSettingsProxy::builder(self.inner().connection())
                .path(connection)?
                .build()
                .await?;

            let s = connection.get_settings().await?;
            let id = s
                .get("connection")
                .unwrap()
                .get("id")
                .map(|v| match v.deref() {
                    Value::Str(v) => v.to_string(),
                    _ => "".to_string(),
                })
                .unwrap();
            if id == name {
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
pub enum ActiveConnectionState {
    #[default]
    Unknown,
    Activating,
    Activated,
    Deactivating,
    Deactivated,
}

impl From<u32> for ActiveConnectionState {
    fn from(device_state: u32) -> Self {
        match device_state {
            1 => ActiveConnectionState::Activating,
            2 => ActiveConnectionState::Activated,
            3 => ActiveConnectionState::Deactivating,
            4 => ActiveConnectionState::Deactivated,
            _ => ActiveConnectionState::Unknown,
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

    fn add_and_activate_connection(
        &self,
        connection: HashMap<&str, HashMap<&str, Value<'_>>>,
        device: &ObjectPath<'_>,
        specific_object: &ObjectPath<'_>,
    ) -> Result<(OwnedObjectPath, OwnedObjectPath)>;

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
    interface = "org.freedesktop.NetworkManager.Device.Wired",
    default_service = "org.freedesktop.NetworkManager"
)]
trait WiredDevice {
    /// Carrier property
    #[zbus(property)]
    fn carrier(&self) -> zbus::Result<bool>;

    /// HwAddress property
    #[zbus(property)]
    fn hw_address(&self) -> zbus::Result<String>;

    /// PermHwAddress property
    #[zbus(property)]
    fn perm_hw_address(&self) -> zbus::Result<String>;

    /// S390Subchannels property
    #[zbus(property)]
    fn s390subchannels(&self) -> zbus::Result<Vec<String>>;

    /// Speed property
    #[zbus(property)]
    fn speed(&self) -> zbus::Result<u32>;
}

#[proxy(
    default_service = "org.freedesktop.NetworkManager",
    default_path = "/org/freedesktop/NetworkManager/Device/Wireless",
    interface = "org.freedesktop.NetworkManager.Device.Wireless"
)]
pub trait WirelessDevice {
    /// GetAccessPoints method
    fn get_access_points(&self) -> zbus::Result<Vec<zbus::zvariant::OwnedObjectPath>>;

    #[zbus(property)]
    fn active_access_point(&self) -> Result<OwnedObjectPath>;

    #[zbus(property)]
    fn access_points(&self) -> Result<Vec<OwnedObjectPath>>;

    #[zbus(property)]
    fn last_scan(&self) -> zbus::Result<i64>;

    fn request_scan(&self, options: HashMap<String, OwnedValue>) -> Result<()>;
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
    fn strength(&self) -> Result<u8>;

    #[zbus(property)]
    fn flags(&self) -> Result<u32>;
}

#[proxy(
    default_service = "org.freedesktop.NetworkManager",
    default_path = "/org/freedesktop/NetworkManager/Settings",
    interface = "org.freedesktop.NetworkManager.Settings"
)]
pub trait Settings {
    fn add_connection(
        &self,
        connection: HashMap<String, HashMap<String, OwnedValue>>,
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
    fn update(&self, settings: HashMap<String, HashMap<String, OwnedValue>>) -> Result<()>;

    fn get_settings(&self) -> Result<HashMap<String, HashMap<String, OwnedValue>>>;
}
