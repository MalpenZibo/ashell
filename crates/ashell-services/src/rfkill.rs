//! Bluetooth soft block through the kernel rfkill interface.
//!
//! This talks to sysfs and `/dev/rfkill` directly instead of spawning the
//! `rfkill` binary: it lives in `/usr/sbin` on some distros, which is not in
//! a normal user's `PATH` there (openSUSE, ALT), and NixOS has no `/usr/sbin`
//! at all. The binary is not setuid, so writing `/dev/rfkill` ourselves needs
//! the same permissions it does: systemd gives the user of a seat write access
//! to the device node, on other systems the user has to be in the `rfkill`
//! group.

use anyhow::Context;
use futures::{
    StreamExt,
    stream::{BoxStream, pending},
};
use inotify::{Inotify, WatchMask};
use log::warn;
use std::io::ErrorKind;
use tokio::io::AsyncWriteExt;

const SYSFS_RFKILL: &str = "/sys/class/rfkill";
const DEV_RFKILL: &str = "/dev/rfkill";

// From linux/rfkill.h.
const RFKILL_TYPE_BLUETOOTH: u8 = 2;
const RFKILL_OP_CHANGE_ALL: u8 = 3;

/// Whether any Bluetooth rfkill switch is soft-blocked.
///
/// A system without rfkill support has nothing to block, so that counts as
/// unblocked, and so does a system whose state we can't read: this is only
/// used to tell whether the radios are off, never to change them.
pub async fn bluetooth_soft_blocked() -> bool {
    match read_bluetooth_soft_block().await {
        Ok(blocked) => blocked,
        Err(err) => {
            warn!("Can't read the rfkill state, assuming bluetooth is not soft blocked: {err:#}");
            false
        }
    }
}

/// Soft-blocks or unblocks every Bluetooth rfkill switch, like
/// `rfkill block|unblock bluetooth`.
///
/// The kernel also applies the new state to Bluetooth devices plugged in
/// later, exactly like the `rfkill` binary does.
pub async fn set_bluetooth_soft_block(blocked: bool) -> anyhow::Result<()> {
    let mut dev = tokio::fs::OpenOptions::new()
        .write(true)
        .open(DEV_RFKILL)
        .await
        .with_context(|| format!("open {DEV_RFKILL}"))?;
    dev.write_all(&bluetooth_event(blocked))
        .await
        .with_context(|| format!("write {DEV_RFKILL}"))?;
    // tokio's File writes in a background task: flush waits for it (and
    // reports its error) so the block lands before set_airplane_mode's wifi toggle.
    dev.flush()
        .await
        .with_context(|| format!("flush {DEV_RFKILL}"))?;

    Ok(())
}

/// Fires on every rfkill change. Never fails: without a watch, soft block
/// changes just aren't noticed until the next read.
pub fn soft_block_changes() -> BoxStream<'static, ()> {
    let watch = Inotify::init().and_then(|inotify| {
        inotify.watches().add(DEV_RFKILL, WatchMask::MODIFY)?;
        inotify.into_event_stream([0; 512])
    });

    match watch {
        Ok(events) => events.map(|_| {}).boxed(),
        Err(err) => {
            warn!("Can't watch {DEV_RFKILL}, disabling rfkill change notifications: {err}");
            pending().boxed()
        }
    }
}

async fn read_bluetooth_soft_block() -> anyhow::Result<bool> {
    let mut entries = match tokio::fs::read_dir(SYSFS_RFKILL).await {
        Ok(entries) => entries,
        // No rfkill support at all.
        Err(err) if err.kind() == ErrorKind::NotFound => return Ok(false),
        Err(err) => {
            return Err(anyhow::Error::new(err).context(format!("read {SYSFS_RFKILL}")));
        }
    };

    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        let Ok(kind) = tokio::fs::read_to_string(path.join("type")).await else {
            continue;
        };
        if kind.trim() != "bluetooth" {
            continue;
        }
        // The soft attribute is the current soft block, the hard one is a
        // switch we can't change anyway.
        if let Ok(soft) = tokio::fs::read_to_string(path.join("soft")).await
            && soft.trim() == "1"
        {
            return Ok(true);
        }
    }

    Ok(false)
}

// struct rfkill_event { u32 idx; u8 type; u8 op; u8 soft; u8 hard; }
// idx is ignored for RFKILL_OP_CHANGE_ALL, the kernel only changes the
// switches of the given type.
fn bluetooth_event(blocked: bool) -> [u8; 8] {
    [
        0,
        0,
        0,
        0,
        RFKILL_TYPE_BLUETOOTH,
        RFKILL_OP_CHANGE_ALL,
        u8::from(blocked),
        0,
    ]
}

#[cfg(test)]
mod tests {
    use super::bluetooth_event;

    #[test]
    fn block_event_matches_kernel_abi() {
        assert_eq!(bluetooth_event(true), [0, 0, 0, 0, 2, 3, 1, 0]);
    }

    #[test]
    fn unblock_event_matches_kernel_abi() {
        assert_eq!(bluetooth_event(false), [0, 0, 0, 0, 2, 3, 0, 0]);
    }
}
