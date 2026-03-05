use super::{AccessPoint, ActiveConnectionInfo, KnownConnection, Vpn};
use futures::StreamExt;
use itertools::Itertools;
use log::{debug, warn};
use std::{collections::HashMap, ops::Deref};
use tokio::process::Command;
use zbus::{
    Result, proxy,
    zvariant::{self, ObjectPath, OwnedObjectPath, OwnedValue, Value},
};

pub struct NetworkDbus<'a>(NetworkManagerProxy<'a>);

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
                device.device_type().await.map(NmDeviceType::from),
                Ok(NmDeviceType::Wifi)
            ) {
                return Ok(true);
            }
        }
        Ok(false)
    }

    pub async fn active_connections_info(&self) -> anyhow::Result<Vec<ActiveConnectionInfo>> {
        let active_connections = self.0.active_connections().await?;
        let mut info = Vec::with_capacity(active_connections.len());

        for ac_path in &active_connections {
            let ac = ActiveConnectionProxy::builder(self.0.inner().connection())
                .path(ac_path)?
                .build()
                .await?;

            if ac.vpn().await.unwrap_or_default() {
                info.push(ActiveConnectionInfo::Vpn {
                    name: ac.id().await?,
                    object_path: ac.inner().path().to_owned().into(),
                });
                continue;
            }

            for device in ac.devices().await.unwrap_or_default() {
                let dp = DeviceProxy::builder(self.0.inner().connection())
                    .path(device)?
                    .build()
                    .await?;
                match dp.device_type().await.map(NmDeviceType::from).ok() {
                    Some(NmDeviceType::Ethernet) => {
                        info.push(ActiveConnectionInfo::Wired {
                            name: ac.id().await?,
                        });
                    }
                    Some(NmDeviceType::Wifi) => {
                        let wd = WirelessDeviceProxy::builder(self.0.inner().connection())
                            .path(dp.0.path())?
                            .build()
                            .await?;
                        if let Ok(ap_path) = wd.active_access_point().await {
                            let ap = AccessPointProxy::builder(self.0.inner().connection())
                                .path(ap_path)?
                                .build()
                                .await?;
                            info.push(ActiveConnectionInfo::WiFi {
                                name: String::from_utf8_lossy(&ap.ssid().await?)
                                    .trim()
                                    .to_string(),
                                strength: ap.strength().await.unwrap_or_default(),
                            });
                        }
                    }
                    Some(NmDeviceType::WireGuard) => {
                        info.push(ActiveConnectionInfo::Vpn {
                            name: ac.id().await?,
                            object_path: ac.inner().path().to_owned().into(),
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

    pub async fn wireless_devices(&self) -> anyhow::Result<Vec<OwnedObjectPath>> {
        let devices = self.devices().await?;
        let mut wireless_devices = Vec::new();
        for d in devices {
            let dp = DeviceProxy::builder(self.0.inner().connection())
                .path(&d)?
                .build()
                .await?;
            if matches!(
                dp.device_type().await.map(NmDeviceType::from),
                Ok(NmDeviceType::Wifi)
            ) {
                wireless_devices.push(d);
            }
        }
        Ok(wireless_devices)
    }

    pub async fn wireless_access_points(&self) -> anyhow::Result<Vec<AccessPoint>> {
        let wireless_devices = self.wireless_devices().await?;
        let mut all_aps = Vec::new();

        for path in wireless_devices {
            let dp = DeviceProxy::builder(self.0.inner().connection())
                .path(&path)?
                .build()
                .await?;
            let wd = WirelessDeviceProxy::builder(self.0.inner().connection())
                .path(&path)?
                .build()
                .await?;
            let access_points = wd.get_access_points().await?;
            let state: DeviceState = dp
                .cached_state()
                .unwrap_or_default()
                .map_or_else(|| DeviceState::Unknown, DeviceState::from);

            let mut aps = HashMap::<String, AccessPoint>::new();
            for ap_path in access_points {
                let ap = AccessPointProxy::builder(self.0.inner().connection())
                    .path(ap_path)?
                    .build()
                    .await?;
                let ssid = String::from_utf8_lossy(&ap.ssid().await?).trim().to_string();
                let public = ap.flags().await.unwrap_or_default() == 0;
                let strength = ap.strength().await?;

                if let Some(existing) = aps.get(&ssid) {
                    if existing.strength > strength {
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
                        device_path: dp.0.path().clone().into(),
                    },
                );
            }

            let mut sorted: Vec<_> = aps.into_values().sorted_by(|a, b| b.strength.cmp(&a.strength)).collect();
            all_aps.append(&mut sorted);
        }

        all_aps.sort_by(|a, b| b.strength.cmp(&a.strength));
        Ok(all_aps)
    }

    pub async fn known_connections(&self) -> anyhow::Result<Vec<KnownConnection>> {
        let wireless_access_points = self.wireless_access_points().await?;
        self.known_connections_internal(&wireless_access_points).await
    }

    async fn known_connections_internal(
        &self,
        wireless_access_points: &[AccessPoint],
    ) -> anyhow::Result<Vec<KnownConnection>> {
        let settings = NetworkSettingsDbus::new(self.0.inner().connection()).await?;
        let connections = settings.list_connections().await?;

        let mut known_ssid = Vec::with_capacity(connections.len());
        let mut known_vpn = Vec::new();

        for c in connections {
            let cs = ConnectionSettingsProxy::builder(self.0.inner().connection())
                .path(c.clone())?
                .build()
                .await?;
            let Ok(s) = cs.get_settings().await else {
                warn!("Failed to get settings for connection {c}");
                continue;
            };

            if s.get("802-11-wireless").is_some() {
                let ssid = s
                    .get("connection")
                    .and_then(|c| c.get("id"))
                    .map(|s| match s.deref() {
                        Value::Str(v) => v.to_string(),
                        _ => String::new(),
                    });
                if let Some(ssid) = ssid {
                    known_ssid.push(ssid);
                }
            } else if s.contains_key("vpn") || s.contains_key("wireguard") {
                let id = s
                    .get("connection")
                    .and_then(|c| c.get("id"))
                    .map(|v| match v.deref() {
                        Value::Str(v) => v.to_string(),
                        _ => String::new(),
                    });
                if let Some(id) = id {
                    known_vpn.push(Vpn { name: id, path: c });
                }
            }
        }

        let known: Vec<_> = wireless_access_points
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

        Ok(known)
    }

    pub async fn set_wifi_enabled(&self, enable: bool) -> anyhow::Result<()> {
        self.set_wireless_enabled(enable).await?;
        Ok(())
    }

    pub async fn set_airplane_mode(&self, enable: bool) -> anyhow::Result<()> {
        let _ = Command::new("rfkill")
            .arg(if enable { "block" } else { "unblock" })
            .arg("bluetooth")
            .output()
            .await;
        self.set_wireless_enabled(!enable).await?;
        Ok(())
    }

    pub async fn scan_nearby_wifi(&self) -> anyhow::Result<()> {
        let aps = self.wireless_access_points().await?;
        for ap in aps {
            let device = WirelessDeviceProxy::builder(self.0.inner().connection())
                .path(ap.device_path)?
                .build()
                .await?;
            device.request_scan(HashMap::new()).await?;
        }
        Ok(())
    }

    pub async fn select_access_point(
        &self,
        ap: &AccessPoint,
        password: Option<String>,
    ) -> anyhow::Result<()> {
        let settings = NetworkSettingsDbus::new(self.0.inner().connection()).await?;
        let connection = settings.find_connection(&ap.ssid).await?;

        if let Some(conn_path) = connection {
            if let Some(password) = password {
                let cs = ConnectionSettingsProxy::builder(self.0.inner().connection())
                    .path(&conn_path)?
                    .build()
                    .await?;
                let mut s = cs.get_settings().await?;
                if let Some(wifi_settings) = s.get_mut("802-11-wireless-security") {
                    let new_password = zvariant::Value::from(password).try_to_owned()?;
                    wifi_settings.insert("psk".to_string(), new_password);
                }
                cs.update(s).await?;
            }
            self.activate_connection(
                conn_path,
                ap.device_path.to_owned(),
                OwnedObjectPath::try_from("/")?,
            )
            .await?;
        } else {
            let name = ap.ssid.clone();
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
                &ap.device_path,
                &ap.path,
            )
            .await?;
        }
        Ok(())
    }

    pub async fn set_vpn(
        &self,
        connection: OwnedObjectPath,
        enable: bool,
    ) -> anyhow::Result<()> {
        if enable {
            self.activate_connection(
                connection,
                OwnedObjectPath::try_from("/").unwrap(),
                OwnedObjectPath::try_from("/").unwrap(),
            )
            .await?;
        } else {
            self.deactivate_connection(connection).await?;
        }
        Ok(())
    }
}

struct NetworkSettingsDbus<'a>(SettingsProxy<'a>);

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

    pub async fn find_connection(&self, name: &str) -> anyhow::Result<Option<OwnedObjectPath>> {
        let connections = self.list_connections().await?;
        for connection in connections {
            let cs = ConnectionSettingsProxy::builder(self.inner().connection())
                .path(connection)?
                .build()
                .await?;
            let s = cs.get_settings().await?;
            let id = s["connection"]
                .get("id")
                .map(|v| match v.deref() {
                    Value::Str(v) => v.to_string(),
                    _ => String::new(),
                })
                .unwrap();
            if id == name {
                return Ok(Some(cs.inner().path().to_owned().into()));
            }
        }
        Ok(None)
    }
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum NmDeviceType {
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

impl From<u32> for NmDeviceType {
    fn from(dt: u32) -> NmDeviceType {
        match dt {
            1 => NmDeviceType::Ethernet,
            2 => NmDeviceType::Wifi,
            5 => NmDeviceType::Bluetooth,
            14 => NmDeviceType::Generic,
            16 => NmDeviceType::TunTap,
            29 => NmDeviceType::WireGuard,
            3..=32 => NmDeviceType::Other,
            _ => NmDeviceType::Unknown,
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
    fn from(ds: u32) -> Self {
        match ds {
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

// --- D-Bus proxies ---

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
    fn state(&self) -> Result<u32>;
}

#[proxy(
    default_service = "org.freedesktop.NetworkManager",
    default_path = "/org/freedesktop/NetworkManager/Device/Wireless",
    interface = "org.freedesktop.NetworkManager.Device.Wireless"
)]
pub trait WirelessDevice {
    fn get_access_points(&self) -> zbus::Result<Vec<OwnedObjectPath>>;

    #[zbus(property)]
    fn active_access_point(&self) -> Result<OwnedObjectPath>;

    #[zbus(property)]
    fn access_points(&self) -> Result<Vec<OwnedObjectPath>>;

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
    #[zbus(property)]
    fn connections(&self) -> Result<Vec<OwnedObjectPath>>;

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
