use dbus::BluetoothDbus;
use futures::StreamExt;
use guido::prelude::*;
use inotify::{Inotify, WatchMask};
use log::{debug, error, info, warn};
use std::io::ErrorKind;
use zbus::zvariant::OwnedObjectPath;

pub mod dbus;

#[derive(PartialEq, Eq, Debug, Clone)]
pub enum BluetoothState {
    Unavailable,
    Active,
    Inactive,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BluetoothDevice {
    pub name: String,
    pub battery: Option<u8>,
    pub path: OwnedObjectPath,
    pub connected: bool,
    pub paired: bool,
}

#[derive(Clone, PartialEq, guido::SignalFields)]
pub struct BluetoothData {
    pub state: BluetoothState,
    pub devices: Vec<BluetoothDevice>,
    pub discovering: bool,
}

impl Default for BluetoothData {
    fn default() -> Self {
        Self {
            state: BluetoothState::Unavailable,
            devices: Vec::new(),
            discovering: false,
        }
    }
}

#[derive(Clone)]
pub enum BluetoothCmd {
    /// BluetoothState read on main thread before sending
    Toggle(BluetoothState),
    StartDiscovery,
    StopDiscovery,
    PairDevice(OwnedObjectPath),
    ConnectDevice(OwnedObjectPath),
    DisconnectDevice(OwnedObjectPath),
    RemoveDevice(OwnedObjectPath),
}

pub fn create() -> (BluetoothDataSignals, Service<BluetoothCmd>) {
    let data = BluetoothDataSignals::new(BluetoothData::default());
    let svc = start_bluetooth_service(data.writers());
    (data, svc)
}

async fn initialize_data(conn: &zbus::Connection) -> anyhow::Result<BluetoothData> {
    let bluetooth = BluetoothDbus::new(conn).await?;
    let state = bluetooth.state().await?;
    let rfkill_soft_block = check_rfkill_soft_block().await;

    let state = match state {
        BluetoothState::Unavailable => BluetoothState::Unavailable,
        BluetoothState::Active if rfkill_soft_block => BluetoothState::Inactive,
        state => state,
    };
    let devices = bluetooth.devices().await?;
    let discovering = bluetooth.discovering().await.unwrap_or(false);

    Ok(BluetoothData {
        state,
        devices,
        discovering,
    })
}

async fn check_rfkill_soft_block() -> bool {
    let output = tokio::process::Command::new("rfkill")
        .args(["list", "bluetooth"])
        .output()
        .await;
    match output {
        Ok(o) => String::from_utf8_lossy(&o.stdout).contains("Soft blocked: yes"),
        Err(e) if e.kind() == ErrorKind::NotFound => {
            warn!("rfkill binary not found");
            false
        }
        Err(_) => false,
    }
}

fn start_bluetooth_service(writers: BluetoothDataWriters) -> Service<BluetoothCmd> {
    create_service::<BluetoothCmd, _, _>(move |mut rx, ctx| async move {
        let conn = match zbus::Connection::system().await {
            Ok(c) => c,
            Err(e) => {
                error!("Failed to connect to system bus: {e}");
                return;
            }
        };

        // Initialize
        match initialize_data(&conn).await {
            Ok(data) => {
                info!("Bluetooth service initialized");
                writers.set(data);
            }
            Err(e) => {
                error!("Failed to initialize bluetooth: {e}");
                return;
            }
        }

        // Set up event streams
        let bluetooth = match BluetoothDbus::new(&conn).await {
            Ok(bt) => bt,
            Err(e) => {
                error!("Failed to create BluetoothDbus: {e}");
                // Still handle commands
                while ctx.is_running() {
                    if let Some(cmd) = rx.recv().await {
                        handle_bt_cmd(&conn, &writers, cmd).await;
                    } else {
                        break;
                    }
                }
                return;
            }
        };

        // Listen for interface add/remove (device discovery, adapter changes)
        let mut iface_added = match bluetooth.bluez.receive_interfaces_added().await {
            Ok(s) => s.map(|_| ()).boxed(),
            Err(_) => futures::stream::pending().boxed(),
        };
        let mut iface_removed = match bluetooth.bluez.receive_interfaces_removed().await {
            Ok(s) => s.map(|_| ()).boxed(),
            Err(_) => futures::stream::pending().boxed(),
        };

        // Listen for adapter powered/discovering changes
        let mut powered_stream = match &bluetooth.adapter {
            Some(adapter) => adapter.receive_powered_changed().await.map(|_| ()).boxed(),
            None => futures::stream::pending().boxed(),
        };
        let mut discovering_stream = match &bluetooth.adapter {
            Some(adapter) => adapter.receive_discovering_changed().await.map(|_| ()).boxed(),
            None => futures::stream::pending().boxed(),
        };

        // rfkill changes
        let mut rfkill_stream = match listen_rfkill_changes().await {
            Ok(s) => s,
            Err(_) => futures::stream::pending().boxed(),
        };

        while ctx.is_running() {
            tokio::select! {
                cmd = rx.recv() => {
                    match cmd {
                        Some(cmd) => handle_bt_cmd(&conn, &writers, cmd).await,
                        None => break,
                    }
                }
                _ = iface_added.next() => {
                    refresh_data(&conn, &writers).await;
                }
                _ = iface_removed.next() => {
                    refresh_data(&conn, &writers).await;
                }
                _ = powered_stream.next() => {
                    refresh_data(&conn, &writers).await;
                }
                _ = discovering_stream.next() => {
                    refresh_data(&conn, &writers).await;
                }
                _ = rfkill_stream.next() => {
                    refresh_data(&conn, &writers).await;
                }
            }
        }
    })
}

async fn refresh_data(conn: &zbus::Connection, writers: &BluetoothDataWriters) {
    if let Ok(data) = initialize_data(conn).await {
        writers.set(data);
    }
}

async fn handle_bt_cmd(
    conn: &zbus::Connection,
    writers: &BluetoothDataWriters,
    cmd: BluetoothCmd,
) {
    let bt = match BluetoothDbus::new(conn).await {
        Ok(bt) => bt,
        Err(e) => {
            error!("Failed to create BluetoothDbus for command: {e}");
            return;
        }
    };

    match cmd {
        BluetoothCmd::Toggle(current) => {
            if current == BluetoothState::Unavailable {
                return;
            }
            let powered = current == BluetoothState::Active;
            debug!("Toggling bluetooth power to: {}", !powered);
            let _ = bt.set_powered(!powered).await;
            refresh_data(conn, writers).await;
        }
        BluetoothCmd::StartDiscovery => {
            let _ = bt.start_discovery().await;
            tokio::time::sleep(tokio::time::Duration::from_secs(15)).await;
            let _ = bt.stop_discovery().await;
            refresh_data(conn, writers).await;
        }
        BluetoothCmd::StopDiscovery => {
            let _ = bt.stop_discovery().await;
            refresh_data(conn, writers).await;
        }
        BluetoothCmd::PairDevice(path) => {
            debug!("Pairing device: {:?}", path);
            let _ = bt.pair_device(&path).await;
            refresh_data(conn, writers).await;
        }
        BluetoothCmd::ConnectDevice(path) => {
            debug!("Connecting device: {:?}", path);
            let _ = bt.connect_device(&path).await;
            refresh_data(conn, writers).await;
        }
        BluetoothCmd::DisconnectDevice(path) => {
            debug!("Disconnecting device: {:?}", path);
            let _ = bt.disconnect_device(&path).await;
            refresh_data(conn, writers).await;
        }
        BluetoothCmd::RemoveDevice(path) => {
            debug!("Removing device: {:?}", path);
            let _ = bt.remove_device(&path).await;
            refresh_data(conn, writers).await;
        }
    }
}

async fn listen_rfkill_changes() -> anyhow::Result<futures::stream::BoxStream<'static, ()>> {
    let inotify = Inotify::init()?;
    match inotify.watches().add("/dev/rfkill", WatchMask::MODIFY) {
        Ok(_) => {
            let buffer = [0; 512];
            Ok(inotify.into_event_stream(buffer)?.map(|_| ()).boxed())
        }
        Err(err) if err.kind() == ErrorKind::NotFound => {
            warn!("/dev/rfkill not found");
            Ok(futures::stream::pending().boxed())
        }
        Err(err) => Err(err.into()),
    }
}
