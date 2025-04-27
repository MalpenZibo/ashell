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

use crate::services::bluetooth::BluetoothService;

use super::dbus::DeviceState;
use super::{AccessPoint, ActiveConnectionInfo, KnownConnection, NetworkEvent};
use access_point::AccessPointProxy;
use adapter::AdapterProxy;
use device::DeviceProxy;
use iced::futures::future::{join_all, select};
use iced::futures::stream::select_all;
use iced::futures::{Stream, StreamExt};
use itertools::Itertools;
use known_network::KnownNetworkProxy;
use log::debug;
use log::info;
use network::NetworkProxy;
use station::StationProxy;
use tokio::process::Command;
use std::{collections::HashMap, ops::Deref};
use zbus::fdo::{ObjectManagerProxy, PropertiesProxy};
use zbus::zvariant::{ObjectPath, OwnedObjectPath, Value};
use zbus::{Result, proxy};

/// Wrapper around the IWD D-Bus ObjectManager
pub struct IwdDbus<'a>(ObjectManagerProxy<'a>);

impl<'a> Deref for IwdDbus<'a> {
    type Target = ObjectManagerProxy<'a>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl super::NetworkBackend for IwdDbus<'_> {
    async fn is_available(&self) -> anyhow::Result<bool> {
        todo!()
    }

    async fn initialize_data(&self) -> anyhow::Result<super::NetworkData> {
        let nm = self;

        // airplane mode
        info!("checking bluetooth");
        let bluetooth_soft_blocked = BluetoothService::check_rfkill_soft_block()
            .await
            .unwrap_or_default();

        info!("wifi present");
        let wifi_present = nm.wifi_device_present().await?;

        info!("wifi enabled");
        let wifi_enabled = nm.wireless_enabled().await.unwrap_or_default();
        debug!("Wifi enabled: {}", wifi_enabled);

        info!("airplane enabled");
        let airplane_mode = bluetooth_soft_blocked && !wifi_enabled;
        debug!("Airplane mode: {}", airplane_mode);

        info!("ac enabled");
        let active_connections = nm.active_connections_info().await?;
        debug!("Active connections: {:?}", active_connections);

        info!("wac enabled");
        let wireless_access_points = nm.wireless_access_points().await?;
        debug!("Wireless access points: {:?}", wireless_access_points);

        info!("kc enabled");
        let known_connections = nm.known_connections().await?;
        debug!("Known connections: {:?}", known_connections);

        info!("sc enabled");
        let is_scanning = join_all(self.stations().await?.iter().map(|s| s.scanning())).await.into_iter().filter_map(|v| v.ok()).any(|v| v);

        info!("connect enabled");
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
    async fn known_connections(
        &self,
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
        // TODO: not acccess points, but ordered networks
        //let known: Vec<_> = wireless_access_points
        //    .iter()
        //    .filter(|a| known_ssid.contains(&a.ssid))
        //    .cloned()
        //    .map(KnownConnection::AccessPoint)
        //    .collect();
        Ok(vec![])
    }


    async fn scan_nearby_wifi(
        &self,
    ) -> anyhow::Result<()> {
        StationProxy::new(self.0.inner().connection()).await?.scan();
        Ok(())
    }

    async fn set_wifi_enabled(
        &self
, enabled: bool) -> anyhow::Result<()> {
        AdapterProxy::new(self.0.inner().connection())
            .await?
            .set_powered(enabled)
            .await?;
        Ok(())
    }

    async fn select_access_point(
        &self,
        ap: &AccessPoint,
        password: Option<String>,
    ) -> anyhow::Result<()> {
        todo!();
        Ok(())
    }

    async fn set_vpn(
        &self,
        path: OwnedObjectPath,
        enable: bool,
    ) -> anyhow::Result<Vec<KnownConnection>> {
        todo!()
    }

    async fn set_airplane_mode(
        &self
, airplane: bool) -> anyhow::Result<()> {
        Command::new("/usr/sbin/rfkill")
            .arg(if airplane { "block" } else { "unblock" })
            .arg("bluetooth")
            .output()
            .await?;
        self.set_wifi_enabled(!airplane).await?;
        Ok(())
    }

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

    pub async fn stations(&self) -> anyhow::Result<Vec<StationProxy>> {
        let objects = self.0.get_managed_objects().await?;
        let mut stations = Vec::new();
        for (path, ifs) in objects {
            if ifs.contains_key("net.connman.iwd.Station") {
                stations.push(StationProxy::builder(self.0.inner().connection())
                    .destination("net.connman.iwd")?
                    .path(path.clone())?
                    .build()
                    .await?);
            }
        }
        Ok(stations)
    }

    pub async fn subscribe_events(
        &self
    ) -> anyhow::Result<impl Stream<Item = NetworkEvent>> {
        let conn = self.0.inner().connection();
        let nm = IwdDbus::new(conn).await?;
        let adapter = AdapterProxy::new(conn).await?;
        let station = StationProxy::new(conn).await?;

        info!("subscribing to events - power");
        let wireless_enabled = adapter
            .receive_powered_changed()
            .await
            .then(|v| async move {
                let value = v.get().await.unwrap_or_default();

                debug!("WiFi enabled changed: {}", value);
                NetworkEvent::WiFiEnabled(value)
            })
            .boxed();

        info!("subscribing to events - connnectivity");
        let connectivity_changed = StationProxy::new(conn)
            .await?
            .receive_state_changed()
            .await
            .then(|val| async move {
                let value = val.get().await.unwrap_or_default().into();

                debug!("Connectivity changed: {:?}", value);
                NetworkEvent::Connectivity(value)
            })
            .boxed();

        info!("subscribing to events - active connections");
        let active_connections_changes = station
            .receive_connected_network_changed()
            .await
            .then({
                let conn = conn.clone();
                move |_| {
                    let conn = conn.clone();
                    async move {
                        let nm = IwdDbus::new(&conn).await.unwrap();
                        let value = nm.active_connections_info().await.unwrap_or_default();

                        debug!("Active connections changed: {:?}", value);
                        NetworkEvent::ActiveConnections(value)
                    }
                }
            })
            .boxed();

        info!("subscribing to events - wireless devices");
        let devices = nm.wireless_devices().await.unwrap_or_default();

        // TODO: can only do this by watching ALL interfaces on iwd
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

        // TODO: When devices list change I need to update the wireless device state changes
        info!("subscribing to events - wireless acs");
        let wireless_ac = nm.wireless_access_points().await?;

        //let mut device_state_changes = Vec::with_capacity(wireless_ac.len());
        info!("subscribing to events - devices");
        for ac in wireless_ac.iter() {
            let dp = DeviceProxy::builder(conn)
                .path(ac.device_path.clone())?
                .build()
                .await?;

            //device_state_changes.push(
            //    dp.rereceive_state_changed()
            //        .await
            //        .filter_map(|val| async move {
            //            let val = val.get().await;
            //            let val = val.map(DeviceState::from).unwrap_or_default();

            //            if val == DeviceState::NeedAuth {
            //                Some(val)
            //            } else {
            //                None
            //            }
            //        })
            //        .map(|_| {
            //            let ssid = ac.ssid.clone();

            //            debug!("Request password for ssid {}", ssid);
            //            NetworkEvent::RequestPasswordForSSID(ssid)
            //        }),
            //);
        }

        // When devices list change I need to update the access points changes
        //let mut ac_changes = Vec::with_capacity(wireless_ac.len());
        for ac in wireless_ac.iter() {
            //let dp = WirelessDeviceProxy::builder(conn)
            //    .path(ac.device_path.clone())?
            //    .build()
            //    .await?;

            //ac_changes.push(
            //    dp.receive_access_points_changed()
            //        .await
            //        .then({
            //            let conn = conn.clone();
            //            move |_| {
            //                let conn = conn.clone();
            //                async move {
            //                    let nm = IwdDbus::new(&conn).await.unwrap();
            //                    let wireless_access_point =
            //                        nm.wireless_access_points().await.unwrap_or_default();
            //                    debug!("access_points_changed {:?}", wireless_access_point);

            //                    NetworkEvent::WirelessAccessPoint(wireless_access_point)
            //                }
            //            }
            //        })
            //        .boxed(),
            //);
        }

        // When devices list change I need to update the wireless strength changes
        //let mut strength_changes = Vec::with_capacity(wireless_ac.len());
        //for ap in wireless_ac {
        //    let ssid = ap.ssid.clone();
        //    let app = AccessPointProxy::builder(conn)
        //        .path(ap.path.clone())?
        //        .build()
        //        .await?;

        //    strength_changes.push(
        //        app.receive_strength_changed()
        //            .await
        //            .then(move |val| {
        //                let ssid = ssid.clone();
        //                async move {
        //                    let value = val.get().await.unwrap_or_default();
        //                    debug!("Strength changed value: {}, {}", &ssid, value);
        //                    NetworkEvent::Strength((ssid.clone(), value))
        //                }
        //            })
        //            .boxed(),
        //    );
        //}
        //let strength_changes = select_all(strength_changes).boxed();

        //let access_points = select_all(ac_changes).boxed();

        //let known_connections = settings
        //    .receive_connections_changed()
        //    .await
        //    .then({
        //        let conn = conn.clone();
        //        move |_| {
        //            let conn = conn.clone();
        //            async move {
        //                let nm = NetworkDbus::new(&conn).await.unwrap();
        //                let wireless_access_points =
        //                    nm.wireless_access_points().await.unwrap_or_default();

        //                let known_connections = nm
        //                    .known_connections(&wireless_access_points)
        //                    .await
        //                    .unwrap_or_default();

        //                debug!("Known connections changed");
        //                NetworkEvent::KnownConnections(known_connections)
        //            }
        //        }
        //    })
        //    .boxed();

        info!("subscribing to events - select_all");
        let events = select_all(vec![
            wireless_enabled,
            //wireless_devices_changed,
            connectivity_changed,
            active_connections_changes,
            //access_points,
            //strength_changes,
            //known_connections,
        ]);

        Ok(events)
    }

    /// Get the state of all station interfaces
    pub async fn connectivity(&self) -> Result<Vec<String>> {
        info!("objects enabled");
        let objects = self.0.get_managed_objects().await?;
        let mut states = Vec::new();
        for (path, ifs) in objects {
            if ifs.contains_key("net.connman.iwd.Station") {
                info!("station enabled");
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
            // FIXME: should this be here? station.scan().await?;
            let paths = station.get_ordered_networks().await?;
            for (p, _s) in paths {
                // _s is between 0 and -10000
                // should be between 0 and 100
                let net = NetworkProxy::builder(self.0.inner().connection())
                    .destination("net.connman.iwd")?
                    .path(p.clone())?
                    .build()
                    .await?;
                aps.push(AccessPoint {
                    ssid: net.name().await?,
                    state: DeviceState::Unknown, // TODO:
                    strength: ((_s / 100) + 100) as u8,
                    public: net.type_().await? == "open" ,
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
