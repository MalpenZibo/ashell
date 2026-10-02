use crate::{bus, rfkill, stream::channel};
use dbus::{ADAPTER, BATTERY, BluetoothDbus, DEVICE};
use futures::{
    FutureExt, SinkExt, Stream, StreamExt, future::ready, stream::BoxStream, stream_select,
};
use log::{debug, error, warn};
use std::time::Duration;
use zbus::{
    MatchRule, MessageStream, fdo::PropertiesChanged, message::Type, zvariant::OwnedObjectPath,
};

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

            while let Some(event) = events.next().await {
                // One action fires a burst of signals (pairing: Paired, Connected,
                // Battery1, Percentage): read once for all those already queued.
                let mut event = Some(event);
                while let Some(current) = event {
                    if let Event::Bluez(running) = current {
                        bluez_running = running;
                    }
                    event = events.next().now_or_never().flatten();
                }

                // Reading bluez while it isn't running would activate it
                let data = if bluez_running {
                    match Self::read_data(&conn).await {
                        Ok(data) => data,
                        Err(err) => {
                            warn!("Failed to read bluetooth data: {err}");
                            continue;
                        }
                    }
                } else {
                    BluetoothData::default()
                };

                if last.as_ref() != Some(&data) {
                    last = Some(data.clone());
                    if output.send(data).await.is_err() {
                        return;
                    }
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
            Some(_) if rfkill::bluetooth_soft_blocked().await? => BluetoothState::Inactive,
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

        // One rule for every bluez object instead of a stream per device
        let rule = MatchRule::builder()
            .msg_type(Type::Signal)
            .interface("org.freedesktop.DBus.Properties")?
            .member("PropertiesChanged")?
            .path_namespace("/org/bluez")?
            .build();
        let properties = MessageStream::for_match_rule(rule, conn, None)
            .await?
            .filter_map(|message| ready(message.ok().filter(shows_change).map(|_| {})));

        let changed = stream_select!(topology, properties, rfkill::soft_block_changes().await?)
            .map(|_| Event::Changed);

        Ok(stream_select!(bluez, changed).boxed())
    }
}

/// Whether a `PropertiesChanged` touches something `BluetoothData` shows, so
/// e.g. the RSSI updates during discovery don't trigger a read each.
fn shows_change(message: &zbus::Message) -> bool {
    let Some(signal) = PropertiesChanged::from_message(message.clone()) else {
        return false;
    };
    let Ok(args) = signal.args() else {
        return false;
    };

    let shown: &[&str] = match args.interface_name().as_str() {
        ADAPTER => &["Powered", "Discovering"],
        DEVICE => &["Alias", "Connected", "Paired"],
        BATTERY => &["Percentage"],
        _ => return false,
    };

    args.changed_properties()
        .keys()
        .chain(args.invalidated_properties().iter())
        .any(|name| shown.contains(name))
}
