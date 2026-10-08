use futures::{
    StreamExt,
    stream::{BoxStream, pending},
};
use inotify::{Inotify, WatchMask};
use log::warn;
use std::io::ErrorKind;
use tokio::process::Command;

pub async fn bluetooth_soft_blocked() -> anyhow::Result<bool> {
    let output = match Command::new("rfkill")
        .args(["list", "bluetooth"])
        .output()
        .await
    {
        Ok(output) => output,
        Err(err) if err.kind() == ErrorKind::NotFound => {
            warn!("rfkill binary not found, assuming bluetooth is not soft blocked");
            return Ok(false);
        }
        Err(err) => return Err(err.into()),
    };

    let output = String::from_utf8(output.stdout)?;

    Ok(output.contains("Soft blocked: yes"))
}

/// Soft blocks or unblocks bluetooth, e.g. for airplane mode.
pub async fn set_bluetooth_soft_block(blocked: bool) -> anyhow::Result<()> {
    Command::new("rfkill")
        .args([if blocked { "block" } else { "unblock" }, "bluetooth"])
        .output()
        .await?;

    Ok(())
}

/// Fires on every rfkill change. Never fails: without a watch, soft block
/// changes just aren't noticed until the next read.
pub fn soft_block_changes() -> BoxStream<'static, ()> {
    let watch = Inotify::init().and_then(|inotify| {
        inotify.watches().add("/dev/rfkill", WatchMask::MODIFY)?;
        inotify.into_event_stream([0; 512])
    });

    match watch {
        Ok(events) => events.map(|_| {}).boxed(),
        Err(err) => {
            warn!("Can't watch /dev/rfkill, disabling rfkill change notifications: {err}");
            pending().boxed()
        }
    }
}
