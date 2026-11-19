//! Caps Lock, Num Lock and Scroll Lock, as the keyboards' LEDs show them.
//!
//! The LEDs' files under `/sys/class/leds` can be read by any user, while
//! the input devices would need the `input` group, which reads every
//! keystroke. The kernel tells nobody when a LED changes, so the files are
//! read again every [`POLL_INTERVAL`], and only while the seat is in use:
//! a lock changes with a key press, and polling an idle or locked session
//! would only wake the CPU up.

mod activity;

use crate::stream::channel;
use futures::{SinkExt, Stream, future::pending};
use log::{info, warn};
use std::{fs, path::PathBuf, time::Duration};
use tokio::{
    io::{Interest, unix::AsyncFd},
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
            // A keyboard unplugged since the last scan has no file anymore
            if let Ok(value) = fs::read_to_string(&led.brightness) {
                *slot = Some(slot.unwrap_or(false) || value.trim() != "0");
            }
        }
        data
    }
}

#[derive(Debug, Clone, Copy)]
enum Lock {
    Caps,
    Num,
    Scroll,
}

struct Led {
    lock: Lock,
    brightness: PathBuf,
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
            let mut monitor = leds_monitor()
                .inspect_err(|err| warn!("Failed to listen for keyboards: {err}"))
                .ok();
            let mut activity = activity::watch()
                .inspect_err(|err| info!("Keyboard locks are polled even when idle: {err}"))
                .ok();

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
                    _ = seat_activity(&mut activity) => {}
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
            let (device, function) = name.to_str()?.split_once("::")?;
            let lock = match function {
                "capslock" if device.starts_with("input") => Lock::Caps,
                "numlock" if device.starts_with("input") => Lock::Num,
                "scrolllock" if device.starts_with("input") => Lock::Scroll,
                _ => return None,
            };
            Some(Led {
                lock,
                brightness: entry.path().join("brightness"),
            })
        })
        .collect()
}

fn leds_monitor() -> anyhow::Result<AsyncFd<udev::MonitorSocket>> {
    let socket = udev::MonitorBuilder::new()?
        .match_subsystem("leds")?
        .listen()?;

    // SAFETY: the socket owns its udev monitor fd until it is dropped,
    // and `as_raw_fd` always returns that same fd.
    Ok(unsafe { AsyncFd::register_with_interest(socket, Interest::READABLE) }?)
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

/// Resolves when the seat goes idle or is used again. Without a Wayland
/// connection it never does, and a lost one leaves the polling on.
async fn seat_activity(activity: &mut Option<watch::Receiver<bool>>) {
    let Some(receiver) = activity else {
        return pending().await;
    };
    if receiver.changed().await.is_err() {
        *activity = None;
    }
}
