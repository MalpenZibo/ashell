use super::{AccessPoint, ActiveConnectionInfo, KnownConnection};
use iced::futures::StreamExt;
use itertools::Itertools;
use log::debug;
use std::{collections::HashMap, ops::Deref};
use zbus::{
    proxy,
    zvariant::{self, ObjectPath, OwnedObjectPath, OwnedValue, Value},
    Result,
};

pub struct NetworkDbus<'a>(NetworkManagerProxy<'a>);

impl<'a> Deref for NetworkDbus<'a> {
    type Target = NetworkManagerProxy<'a>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a> NetworkDbus<'a> {
    pub async fn new(conn: &zbus::Connection) -> anyhow::Result<Self> {
        let nm = NetworkManagerProxy::new(&conn).await?;

        Ok(Self(nm))
    }

    pub async fn connectivity(&self) -> Result<ConnectivityState> {
        self.0.connectivity().await.map(ConnectivityState::from)
    }

    pub async fn active_connections(&self) -> anyhow::Result<Vec<ActiveConnectionInfo>> {
        let active_connections = self.0.active_connections().await?;
        let mut ac_proxies: Vec<ActiveConnectionProxy> =
            Vec::with_capacity(active_connections.len());
        for active_connection in &active_connections {
            let active_connection = ActiveConnectionProxy::builder(self.0.inner().connection())
                .path(active_connection)?
                .build()
                .await?;
            ac_proxies.push(active_connection.into());
        }

        let mut info = Vec::<ActiveConnectionInfo>::with_capacity(active_connections.len());
        for connection in ac_proxies {
            let state = connection
                .state()
                .await
                .map(ActiveConnectionState::from)
                .unwrap_or(ActiveConnectionState::Unknown);

            if connection.vpn().await.unwrap_or_default() {
                info.push(ActiveConnectionInfo::Vpn {
                    name: connection.id().await?,
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
                                name: String::from_utf8_lossy(&access_point.ssid().await?)
                                    .into_owned(),
                                state,
                                strength: access_point.strength().await.unwrap_or_default(),
                            });
                        }
                    }
                    Some(DeviceType::WireGuard) => {
                        info.push(ActiveConnectionInfo::Vpn {
                            name: connection.id().await?,
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

    pub async fn known_connections(
        &self,
        wireless_access_points: &[AccessPoint],
        active_connections: &[ActiveConnectionInfo],
    ) -> anyhow::Result<Vec<KnownConnection>> {
        let settings = NetworkSettingsDbus::new(self.0.inner().connection()).await?;

        let known_connections = settings.know_connections().await?;

        let mut known_ssid = Vec::with_capacity(known_connections.len());
        let mut known_vpn = Vec::new();
        for c in known_connections {
            let c = ConnectionSettingsProxy::builder(self.0.inner().connection())
                .path(c)?
                .build()
                .await?;
            let s = c.get_settings().await.unwrap();
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
                    known_vpn.push(id);
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

    pub async fn wireless_access_points(&self) -> anyhow::Result<Vec<AccessPoint>> {
        let devices = self.devices().await.ok().unwrap_or_default();
        let wireless_access_point_futures: Vec<_> = devices
            .into_iter()
            .map(|device| async move {
                let device = DeviceProxy::builder(self.0.inner().connection())
                    .path(device)?
                    .build()
                    .await?;

                if let Ok(DeviceType::Wifi) = device.device_type().await.map(DeviceType::from) {
                    let wireless_device = WirelessDeviceProxy::builder(self.0.inner().connection())
                        .path(device.0.path())?
                        .build()
                        .await?;
                    wireless_device.request_scan(HashMap::new()).await?;
                    let mut scan_changed = wireless_device.receive_last_scan_changed().await;
                    if let Some(t) = scan_changed.next().await {
                        if let Ok(-1) = t.get().await {
                            eprintln!("scan errored");
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
                                path: ap.inner().path().to_owned(),
                                device_path: device.0.path().to_owned(),
                            },
                        );
                    }

                    let aps = aps
                        .into_values()
                        .sorted_by(|a, b| b.strength.cmp(&a.strength))
                        .collect();

                    Ok(aps)
                } else {
                    Ok(Vec::new())
                }
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

    pub async fn select_access_point(&self, access_point: &AccessPoint) -> anyhow::Result<()> {
        let settings = NetworkSettingsDbus::new(self.0.inner().connection()).await?;
        let connection = settings.find_connection(&access_point.ssid).await?;

        if let Some(connection) = connection {
            self.activate_connection(
                connection,
                access_point.device_path.to_owned().into(),
                OwnedObjectPath::try_from("/")?,
            )
            .await?;
        } else {
            let name = access_point.ssid.clone();
            debug!("Create new wifi connection: {}", name);

            let conn_settings: HashMap<&str, HashMap<&str, zvariant::Value>> = HashMap::from([
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

            // if let Some(pass) = password {
            //     conn_settings.insert(
            //         "802-11-wireless-security",
            //         HashMap::from([
            //             ("psk", Value::Str(pass.into())),
            //             ("key-mgmt", Value::Str("wpa-psk".into())),
            //         ]),
            //     );
            // }
            //
            // Combine these all in a single configuration.
            //
            self.add_and_activate_connection(
                conn_settings,
                &access_point.device_path,
                &access_point.path,
            )
            .await?;
        }

        Ok(())
    }
}

pub struct NetworkSettingsDbus<'a>(SettingsProxy<'a>);

impl<'a> Deref for NetworkSettingsDbus<'a> {
    type Target = SettingsProxy<'a>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a> NetworkSettingsDbus<'a> {
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
trait NetworkManager {
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
trait Device {
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
trait WirelessDevice {
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
trait AccessPoint {
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
trait Settings {
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
