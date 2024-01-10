use crate::{
    components::icons::Icons,
    modules::settings::NetMessage,
    style::{RED, TEXT, YELLOW},
};
use iced::{
    futures::{
        stream::{self},
        FutureExt, SinkExt, StreamExt,
    },
    Color, Subscription,
};
use zbus::{dbus_proxy, zvariant::OwnedObjectPath, Connection, Result};

static WIFI_SIGNAL_ICONS: [Icons; 5] = [
    Icons::Wifi0,
    Icons::Wifi1,
    Icons::Wifi2,
    Icons::Wifi3,
    Icons::Wifi4,
];

#[dbus_proxy(
    interface = "org.freedesktop.NetworkManager",
    default_service = "org.freedesktop.NetworkManager",
    default_path = "/org/freedesktop/NetworkManager"
)]
trait NetworkManager {
    #[dbus_proxy(property)]
    fn devices(&self) -> Result<Vec<OwnedObjectPath>>;

    #[dbus_proxy(property)]
    fn active_connections(&self) -> Result<Vec<OwnedObjectPath>>;
}

#[dbus_proxy(
    default_service = "org.freedesktop.NetworkManager",
    default_path = "/org/freedesktop/NetworkManager/Connection/Active",
    interface = "org.freedesktop.NetworkManager.Connection.Active"
)]
trait ActiveConnection {
    #[dbus_proxy(property)]
    fn id(&self) -> Result<String>;

    #[dbus_proxy(property)]
    fn uuid(&self) -> Result<String>;

    #[dbus_proxy(property, name = "Type")]
    fn connection_type(&self) -> Result<String>;

    #[dbus_proxy(property)]
    fn state(&self) -> Result<u32>;

    #[dbus_proxy(property)]
    fn vpn(&self) -> Result<bool>;

    #[dbus_proxy(property)]
    fn devices(&self) -> Result<Vec<OwnedObjectPath>>;
}

#[dbus_proxy(
    default_service = "org.freedesktop.NetworkManager",
    default_path = "/org/freedesktop/NetworkManager/Device",
    interface = "org.freedesktop.NetworkManager.Device"
)]
trait Device {
    #[dbus_proxy(property)]
    fn device_type(&self) -> Result<u32>;

    #[dbus_proxy(property)]
    fn active_connection(&self) -> Result<OwnedObjectPath>;
}

#[dbus_proxy(
    default_service = "org.freedesktop.NetworkManager",
    default_path = "/org/freedesktop/NetworkManager/Device/Wireless",
    interface = "org.freedesktop.NetworkManager.Device.Wireless"
)]
trait DeviceWireless {
    #[dbus_proxy(property)]
    fn active_access_point(&self) -> Result<OwnedObjectPath>;
}

#[dbus_proxy(
    default_service = "org.freedesktop.NetworkManager",
    default_path = "/org/freedesktop/NetworkManager/AccessPoint",
    interface = "org.freedesktop.NetworkManager.AccessPoint"
)]
trait AccessPoint {
    #[dbus_proxy(property)]
    fn strength(&self) -> Result<u8>;
}

#[derive(Debug, Clone)]
pub struct Wifi {
    connection_ssid: String,
    signal: u8,
}

impl Wifi {
    pub fn get_icon(&self) -> Icons {
        WIFI_SIGNAL_ICONS[f32::floor(self.signal as f32 / 100.) as usize % (4 - 1 + 1) + 1]
    }

    pub fn get_color(&self) -> Color {
        match self.signal {
            0 => RED,
            1 => YELLOW,
            _ => TEXT,
        }
    }
}

pub fn subscription() -> Subscription<NetMessage> {
    iced::Subscription::batch(vec![
        iced::subscription::channel("nm-dbus-wifi-listener", 100, |mut output| async move {
            let conn = Connection::system().await.unwrap();
            let nm = NetworkManagerProxy::new(&conn).await.unwrap();

            struct Proxies<'a> {
                device: DeviceProxy<'a>,
                wireless_device: DeviceWirelessProxy<'a>,
                access_point: AccessPointProxy<'a>,
                connection: ActiveConnectionProxy<'a>,
            }

            let init = || async {
                let mut devices = nm.devices().await.unwrap_or(vec![]);
                let devices = devices.drain(..);

                let wifi_device = stream::iter(devices.into_iter())
                    .filter_map(|d| {
                        let conn = conn.clone();
                        async move {
                            let builder = DeviceProxy::builder(&conn).path(d.to_owned());
                            if let Ok(builder) = builder {
                                let device = builder.build().await;
                                if let Ok(device) = device {
                                    let device_type = device.device_type().await;
                                    if device_type == Ok(2) {
                                        return Some(d);
                                    }
                                }
                            }

                            None
                        }
                    })
                    .collect::<Vec<_>>()
                    .await;
                let wifi_device = wifi_device.first();
                if let Some(wifi_device) = wifi_device {
                    let device = DeviceProxy::builder(&conn)
                        .path(wifi_device.to_owned())
                        .unwrap()
                        .build()
                        .await
                        .unwrap();
                    let wireless_device = DeviceWirelessProxy::builder(&conn)
                        .path(wifi_device.to_owned())
                        .unwrap()
                        .build()
                        .await
                        .unwrap();
                    let access_point = AccessPointProxy::builder(&conn)
                        .path(
                            wireless_device
                                .active_access_point()
                                .await
                                .unwrap()
                                .to_owned(),
                        )
                        .unwrap()
                        .build()
                        .await
                        .unwrap();

                    let connection = ActiveConnectionProxy::builder(&conn)
                        .path(device.active_connection().await.unwrap().to_owned())
                        .unwrap()
                        .build()
                        .await
                        .unwrap();

                    Some(Proxies {
                        device,
                        wireless_device,
                        access_point,
                        connection,
                    })
                } else {
                    None
                }
            };

            let mut maybe_proxies = init().await;

            let mut wifi = if let Some(proxies) = maybe_proxies.as_ref() {
                Some(Wifi {
                    connection_ssid: proxies.connection.id().await.unwrap(),
                    signal: proxies.access_point.strength().await.unwrap(),
                })
            } else {
                None
            };

            let _ = output.send(NetMessage::Wifi(wifi.clone())).await;

            loop {
                let mut devices_change = nm.receive_devices_changed().await;

                if let Some((
                    mut access_point_change,
                    mut connection_change,
                    mut connection_id,
                    mut strength,
                )) = if let Some(ref mut proxies) = maybe_proxies.as_mut() {
                    let access_point_change = proxies
                        .wireless_device
                        .receive_active_access_point_changed()
                        .await;
                    let connection_change =
                        proxies.device.receive_active_connection_changed().await;
                    let connection_id = proxies.connection.receive_id_changed().await;
                    let strength = proxies.access_point.receive_strength_changed().await;

                    Some((
                        access_point_change,
                        connection_change,
                        connection_id,
                        strength,
                    ))
                } else {
                    None
                } {
                    iced::futures::select_biased! {
                        v = devices_change.next().fuse() => {
                            if v.is_some() {
                                maybe_proxies = init().await;

                                wifi = if let Some(proxies) = maybe_proxies.as_ref() {
                                    Some(Wifi {
                                        connection_ssid: proxies.connection.id().await.unwrap(),
                                        signal: proxies.access_point.strength().await.unwrap(),
                                    })
                                } else { None };

                                let _ = output.send(NetMessage::Wifi(wifi.clone())).await;
                            }
                        }
                        v = access_point_change.next().fuse() => {
                            if let Some(value) = v {
                                if let Some(proxies) = maybe_proxies.as_mut() {
                                proxies.access_point = AccessPointProxy::builder(&conn)
                                    .path(value.get().await.unwrap().to_owned())
                                    .unwrap()
                                    .build()
                                    .await
                                    .unwrap();
                                wifi = Some(Wifi {
                                    connection_ssid: proxies.connection.id().await.unwrap(),
                                    signal: proxies.access_point.strength().await.unwrap(),
                                });
                                let _ = output.send(NetMessage::Wifi(wifi.clone())).await;
                                }
                            }
                        },
                        v = connection_change.next().fuse() => {
                            if let Some(value) = v {
                                if let Some(proxies) = maybe_proxies.as_mut() {
                                proxies.connection = ActiveConnectionProxy::builder(&conn)
                                    .path(value.get().await.unwrap().to_owned())
                                    .unwrap()
                                    .build()
                                    .await
                                    .unwrap();
                                wifi = Some(Wifi {
                                    connection_ssid: proxies.connection.id().await.unwrap(),
                                    signal: proxies.access_point.strength().await.unwrap(),
                                });
                                let _ = output.send(NetMessage::Wifi(wifi.clone())).await;
                                }
                            }
                        },
                        v = connection_id.next().fuse() => {
                            if let Some(connection) = v {
                                if let Some(ref mut wifi) = wifi {
                                    let connection = connection.get().await.unwrap();
                                    wifi.connection_ssid = connection;
                                    let _ = output.send(NetMessage::Wifi(Some(wifi.clone()))).await;
                                }
                            }
                        },
                        v = strength.next().fuse() => {
                            if let Some(strength) = v {
                                if let Some(ref mut wifi) = wifi {
                                    let value = strength.get().await.unwrap();

                                    if value.abs_diff(wifi.signal) > 10 {
                                        wifi.signal = value;
                                        let _ = output.send(NetMessage::Wifi(Some(wifi.clone()))).await;
                                    }
                                }
                            }
                        },
                    };
                } else if devices_change.next().await.is_some() {
                    maybe_proxies = init().await;

                    wifi = if let Some(proxies) = maybe_proxies.as_ref() {
                        Some(Wifi {
                            connection_ssid: proxies.connection.id().await.unwrap(),
                            signal: proxies.access_point.strength().await.unwrap(),
                        })
                    } else {
                        None
                    };

                    let _ = output.send(NetMessage::Wifi(None)).await;
                }
            }
        }),
        iced::subscription::channel(
            "nm-dbus-vpn-active-listener",
            100,
            |mut output| async move {
                let conn = Connection::system().await.unwrap();
                let nm = NetworkManagerProxy::new(&conn).await.unwrap();

                let mut connections = nm.receive_active_connections_changed().await;

                loop {
                    if let Some(connections) = connections.next().await {
                        let active_vpn = stream::iter(connections.get().await.unwrap().iter())
                            .any(|c| {
                                let conn = conn.clone();
                                async move {
                                    ActiveConnectionProxy::builder(&conn)
                                        .path(c.to_owned())
                                        .unwrap()
                                        .build()
                                        .await
                                        .unwrap()
                                        .vpn()
                                        .await
                                        .unwrap()
                                }
                            })
                            .await;

                        let _ = output.send(NetMessage::VpnActive(active_vpn)).await;
                    }
                }
            },
        ),
    ])
}
