use crate::{
    bus,
    rfkill::{self, Kind, Switches},
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
use std::{
    io::{self, ErrorKind},
    pin::pin,
    time::Duration,
};
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
    /// Whether a Bluetooth rfkill switch is soft-blocked, `None` while
    /// `/dev/rfkill` can't be read.
    Rfkill(Option<bool>),
    /// Something shown in `BluetoothData` may have changed.
    Changed,
}

/// What the events tell beyond bluez's objects.
#[derive(Default)]
struct Sources {
    bluez_running: bool,
    /// The last `Event::Rfkill`, `None` before the first, which comes right
    /// after subscribing.
    rfkill: Option<Option<bool>>,
    /// The last soft block known, kept while none can be read.
    soft_blocked: bool,
}

impl Sources {
    fn apply(&mut self, event: Event) {
        match event {
            Event::Bluez(running) => self.bluez_running = running,
            Event::Rfkill(blocked) => self.rfkill = Some(blocked),
            Event::Changed => {}
        }
    }

    /// The soft block to show, `None` before the first `Event::Rfkill`.
    /// While `/dev/rfkill` can't be read it asks `sysfs` on every read, and
    /// keeps the last known value when that fails too.
    fn soft_blocked(&mut self, sysfs: impl FnOnce() -> io::Result<bool>) -> Option<bool> {
        let blocked = match self.rfkill? {
            Some(blocked) => blocked,
            None => sysfs().unwrap_or(self.soft_blocked),
        };
        self.soft_blocked = blocked;

        Some(blocked)
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

            let mut sources = Sources::default();
            let mut last = None;
            // Something changed while the last read was in flight: read again
            let mut stale = false;

            loop {
                if !stale {
                    let Some(event) = events.next().await else {
                        return;
                    };
                    sources.apply(event);
                }
                stale = false;

                // One action fires a burst of signals (pairing: Paired, Connected,
                // Battery1, Percentage): read once for all those already queued.
                while let Some(event) = events.next().now_or_never().flatten() {
                    sources.apply(event);
                }

                let Some(soft_blocked) =
                    sources.soft_blocked(|| rfkill::sysfs_soft_blocked(Kind::Bluetooth))
                else {
                    continue;
                };

                // Reading bluez while it isn't running would activate it
                let data = if sources.bluez_running {
                    // Keep consuming events while waiting for the reply: once a zbus
                    // signal queue is full, the connection stops reading replies too.
                    let mut read = pin!(Self::read_data(&conn, soft_blocked));
                    let result = loop {
                        match select(read.as_mut(), events.next()).await {
                            Either::Left((result, _)) => break result,
                            Either::Right((Some(event), _)) => {
                                sources.apply(event);
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
            BluetoothCommand::Toggle => {
                match Self::read_data(&self.conn, soft_blocked_now()).await?.state {
                    BluetoothState::Unavailable => {}
                    BluetoothState::Active => bluetooth.set_powered(false).await?,
                    BluetoothState::Inactive => bluetooth.set_powered(true).await?,
                }
            }
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

    async fn read_data(
        conn: &zbus::Connection,
        soft_blocked: bool,
    ) -> anyhow::Result<BluetoothData> {
        let snapshot = dbus::snapshot(conn).await?;

        let state = match snapshot.adapter {
            None => BluetoothState::Unavailable,
            Some(adapter) if !adapter.powered => BluetoothState::Inactive,
            Some(_) if soft_blocked => BluetoothState::Inactive,
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

        // Every switch change comes through, keep only the Bluetooth soft block
        let mut last = None;
        let rfkill = rfkill::switches().boxed().filter_map(move |state| {
            let blocked = state.map(|state| soft_blocked(&state.switches));
            ready((last.replace(blocked) != Some(blocked)).then_some(Event::Rfkill(blocked)))
        });

        let changed = stream_select!(topology, properties).map(|_| Event::Changed);

        Ok(stream_select!(bluez, rfkill, changed).boxed())
    }
}

/// Whether any Bluetooth rfkill switch is soft-blocked. A real hard block
/// already shows as an unpowered adapter, and a platform switch that reports
/// one wrongly must not show a working radio as off.
fn soft_blocked(switches: &Switches) -> bool {
    switches
        .values()
        .any(|switch| switch.kind == Kind::Bluetooth && switch.soft)
}

/// [`soft_blocked`] read once, from sysfs when `/dev/rfkill` can't be read. A
/// system without rfkill has nothing to block, and so does one whose switches
/// can't be read at all: this only tells whether the radio is off, it never
/// changes it.
fn soft_blocked_now() -> bool {
    rfkill::current()
        .map(|switches| soft_blocked(&switches))
        .or_else(|_| rfkill::sysfs_soft_blocked(Kind::Bluetooth))
        .unwrap_or_else(|err| {
            if err.kind() != ErrorKind::NotFound {
                warn!("Can't read the rfkill state, assuming bluetooth is not soft blocked: {err}");
            }
            false
        })
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rfkill::Switch;

    fn switches(radios: &[(Kind, bool, bool)]) -> Switches {
        radios
            .iter()
            .zip(0..)
            .map(|(&(kind, soft, hard), idx)| (idx, Switch { kind, soft, hard }))
            .collect()
    }

    #[test]
    fn unblocked_without_bluetooth_switches() {
        assert!(!soft_blocked(&switches(&[])));
        assert!(!soft_blocked(&switches(&[(Kind::Wlan, true, false)])));
    }

    #[test]
    fn any_soft_blocked_bluetooth_switch_blocks() {
        assert!(soft_blocked(&switches(&[
            (Kind::Bluetooth, false, false),
            (Kind::Bluetooth, true, false),
        ])));
    }

    #[test]
    fn waits_for_the_first_rfkill_state() {
        let mut sources = Sources::default();
        assert_eq!(sources.soft_blocked(|| Ok(true)), None);
    }

    #[test]
    fn asks_sysfs_while_rfkill_is_unreadable() {
        let mut sources = Sources::default();
        sources.apply(Event::Rfkill(None));
        assert_eq!(sources.soft_blocked(|| Ok(true)), Some(true));
    }

    #[test]
    fn keeps_the_last_block_when_nothing_can_be_read() {
        let unreadable = || Err(ErrorKind::PermissionDenied.into());
        let mut sources = Sources::default();
        sources.apply(Event::Rfkill(Some(true)));
        assert_eq!(sources.soft_blocked(unreadable), Some(true));

        sources.apply(Event::Rfkill(None));
        assert_eq!(sources.soft_blocked(unreadable), Some(true));

        sources.apply(Event::Rfkill(Some(false)));
        assert_eq!(sources.soft_blocked(unreadable), Some(false));
    }

    #[test]
    fn hard_block_alone_does_not_count() {
        assert!(!soft_blocked(&switches(&[(Kind::Bluetooth, false, true)])));
    }
}
