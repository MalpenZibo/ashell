use futures::{
    Stream, StreamExt,
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

pub async fn soft_block_changes() -> anyhow::Result<impl Stream<Item = ()> + Send + use<>> {
    let inotify = Inotify::init()?;

    let changes: BoxStream<'static, ()> =
        match inotify.watches().add("/dev/rfkill", WatchMask::MODIFY) {
            Ok(_) => {
                let buffer = [0; 512];
                inotify.into_event_stream(buffer)?.map(|_| {}).boxed()
            }
            Err(err) if err.kind() == ErrorKind::NotFound => {
                warn!("/dev/rfkill not found, disabling rfkill change notifications");
                pending().boxed()
            }
            Err(err) => return Err(err.into()),
        };

    Ok(changes)
}
