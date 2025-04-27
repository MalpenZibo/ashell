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

// source for dbus: https://git.kernel.org/pub/scm/network/wireless/iwd.git/tree/doc

use super::{AccessPoint, ActiveConnectionInfo, KnownConnection};
use access_point::AccessPointProxy;
use async_trait::async_trait;
use device::DeviceProxy;
use iced::futures::{Stream, StreamExt};
use itertools::Itertools;
use known_network::KnownNetworkProxy;
use log::debug;
use network::NetworkProxy;
use station::StationProxy;
use std::{collections::HashMap, ops::Deref};
use zbus::fdo::{ObjectManagerProxy, PropertiesProxy};
use zbus::zvariant::{OwnedObjectPath, Value};
use zbus::{Result, proxy};

/// Wrapper around the IWD D-Bus ObjectManager
pub struct IwdDbus<'a>(ObjectManagerProxy<'a>);

impl<'a> Deref for IwdDbus<'a> {
    type Target = ObjectManagerProxy<'a>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[async_trait]
impl super::NetworkBackend for IwdDbus<'_> {
    /// Checks if the dbus server is running.
    async fn is_available(&self) -> anyhow::Result<bool> {unimplemented!()}

    /// Initializes the backend and fetches the initial network data.
    async fn initialize_data(&self) -> anyhow::Result<super::NetworkData> {unimplemented!()}

    /// Subscribes to network events from the backend.
    /// Returns a stream of `NetworkEvent`s.
    async fn subscribe_events(&self) -> anyhow::Result<Box<dyn Stream<Item = super::NetworkEvent>>> {unimplemented!()}

    /// Toggles the airplane mode.
    async fn set_airplane_mode(&self, enable: bool) -> anyhow::Result<()> {unimplemented!()}

    /// Scans for nearby Wi-Fi networks.
    async fn scan_nearby_wifi(&self) -> anyhow::Result<()> {unimplemented!()}

    /// Enables or disables Wi-Fi.
    async fn set_wifi_enabled(&self, enable: bool) -> anyhow::Result<()>{unimplemented!()}

    /// Connects to a specific access point, potentially with a password.
    /// Returns the updated list of known connections.
    async fn select_access_point(
        &self,
        ap: &AccessPoint,
        password: Option<String>,
    ) -> anyhow::Result<Vec<KnownConnection>>{unimplemented!()}

    /// Enables or disables a VPN connection.
    /// Returns the updated list of known connections.
    async fn set_vpn(
        &self,
        connection_path: OwnedObjectPath,
        enable: bool,
    ) -> anyhow::Result<Vec<KnownConnection>>{unimplemented!()}
}

impl IwdDbus<'_> {
    /// Connect to the system bus and the IWD service
    pub async fn new(conn: &zbus::Connection) -> anyhow::Result<Self> {
        let manager = ObjectManagerProxy::builder(conn)
            .destination("net.connman.iwd")?
            .path("/")?
            .build()
            .await?;
        Ok(Self(manager))
    }

    /// Get the state of all station interfaces
    pub async fn connectivity(&self) -> Result<Vec<String>> {
        let objects = self.0.get_managed_objects().await?;
        let mut states = Vec::new();
        for (path, ifs) in objects {
            if ifs.contains_key("net.connman.iwd.Station") {
                let station = StationProxy::builder(self.0.inner().connection())
                    .destination("net.connman.iwd")?
                    .path(path)?
                    .build()
                    .await?;
                states.push(station.state().await?);
            }
        }
        Ok(states)
    }

    /// Return true if any device in station mode is present
    pub async fn wifi_device_present(&self) -> anyhow::Result<bool> {
        let devices = self.wireless_devices().await?;

        for path in devices {
            let device = DeviceProxy::builder(self.0.inner().connection())
                .destination("net.connman.iwd")?
                .path(path)?
                .build()
                .await?;
            if device.mode().await? == "station" && device.powered().await? {
                return Ok(true);
            }
        }
        Ok(false)
    }

    pub async fn devices(&self) -> anyhow::Result<Vec<OwnedObjectPath>> {
        let objects = self.0.get_managed_objects().await?;
        let mut devs = Vec::new();
        for (path, ifs) in objects {
            if ifs.contains_key("net.connman.iwd.Device") {
                devs.push(path.clone().into());
            }
        }
        Ok(devs)
    }

    /// List all networks currently connected (Connected = true)
    pub async fn active_connections(&self) -> anyhow::Result<Vec<OwnedObjectPath>> {
        let mut result = Vec::new();
        let objects = self.0.get_managed_objects().await?;
        for (path, ifs) in objects {
            if let Some(props) = ifs.get("net.connman.iwd.Network") {
                if let Some(v) = props.get("Connected") {
                    if let Ok(true) = v.downcast_ref::<bool>() {
                        result.push(path.clone().into());
                    }
                }
            }
        }
        Ok(result)
    }

    /// Detailed info on active connections
    pub async fn active_connections_info(&self) -> anyhow::Result<Vec<ActiveConnectionInfo>> {
        let nets = self.active_connections().await?;
        let mut info = Vec::new();
        for net_path in nets {
            let net = NetworkProxy::builder(self.0.inner().connection())
                .destination("net.connman.iwd")?
                .path(net_path.clone())?
                .build()
                .await?;
            let ssid = net.name().await?;
            // strength not directly on Network; placeholder 0
            info.push(ActiveConnectionInfo::WiFi {
                id: ssid.clone(),
                name: ssid,
                strength: 0,
            });
        }
        Ok(info)
    }

    /// List known (provisioned) SSIDs
    pub async fn known_connections(
        &self,
        wireless_access_points: &[AccessPoint],
    ) -> anyhow::Result<Vec<KnownConnection>> {
        let objects = self.0.get_managed_objects().await?;
        let mut known_ssid = Vec::new();
        for (path, ifs) in &objects {
            if ifs.contains_key("net.connman.iwd.KnownNetwork") {
                let kn = KnownNetworkProxy::builder(self.0.inner().connection())
                    .destination("net.connman.iwd")?
                    .path(path.clone())?
                    .build()
                    .await?;
                known_ssid.push(kn.name().await?);
            }
        }
        let known: Vec<_> = wireless_access_points
            .iter()
            .filter(|a| known_ssid.contains(&a.ssid))
            .cloned()
            .map(KnownConnection::AccessPoint)
            .collect();
        Ok(known)
    }

    /// List all wireless (station-mode) devices
    pub async fn wireless_devices(&self) -> anyhow::Result<Vec<OwnedObjectPath>> {
        let devices = self.devices().await?;
        let mut devs = Vec::new();
        for path in devices {
            let device = DeviceProxy::builder(self.0.inner().connection())
                .destination("net.connman.iwd")?
                .path(path.clone())?
                .build()
                .await?;
            if device.mode().await? == "station" {
                devs.push(path.clone().into());
            }
        }
        Ok(devs)
    }

    /// Scan and list available access points
    pub async fn wireless_access_points(&self) -> anyhow::Result<Vec<AccessPoint>> {
        let devs = self.wireless_devices().await?;
        let mut aps = Vec::new();
        for d in devs {
            let station = StationProxy::builder(self.0.inner().connection())
                .destination("net.connman.iwd")?
                .path(d.clone())?
                .build()
                .await?;
            station.scan().await?;
            let paths = station.get_ordered_networks().await?;
            for (p, _i) in paths {
                let ap = AccessPointProxy::builder(self.0.inner().connection())
                    .destination("net.connman.iwd")?
                    .path(p.clone())?
                    .build()
                    .await?;
                aps.push(AccessPoint {
                    ssid: ap.name().await?,
                    strength: ap.strength().await?,
                    public: ap.flags().await.unwrap_or_default() == 0,
                    working: false,
                    path: p.into_inner(),
                    device_path: d.clone().into_inner(),
                });
            }
        }
        aps.sort_by(|a, b| b.strength.cmp(&a.strength));
        Ok(aps)
    }

    pub async fn wireless_enabled(&self) -> anyhow::Result<bool> {
        let devs = self.wireless_devices().await?;
        for d in devs {
            let device = DeviceProxy::builder(self.0.inner().connection())
                .destination("net.connman.iwd")?
                .path(d.clone())?
                .build()
                .await?;
            if device.powered().await? {
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// Activate or create-and-activate an access point
    pub async fn select_access_point(
        &self,
        access_point: &AccessPoint,
        password: Option<String>,
    ) -> anyhow::Result<()> {
        // IWD auto-saves credentials; simply call Connect on the Network object
        let net_path = access_point.path.clone();
        let net = NetworkProxy::builder(self.0.inner().connection())
            .destination("net.connman.iwd")?
            .path(net_path)?
            .build()
            .await?;
        // TODO:
        //if let Some(p) = password {
        //    net.set_passphrase(p).await.ok();
        //}
        net.connect().await?;
        Ok(())
    }
}
