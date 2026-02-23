use guido::prelude::*;
use log::{debug, error, warn};
use std::{
    fs,
    path::{Path, PathBuf},
};
use tokio::io::{Interest, unix::AsyncFd};
use zbus::proxy;

#[derive(Clone, Debug, PartialEq, guido::SignalFields)]
pub struct BrightnessData {
    pub current: u32,
    pub max: u32,
}

impl Default for BrightnessData {
    fn default() -> Self {
        Self { current: 0, max: 1 }
    }
}

#[derive(Clone)]
pub enum BrightnessCmd {
    Set(u32),
    Refresh,
}

pub fn create() -> (BrightnessDataSignals, Service<BrightnessCmd>) {
    let data = BrightnessDataSignals::new(BrightnessData::default());
    let svc = start_brightness_service(data.writers());
    (data, svc)
}

fn get_max_brightness(device_path: &Path) -> anyhow::Result<u32> {
    let max_brightness = fs::read_to_string(device_path.join("max_brightness"))?;
    Ok(max_brightness.trim().parse::<u32>()?)
}

fn get_actual_brightness(device_path: &Path) -> anyhow::Result<u32> {
    let actual_brightness = fs::read_to_string(device_path.join("actual_brightness"))?;
    Ok(actual_brightness.trim().parse::<u32>()?)
}

fn backlight_enumerate() -> anyhow::Result<Vec<udev::Device>> {
    let mut enumerator = udev::Enumerator::new()?;
    enumerator.match_subsystem("backlight")?;
    Ok(enumerator.scan_devices()?.collect())
}

async fn backlight_monitor_listener() -> anyhow::Result<AsyncFd<udev::MonitorSocket>> {
    let socket = udev::MonitorBuilder::new()?
        .match_subsystem("backlight")?
        .listen()?;
    Ok(AsyncFd::with_interest(
        socket,
        Interest::READABLE | Interest::WRITABLE,
    )?)
}

async fn set_brightness(
    conn: &zbus::Connection,
    device_path: &Path,
    value: u32,
) -> anyhow::Result<()> {
    let brightness_ctrl = BrightnessCtrlProxy::new(conn).await?;
    let device_name = device_path
        .iter()
        .next_back()
        .and_then(|d| d.to_str())
        .unwrap_or_default();
    brightness_ctrl
        .set_brightness("backlight", device_name, value)
        .await?;
    Ok(())
}

fn start_brightness_service(writers: BrightnessDataWriters) -> Service<BrightnessCmd> {
    create_service::<BrightnessCmd, _, _>(move |mut rx, ctx| async move {
        // Find backlight device
        let backlight_devices = match backlight_enumerate() {
            Ok(d) => d,
            Err(e) => {
                error!("Failed to enumerate backlight devices: {e}");
                return;
            }
        };

        let device = match backlight_devices
            .iter()
            .find(|d| d.subsystem().and_then(|s| s.to_str()) == Some("backlight"))
        {
            Some(d) => d,
            None => {
                warn!("No backlight devices found");
                return;
            }
        };

        let device_path = device.syspath().to_path_buf();

        let conn = match zbus::Connection::system().await {
            Ok(c) => c,
            Err(e) => {
                error!("Failed to connect to system bus: {e}");
                return;
            }
        };

        // Initialize brightness data
        let max = get_max_brightness(&device_path).unwrap_or(1);
        let current = get_actual_brightness(&device_path).unwrap_or(0);
        writers.set(BrightnessData { current, max });

        // Start monitoring
        let mut socket = match backlight_monitor_listener().await {
            Ok(s) => s,
            Err(e) => {
                error!("Failed to create backlight monitor: {e}");
                // Still handle commands even without monitoring
                while ctx.is_running() {
                    if let Some(cmd) = rx.recv().await {
                        handle_command(&conn, &device_path, &writers, cmd).await;
                    } else {
                        break;
                    }
                }
                return;
            }
        };

        while ctx.is_running() {
            tokio::select! {
                cmd = rx.recv() => {
                    match cmd {
                        Some(cmd) => handle_command(&conn, &device_path, &writers, cmd).await,
                        None => break,
                    }
                }
                result = socket.writable_mut() => {
                    if let Ok(mut guard) = result {
                        for evt in guard.get_inner().iter() {
                            if evt.device().subsystem().and_then(|s| s.to_str()) == Some("backlight") {
                                if let udev::EventType::Change = evt.event_type() {
                                    let new_value = get_actual_brightness(&device_path).unwrap_or(0);
                                    writers.current.set(new_value);
                                }
                            }
                        }
                        guard.clear_ready();
                    }
                }
            }
        }
    })
}

async fn handle_command(
    conn: &zbus::Connection,
    device_path: &Path,
    writers: &BrightnessDataWriters,
    cmd: BrightnessCmd,
) {
    match cmd {
        BrightnessCmd::Set(v) => {
            debug!("Setting brightness to {v}");
            let _ = set_brightness(conn, device_path, v).await;
        }
        BrightnessCmd::Refresh => {
            debug!("Refreshing brightness data");
            let current = get_actual_brightness(device_path).unwrap_or(0);
            writers.current.set(current);
        }
    }
}

#[proxy(
    default_service = "org.freedesktop.login1",
    default_path = "/org/freedesktop/login1/session/auto",
    interface = "org.freedesktop.login1.Session"
)]
trait BrightnessCtrl {
    fn set_brightness(&self, subsystem: &str, name: &str, value: u32) -> zbus::Result<()>;
}
