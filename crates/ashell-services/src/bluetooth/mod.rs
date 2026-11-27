use crate::{
    bus, rfkill,
    stream::{channel, send},
};
use dbus::{ADAPTER, BATTERY, BluetoothDbus, DEVICE};
use futures::{
    FutureExt, Stream, StreamExt,
    future::{Either, ready, select},
    stream::BoxStream,
    stream_select,
};
use log::{debug, error, warn};
use std::{pin::pin, time::Duration};
use zbus::{OwnedObjectPath, fdo::PropertiesChanged};

mod dbus;

const BLUEZ: &str = "org.bluez";
const DISCOVERY_DURATION: Duration = Duration::from_secs(15);

#[derive(PartialEq, Eq, Debug, Clone)]
pub enum BluetoothState {
    Unavailable,
    Active,
    Inactive,
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct BluetoothDevice {
    pub name: String,
    pub battery: Option<u8>,
    pub path: OwnedObjectPath,
    pub connected: bool,
    pub paired: bool,
}

#[derive(PartialEq, Eq, Debug, Clone)]
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

#[derive(Debug, Clone)]
pub enum BluetoothCommand {
    Toggle,
    StartDiscovery,
    PairDevice(OwnedObjectPath),
    ConnectDevice(OwnedObjectPath),
    DisconnectDevice(OwnedObjectPath),
    RemoveDevice(OwnedObjectPath),
}

enum Event {
    /// Whether bluez is running.
    Bluez(bool),
    /// Something shown in `BluetoothData` may have changed.
    Changed,
}

impl Event {
    fn apply(self, bluez_running: &mut bool) {
        if let Self::Bluez(running) = self {
            *bluez_running = running;
        }
    }
}

#[derive(Debug, Clone)]
pub struct Bluetooth {
    conn: zbus::Connection,
}

impl Bluetooth {
    pub async fn connect() -> anyhow::Result<Self> {
        let conn = zbus::Connection::system().await?;

        Ok(Self { conn })
    }

    /// Yields the current state once subscribed, then a fresh snapshot on every change.
    pub fn updates(&self) -> impl Stream<Item = BluetoothData> + Send + 'static {
        let conn = self.conn.clone();

        channel(100, |mut output| async move {
            let mut events = match Self::events(&conn).await {
                Ok(events) => events,
                Err(err) => {
                    error!("Failed to listen for bluetooth events: {err}");
                    return;
                }
            };

            let mut bluez_running = false;
            let mut last = None;
            // Something changed while the last read was in flight: read again
            let mut stale = false;

            loop {
                if !stale {
                    let Some(event) = events.next().await else {
                        return;
                    };
                    event.apply(&mut bluez_running);
                }
                stale = false;

                // One action fires a burst of signals (pairing: Paired, Connected,
                // Battery1, Percentage): read once for all those already queued.
                while let Some(event) = events.next().now_or_never().flatten() {
                    event.apply(&mut bluez_running);
                }

                // Reading bluez while it isn't running would activate it
                let data = if bluez_running {
                    // Keep consuming events while waiting for the reply: once a zbus
                    // signal queue is full, the connection stops reading replies too.
                    let mut read = pin!(Self::read_data(&conn));
                    let result = loop {
                        match select(read.as_mut(), events.next()).await {
                            Either::Left((result, _)) => break result,
                            Either::Right((Some(event), _)) => {
                                event.apply(&mut bluez_running);
                                stale = true;
                            }
                            Either::Right((None, _)) => return,
                        }
                    };

                    match result {
                        Ok(data) => data,
                        Err(err) => {
                            warn!("Failed to read bluetooth data: {err}");
                            continue;
                        }
                    }
                } else {
                    BluetoothData::default()
                };

                if !send(&mut output, &mut last, data).await {
                    return;
                }
            }
        })
    }

    pub async fn execute(&self, command: BluetoothCommand) -> anyhow::Result<()> {
        debug!("Bluetooth command: {command:?}");

        let bluetooth = BluetoothDbus::new(&self.conn).await?;

        match command {
            BluetoothCommand::Toggle => match Self::read_data(&self.conn).await?.state {
                BluetoothState::Unavailable => {}
                BluetoothState::Active => bluetooth.set_powered(false).await?,
                BluetoothState::Inactive => bluetooth.set_powered(true).await?,
            },
            BluetoothCommand::StartDiscovery => {
                bluetooth.start_discovery().await?;
                tokio::time::sleep(DISCOVERY_DURATION).await;
                bluetooth.stop_discovery().await?;
            }
            BluetoothCommand::PairDevice(device) => bluetooth.pair_device(&device).await?,
            BluetoothCommand::ConnectDevice(device) => bluetooth.connect_device(&device).await?,
            BluetoothCommand::DisconnectDevice(device) => {
                bluetooth.disconnect_device(&device).await?
            }
            BluetoothCommand::RemoveDevice(device) => bluetooth.remove_device(&device).await?,
        }

        Ok(())
    }

    async fn read_data(conn: &zbus::Connection) -> anyhow::Result<BluetoothData> {
        let snapshot = dbus::snapshot(conn).await?;

        let state = match snapshot.adapter {
            None => BluetoothState::Unavailable,
            Some(adapter) if !adapter.powered => BluetoothState::Inactive,
            Some(_) if rfkill::bluetooth_soft_blocked().await => BluetoothState::Inactive,
            Some(_) => BluetoothState::Active,
        };

        Ok(BluetoothData {
            state,
            discovering: snapshot.adapter.is_some_and(|adapter| adapter.discovering),
            devices: snapshot.devices,
        })
    }

    /// Everything that can change `BluetoothData`, subscribed once: bluez
    /// starting or stopping, objects appearing or going, the properties shown,
    /// and rfkill.
    async fn events(conn: &zbus::Connection) -> anyhow::Result<BoxStream<'static, Event>> {
        let bluez = bus::owner_watch(conn, BLUEZ).await?.map(Event::Bluez);

        let objects = dbus::object_manager(conn).await?;
        let topology = stream_select!(
            objects.receive_interfaces_added().await?.map(|_| {}),
            objects.receive_interfaces_removed().await?.map(|_| {}),
        );

        let properties = bus::properties_changed(conn, BLUEZ, "/org/bluez")
            .await?
            .filter_map(|signal| ready(shows_change(&signal).then_some(())));

        let changed = stream_select!(topology, properties, rfkill::soft_block_changes())
            .map(|_| Event::Changed);

        Ok(stream_select!(bluez, changed).boxed())
    }
}

/// Whether a `PropertiesChanged` touches something `BluetoothData` shows, so
/// e.g. the RSSI updates during discovery don't trigger a read each.
fn shows_change(signal: &PropertiesChanged) -> bool {
    let Ok(args) = signal.args() else {
        return false;
    };

    let shown: &[&str] = match args.interface_name().as_str() {
        ADAPTER => &["Powered", "Discovering"],
        DEVICE => &["Alias", "Connected", "Paired"],
        BATTERY => &["Percentage"],
        _ => return false,
    };

    bus::changes_any(&args, shown)
}
