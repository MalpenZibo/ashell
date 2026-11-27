//! The kernel rfkill interface: the radio switches and their soft blocks.
//!
//! This talks to `/dev/rfkill` directly instead of spawning the `rfkill`
//! binary: it lives in `/usr/sbin` on some distros, which is not in a normal
//! user's `PATH` there (openSUSE, ALT), and NixOS has no `/usr/sbin` at all.
//! The binary is not setuid, so writing `/dev/rfkill` ourselves needs the
//! same permissions it does: systemd gives the user of a seat write access to
//! the device node, on other systems the user has to be in the `rfkill`
//! group.

use crate::{
    retry::{RETRY_DELAY, failed},
    stream::{channel, send},
    udev_monitor,
};
use futures::{
    FutureExt, Stream, StreamExt,
    channel::mpsc::Sender,
    future::ready,
    select,
    stream::{BoxStream, pending},
};
use inotify::{Inotify, WatchMask};
use log::{info, warn};
use std::{
    collections::BTreeMap,
    ffi::{CStr, OsStr},
    fs::{self, File, OpenOptions},
    io::{self, ErrorKind, Read, Write},
    os::unix::fs::OpenOptionsExt,
    path::Path,
};
use tokio::{
    io::{Interest, unix::AsyncFd},
    time::{sleep, timeout},
};

const DEV_DIR: &str = "/dev";
const DEV_NAME: &str = "rfkill";
const DEV_RFKILL: &str = "/dev/rfkill";
const DEV_RFKILL_C: &CStr = c"/dev/rfkill";
const SYSFS_RFKILL: &str = "/sys/class/rfkill";

// From linux/rfkill.h.
const TYPE_WLAN: u8 = 1;
const TYPE_BLUETOOTH: u8 = 2;
const OP_ADD: u8 = 0;
const OP_DEL: u8 = 1;
const OP_CHANGE: u8 = 2;
const OP_CHANGE_ALL: u8 = 3;
const EVENT_SIZE: usize = 8;
// The `type` attribute in sysfs, by `RFKILL_TYPE_*`.
const SYSFS_TYPES: [&str; 9] = [
    "all",
    "wlan",
    "bluetooth",
    "ultrawideband",
    "wimax",
    "wwan",
    "gps",
    "fm",
    "nfc",
];

/// The radio a switch blocks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Kind {
    Wlan,
    Bluetooth,
    /// Any other `RFKILL_TYPE_*`, such as WWAN, NFC or GPS.
    Other(OtherKind),
}

/// A radio type without a variant of its own, never Wi-Fi or Bluetooth.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OtherKind(u8);

impl OtherKind {
    /// The `RFKILL_TYPE_*` value.
    pub fn raw(self) -> u8 {
        self.0
    }
}

impl Kind {
    pub(crate) fn from_raw(raw: u8) -> Self {
        match raw {
            TYPE_WLAN => Self::Wlan,
            TYPE_BLUETOOTH => Self::Bluetooth,
            other => Self::Other(OtherKind(other)),
        }
    }

    fn raw(self) -> u8 {
        match self {
            Self::Wlan => TYPE_WLAN,
            Self::Bluetooth => TYPE_BLUETOOTH,
            Self::Other(other) => other.raw(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Switch {
    pub kind: Kind,
    pub soft: bool,
    pub hard: bool,
}

impl Switch {
    pub fn blocked(&self) -> bool {
        self.soft || self.hard
    }
}

/// The switches by index, as in `/sys/class/rfkill/rfkill<idx>`.
pub type Switches = BTreeMap<u32, Switch>;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct State {
    pub switches: Switches,
    /// Whether this process may write `/dev/rfkill`, checked on every change
    /// of the switches and of the node's permissions.
    pub writable: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Op {
    Add,
    Del,
    Change,
    Other,
}

/// A `struct rfkill_event` as `/dev/rfkill` reports it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Event {
    idx: u32,
    switch: Switch,
    op: Op,
}

impl Event {
    // struct rfkill_event { u32 idx; u8 type; u8 op; u8 soft; u8 hard; }
    fn parse(bytes: [u8; EVENT_SIZE]) -> Self {
        Self {
            idx: u32::from_ne_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
            switch: Switch {
                kind: Kind::from_raw(bytes[4]),
                soft: bytes[6] != 0,
                hard: bytes[7] != 0,
            },
            op: match bytes[5] {
                OP_ADD => Op::Add,
                OP_DEL => Op::Del,
                OP_CHANGE => Op::Change,
                _ => Op::Other,
            },
        }
    }
}

fn apply(switches: &mut Switches, events: impl IntoIterator<Item = Event>) {
    for event in events {
        match event.op {
            Op::Add | Op::Change => {
                switches.insert(event.idx, event.switch);
            }
            Op::Del => {
                switches.remove(&event.idx);
            }
            Op::Other => {}
        }
    }
}

/// Yields the switches first as they are, then after every change. `None`
/// while `/dev/rfkill` can't be read: it waits for the device node to appear,
/// and retries after other errors.
pub fn switches() -> impl Stream<Item = Option<State>> + Send + 'static {
    channel(16, |mut output| async move {
        let mut last = None;
        let mut failing = false;

        loop {
            match Reader::open() {
                Ok(reader) => match follow(reader, &mut output, &mut last, &mut failing).await {
                    Ok(()) => return,
                    Err(err) => failed!(&mut failing, "Stopped reading {DEV_RFKILL}: {err}"),
                },
                Err(err) if err.kind() == ErrorKind::NotFound => {
                    if !send(&mut output, &mut last, None).await {
                        return;
                    }
                    info!("No {DEV_RFKILL}, waiting for it");
                    wait_for_node().await;
                    continue;
                }
                Err(err) => failed!(&mut failing, "Can't open {DEV_RFKILL}: {err}"),
            }

            if !send(&mut output, &mut last, None).await {
                return;
            }
            udev_announced(udev_monitor::listen("misc").ok()).await;
        }
    })
}

/// Sends the switches until reading fails, or returns `Ok` once nobody
/// listens anymore.
async fn follow(
    mut reader: Reader,
    output: &mut Sender<Option<State>>,
    last: &mut Option<Option<State>>,
    failing: &mut bool,
) -> io::Result<()> {
    // logind adds and removes the uaccess ACL without any rfkill event
    let mut permissions = node_changes(WatchMask::ATTRIB);
    let mut switches = Switches::new();
    apply(&mut switches, reader.queued()?);

    loop {
        let state = State {
            switches: switches.clone(),
            writable: writable(),
        };
        if !send(output, last, Some(state)).await {
            return Ok(());
        }
        *failing = false;

        select! {
            batch = reader.next_batch().fuse() => apply(&mut switches, batch?),
            _ = permissions.next().fuse() => {}
        }
    }
}

/// Returns once `/dev/rfkill` exists and udev has set it up. The watches only
/// live while waiting.
async fn wait_for_node() {
    let udev = udev_monitor::listen("misc").ok();
    let watch = Inotify::init().and_then(|inotify| {
        inotify.watches().add(DEV_DIR, WatchMask::CREATE)?;
        inotify.into_event_stream([0; 1024])
    });
    let mut events = match watch {
        Ok(events) => events,
        Err(err) => {
            warn!("Can't watch {DEV_DIR}, looking for {DEV_RFKILL} again later: {err}");
            sleep(RETRY_DELAY).await;
            return;
        }
    };

    // Created between the failed open and the watch
    if Path::new(DEV_RFKILL).exists() {
        return;
    }

    while let Some(event) = events.next().await {
        match event {
            Ok(event) if event.name.as_deref() == Some(OsStr::new(DEV_NAME)) => break,
            Ok(_) => {}
            Err(_) => {
                sleep(RETRY_DELAY).await;
                return;
            }
        }
    }

    // The node is created before udev gives it its group, mode and ACL
    udev_announced(udev).await;
}

/// Returns when udev reports the device node, which is when it has applied
/// its permissions, or after [`RETRY_DELAY`].
async fn udev_announced(socket: Option<AsyncFd<udev::MonitorSocket>>) {
    let Some(mut socket) = socket else {
        sleep(RETRY_DELAY).await;
        return;
    };

    let udev_event = async {
        loop {
            let Ok(mut guard) = socket.readable_mut().await else {
                return std::future::pending().await;
            };
            let found = guard
                .get_inner()
                .iter()
                .any(|event| event.sysname() == DEV_NAME);
            guard.clear_ready();
            if found {
                return;
            }
        }
    };

    let _ = timeout(RETRY_DELAY, udev_event).await;
}

/// Fires on the inotify events of `mask` on `/dev/rfkill`. Never fails:
/// without a watch, or once it errors, it just stays silent.
fn node_changes(mask: WatchMask) -> BoxStream<'static, ()> {
    let watch = Inotify::init().and_then(|inotify| {
        inotify.watches().add(DEV_RFKILL, mask)?;
        inotify.into_event_stream([0; 512])
    });

    match watch {
        Ok(events) => events
            .take_while(|event| ready(event.is_ok()))
            .map(|_| {})
            .chain(pending())
            .boxed(),
        Err(err) => {
            warn!("Can't watch {DEV_RFKILL}, its changes won't be noticed: {err}");
            pending().boxed()
        }
    }
}

fn writable() -> bool {
    // SAFETY: the path is a valid NUL-terminated string.
    unsafe { libc::access(DEV_RFKILL_C.as_ptr(), libc::W_OK) == 0 }
}

/// `/dev/rfkill` opened for reading. The kernel queues an add event for every
/// switch present at open, then an event per change.
struct Reader(AsyncFd<File>);

impl Reader {
    fn open() -> io::Result<Self> {
        let file = open(false)?;
        // SAFETY: the File owns its fd until it is dropped, and `as_raw_fd`
        // always returns that same fd.
        Ok(Self(unsafe {
            AsyncFd::register_with_interest(file, Interest::READABLE)
        }?))
    }

    /// Every event queued so far, without waiting.
    fn queued(&self) -> io::Result<Vec<Event>> {
        read_queued(self.0.get_ref())
    }

    /// Waits for events, then returns every one queued.
    async fn next_batch(&mut self) -> io::Result<Vec<Event>> {
        loop {
            let mut guard = self.0.readable().await?;
            // Read until WouldBlock, so the readiness can be cleared
            let events = read_queued(guard.get_inner())?;
            guard.clear_ready();
            if !events.is_empty() {
                return Ok(events);
            }
        }
    }
}

fn open(write: bool) -> io::Result<File> {
    OpenOptions::new()
        .read(true)
        .write(write)
        .custom_flags(libc::O_NONBLOCK)
        .open(DEV_RFKILL)
}

/// The switches as they are now, read once from `/dev/rfkill`.
pub fn current() -> io::Result<Switches> {
    read_switches(&open(false)?)
}

/// Whether any switch of `kind` is soft-blocked, read from sysfs. Anyone can
/// read sysfs, while `/dev/rfkill` is root-only without systemd's udev rules.
pub fn sysfs_soft_blocked(kind: Kind) -> io::Result<bool> {
    let Some(&name) = SYSFS_TYPES.get(usize::from(kind.raw())) else {
        return Ok(false);
    };

    for entry in fs::read_dir(SYSFS_RFKILL)? {
        let path = entry?.path();
        let attr = |attr| fs::read_to_string(path.join(attr)).unwrap_or_default();
        if attr("type").trim() == name && attr("soft").trim() == "1" {
            return Ok(true);
        }
    }

    Ok(false)
}

fn read_switches(file: &File) -> io::Result<Switches> {
    let mut switches = Switches::new();
    apply(&mut switches, read_queued(file)?);
    Ok(switches)
}

fn read_queued(mut file: &File) -> io::Result<Vec<Event>> {
    let mut events = Vec::new();
    let mut buf = [0; EVENT_SIZE];
    loop {
        match file.read(&mut buf) {
            Ok(EVENT_SIZE) => events.push(Event::parse(buf)),
            Ok(0) => return Err(ErrorKind::UnexpectedEof.into()),
            Ok(_) => {}
            Err(err) if err.kind() == ErrorKind::WouldBlock => return Ok(events),
            Err(err) if err.kind() == ErrorKind::Interrupted => {}
            Err(err) => return Err(err),
        }
    }
}

/// `/dev/rfkill` opened for writing, with the switches present when it was
/// opened.
pub struct Control {
    file: File,
    switches: Switches,
}

impl Control {
    pub fn open() -> io::Result<Self> {
        let file = open(true)?;
        let switches = read_switches(&file)?;

        Ok(Self { file, switches })
    }

    pub fn switches(&self) -> &Switches {
        &self.switches
    }

    /// Soft-blocks or unblocks every switch of each of `kinds`, like
    /// `rfkill block|unblock <type>`. The kernel also applies the new state
    /// to devices of those kinds plugged in later.
    pub fn set_soft_blocks(&self, kinds: &[Kind], blocked: bool) -> io::Result<()> {
        for kind in kinds {
            (&self.file).write_all(&change_all_event(*kind, blocked))?;
        }

        Ok(())
    }
}

// idx is ignored for RFKILL_OP_CHANGE_ALL, the kernel only changes the
// switches of the given type.
fn change_all_event(kind: Kind, blocked: bool) -> [u8; EVENT_SIZE] {
    [0, 0, 0, 0, kind.raw(), OP_CHANGE_ALL, u8::from(blocked), 0]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(idx: u32, kind: u8, op: u8, soft: bool, hard: bool) -> Event {
        let idx = idx.to_ne_bytes();
        Event::parse([
            idx[0],
            idx[1],
            idx[2],
            idx[3],
            kind,
            op,
            u8::from(soft),
            u8::from(hard),
        ])
    }

    #[test]
    fn change_all_event_matches_kernel_abi() {
        assert_eq!(
            change_all_event(Kind::Bluetooth, true),
            [0, 0, 0, 0, 2, 3, 1, 0]
        );
        assert_eq!(
            change_all_event(Kind::Wlan, false),
            [0, 0, 0, 0, 1, 3, 0, 0]
        );
        assert_eq!(
            change_all_event(Kind::from_raw(5), true),
            [0, 0, 0, 0, 5, 3, 1, 0]
        );
    }

    #[test]
    fn parses_kernel_event() {
        assert_eq!(
            event(7, 1, 2, true, false),
            Event {
                idx: 7,
                switch: Switch {
                    kind: Kind::Wlan,
                    soft: true,
                    hard: false,
                },
                op: Op::Change,
            }
        );
        assert_eq!(event(0, 5, 3, false, true).switch.kind.raw(), 5);
        assert_eq!(event(0, 2, 3, false, true).switch.kind, Kind::Bluetooth);
        assert_eq!(event(0, 5, 3, false, true).op, Op::Other);
    }

    #[test]
    fn table_follows_adds_changes_and_removals() {
        let mut switches = Switches::new();
        apply(
            &mut switches,
            [
                event(0, 2, OP_ADD, false, false),
                event(1, 1, OP_ADD, false, true),
            ],
        );
        assert_eq!(switches.len(), 2);
        assert!(switches[&1].blocked());

        apply(
            &mut switches,
            [
                event(0, 2, OP_CHANGE, true, false),
                event(1, 1, OP_DEL, false, false),
            ],
        );
        assert_eq!(
            switches.into_iter().collect::<Vec<_>>(),
            [(
                0,
                Switch {
                    kind: Kind::Bluetooth,
                    soft: true,
                    hard: false,
                }
            )]
        );
    }
}
