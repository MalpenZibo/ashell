use crate::modules::settings::bluetooth::{BluetoothMessage, BluetoothState, Device};
use iced::{
    futures::{
        self,
        channel::mpsc::Sender,
        stream::{self, select_all},
        FutureExt, SinkExt, Stream, StreamExt,
    },
    subscription, Subscription,
};
use log::{debug, warn};
use std::collections::HashMap;
use zbus::{
    proxy,
    zvariant::{OwnedObjectPath, OwnedValue},
};

type ManagedObjects = HashMap<OwnedObjectPath, HashMap<String, HashMap<String, OwnedValue>>>;

#[proxy(
    default_service = "org.bluez",
    default_path = "/",
    interface = "org.freedesktop.DBus.ObjectManager"
)]
trait BluezObjectManager {
    fn get_managed_objects(&self) -> zbus::Result<ManagedObjects>;

    #[zbus(signal)]
    fn interfaces_added(&self) -> Result<()>;

    #[zbus(signal)]
    fn interfaces_removed(&self) -> Result<()>;
}

#[proxy(
    default_service = "org.bluez",
    default_path = "/org/bluez/hci0",
    interface = "org.bluez.Adapter1"
)]
trait Adapter {
    #[zbus(property)]
    fn powered(&self) -> zbus::Result<bool>;

    #[zbus(property)]
    fn set_powered(&self, value: bool) -> zbus::Result<()>;
}

#[proxy(default_service = "org.bluez", interface = "org.bluez.Device1")]
trait Device {
    #[zbus(property)]
    fn name(&self) -> zbus::Result<String>;

    #[zbus(property)]
    fn connected(&self) -> zbus::Result<bool>;
}

#[proxy(default_service = "org.bluez", interface = "org.bluez.Battery1")]
trait Battery {
    #[zbus(property)]
    fn percentage(&self) -> zbus::Result<u8>;
}

async fn get_adapter<'a>(bluez: &BluezObjectManagerProxy<'a>) -> Option<OwnedObjectPath> {
    bluez
        .get_managed_objects()
        .await
        .expect("Failed to get bluez managed objects")
        .into_iter()
        .filter_map(|(key, item)| {
            if item.contains_key("org.bluez.Adapter1") {
                Some(key)
            } else {
                None
            }
        })
        .next()
}

async fn get_connected_devices<'a>(
    conn: &zbus::Connection,
    bluez: &BluezObjectManagerProxy<'a>,
) -> Vec<(String, Option<(u8, BatteryProxy<'a>)>)> {
    let bluez = bluez
        .get_managed_objects()
        .await
        .expect("Failed to get bluez managed objects")
        .into_iter()
        .filter_map(|(key, item)| {
            if item.contains_key("org.bluez.Device1") {
                Some(key)
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    let mut devices = Vec::new();
    for device_path in bluez {
        let device = DeviceProxy::builder(conn)
            .path(device_path.clone())
            .expect("Failed to set DeviceProxy path")
            .build()
            .await
            .expect("Failed to build DeviceProxy");

        if let Ok(name) = device.name().await {
            let connected = device.connected().await;

            if connected == Ok(true) {
                let battery = if let Ok(battery) = BatteryProxy::builder(conn).path(device_path) {
                    let battery = battery.build().await;

                    if let Ok(battery) = battery {
                        let battery_value = battery.percentage().await;
                        if let Ok(battery_value) = battery_value {
                            Some((battery_value, battery))
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                };

                devices.push((name, battery));
            }
        }
    }

    devices
}

pub enum BluetoothCommand {
    TogglePower,
}

async fn handle_bluetooth_command<'a>(
    command: BluetoothCommand,
    adapter: &AdapterProxy<'a>,
) -> Result<BluetoothMessage, ()> {
    match command {
        BluetoothCommand::TogglePower => {
            if let Ok(current_state) = adapter.powered().await {
                debug!("Current adapter powered state: {}", current_state);
                adapter
                    .set_powered(!current_state)
                    .await
                    .map_err(|e| {
                        warn!("Failed to set adapter powered state: {:?}", e);
                    })
                    .map(|_| {
                        BluetoothMessage::Status(if !current_state {
                            BluetoothState::Inactive
                        } else {
                            BluetoothState::Active
                        })
                    })
            } else {
                warn!("Failed to get adapter powered state");
                Err(())
            }
        }
    }
}

async fn handle_adapter<'a>(
    conn: &zbus::Connection,
    bluez: &BluezObjectManagerProxy<'a>,
    adapter: &AdapterProxy<'a>,
    rx: &mut tokio::sync::mpsc::UnboundedReceiver<BluetoothCommand>,
    output: &mut Sender<BluetoothMessage>,
) {
    let _ = output
        .send(BluetoothMessage::Status(
            if adapter
                .powered()
                .await
                .expect("Failed to get adapter powered state")
            {
                BluetoothState::Active
            } else {
                BluetoothState::Inactive
            },
        ))
        .await;

    let mut powered_signal = adapter.receive_powered_changed().await;
    let mut added_signal = bluez
        .receive_interfaces_added()
        .await
        .expect("Failed to receive bluez interfaces added signal");
    let mut removed_signal = bluez
        .receive_interfaces_removed()
        .await
        .expect("Failed to receive bluez interfaces removed signal");

    loop {
        let devices = get_connected_devices(conn, bluez).await;
        let mut battery_signals = Vec::new();
        for (i, b) in devices.iter().enumerate().filter_map(|(i, (_, b))| {
            if let Some((_, b)) = b {
                Some((i, b))
            } else {
                None
            }
        }) {
            battery_signals.push(b.receive_percentage_changed().await.map(move |v| (i, v)))
        }

        let _ = output
            .send(BluetoothMessage::DeviceList(
                devices
                    .into_iter()
                    .map(|(n, b)| Device {
                        name: n.to_owned(),
                        battery: b.as_ref().map(|(v, _)| *v),
                    })
                    .collect::<Vec<_>>(),
            ))
            .await;

        let mut battery_signals: Box<dyn Stream<Item = _> + Unpin + Send> =
            if battery_signals.is_empty() {
                Box::new(stream::pending())
            } else {
                Box::new(select_all(battery_signals))
            };

        futures::select! {
            v = rx.recv().fuse() => {
                if let Some(v) = v {
                    let res = handle_bluetooth_command(v, adapter).await;
                    if let Ok(res) = res {
                        let _ = output.send(res).await;
                    } else {
                        return;
                    }
                }
            },
            v = powered_signal.next().fuse() => {
                if let Some(v) = v {
                    if let Ok(value) = v.get().await {
                        let _ = output.send(
                            BluetoothMessage::Status(if value {
                                BluetoothState::Active
                            } else {
                                BluetoothState::Inactive
                            })
                        ).await;
                    }
                }
            },
            _ = added_signal.next().fuse() => {
            },
            _ = removed_signal.next().fuse() => {
            },
            _ = battery_signals.next().fuse() => {
            }
        }
    }
}

pub fn subscription(
    rx: Option<tokio::sync::mpsc::UnboundedReceiver<BluetoothCommand>>,
) -> Subscription<BluetoothMessage> {
    subscription::channel(
        "bluez-dbus-connection-listener",
        100,
        |mut output| async move {
            let mut rx = rx.expect("Failed to get commander receiver");
            let conn = zbus::Connection::system()
                .await
                .expect("Failed to connect to system bus");

            loop {
                println!("Getting adapter");
                let bluez = BluezObjectManagerProxy::new(&conn)
                    .await
                    .expect("Failed to create bluez obj manager proxy");

                let adapter_path = get_adapter(&bluez).await;

                if let Some(adapter_path) = adapter_path {
                    let adapter = AdapterProxy::builder(&conn)
                        .path(adapter_path)
                        .expect("Failed to set adapter proxy path")
                        .build()
                        .await
                        .expect("Failed to create adapter proxy");

                    handle_adapter(&conn, &bluez, &adapter, &mut rx, &mut output).await;
                } else {
                    loop {
                        rx.recv().await;
                    }
                }
            }
        },
    )
}
