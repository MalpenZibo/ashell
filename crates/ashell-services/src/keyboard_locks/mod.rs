//! Caps Lock, Num Lock and Scroll Lock, as the keyboards' LEDs show them.
//!
//! The LEDs' files under `/sys/class/leds` can be read by any user, while
//! the input devices would need the `input` group, which reads every
//! keystroke. The kernel tells nobody when a LED changes, so the files are
//! read again every [`POLL_INTERVAL`], and only while the seat is in use:
//! a lock changes with a key press, and polling an idle or locked session
//! would only wake the CPU up.

mod activity;

use crate::{stream::channel, udev_monitor};
use futures::{SinkExt, Stream, future::pending};
use log::warn;
use std::{
    fs::{self, File},
    os::unix::fs::FileExt,
    time::Duration,
};
use tokio::{
    io::unix::AsyncFd,
    sync::watch,
    time::{MissedTickBehavior, interval},
};

const LEDS_DIR: &str = "/sys/class/leds";
const POLL_INTERVAL: Duration = Duration::from_millis(250);

/// Each lock is `None` when no keyboard has its LED.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct KeyboardLocksData {
    pub caps_lock: Option<bool>,
    pub num_lock: Option<bool>,
    pub scroll_lock: Option<bool>,
}

impl KeyboardLocksData {
    fn read(leds: &[Led]) -> Self {
        let mut data = Self::default();
        for led in leds {
            let slot = match led.lock {
                Lock::Caps => &mut data.caps_lock,
                Lock::Num => &mut data.num_lock,
                Lock::Scroll => &mut data.scroll_lock,
            };
            let mut value = [0];
            // A keyboard unplugged since the last scan fails to read
            if led.brightness.read_at(&mut value, 0).is_ok() {
                *slot = Some(slot.unwrap_or(false) || value[0] != b'0');
            }
        }
        data
    }
}

enum Lock {
    Caps,
    Num,
    Scroll,
}

struct Led {
    lock: Lock,
    brightness: File,
}

#[derive(Debug, Clone)]
pub struct KeyboardLocks {
    _private: (),
}

impl KeyboardLocks {
    pub async fn connect() -> anyhow::Result<Self> {
        Ok(Self { _private: () })
    }

    /// Yields the current state first, then a snapshot after every change.
    pub fn updates(&self) -> impl Stream<Item = KeyboardLocksData> + Send + 'static {
        channel(16, |mut output| async move {
            // Listen before the first scan, so a keyboard added in between is not missed
            let mut monitor = udev_monitor::listen("leds")
                .inspect_err(|err| warn!("Failed to listen for keyboards: {err}"))
                .ok();
            let mut activity = Some(activity::watch());

            let mut leds = find_leds();
            let mut data = KeyboardLocksData::read(&leds);
            if output.send(data).await.is_err() {
                return;
            }

            let mut ticker = interval(POLL_INTERVAL);
            ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);
            loop {
                let active = activity.as_ref().is_none_or(|active| *active.borrow());
                tokio::select! {
                    _ = ticker.tick(), if active && !leds.is_empty() => {}
                    added_or_removed = keyboards_changed(monitor.as_mut()) => match added_or_removed {
                        Ok(true) => leds = find_leds(),
                        Ok(false) => continue,
                        Err(err) => {
                            warn!("Stopped listening for keyboards: {err}");
                            monitor = None;
                        }
                    },
                    // Its first tick would read again right away
                    _ = seat_activity(&mut activity) => ticker.reset(),
                }

                let next = KeyboardLocksData::read(&leds);
                if next != data {
                    data = next;
                    if output.send(data).await.is_err() {
                        return;
                    }
                }
            }
        })
    }
}

fn find_leds() -> Vec<Led> {
    let Ok(entries) = fs::read_dir(LEDS_DIR) else {
        return Vec::new();
    };
    entries
        .flatten()
        .filter_map(|entry| {
            let name = entry.file_name();
            let function = name.to_str()?.strip_prefix("input")?.split_once("::")?.1;
            let lock = match function {
                "capslock" => Lock::Caps,
                "numlock" => Lock::Num,
                "scrolllock" => Lock::Scroll,
                _ => return None,
            };
            Some(Led {
                lock,
                brightness: File::open(entry.path().join("brightness")).ok()?,
            })
        })
        .collect()
}

/// Whether a LED came or went. Never resolves without a monitor.
async fn keyboards_changed(
    monitor: Option<&mut AsyncFd<udev::MonitorSocket>>,
) -> std::io::Result<bool> {
    let Some(monitor) = monitor else {
        return pending().await;
    };
    let mut guard = monitor.readable_mut().await?;
    let changed = guard.get_inner().iter().any(|event| {
        matches!(
            event.event_type(),
            udev::EventType::Add | udev::EventType::Remove
        )
    });
    guard.clear_ready();
    Ok(changed)
}

/// Resolves when the seat goes idle or is used again. Once that can't be
/// followed anymore it never does, and the polling stays on.
async fn seat_activity(activity: &mut Option<watch::Receiver<bool>>) {
    let Some(receiver) = activity else {
        return pending().await;
    };
    if receiver.changed().await.is_err() {
        *activity = None;
    }
}
