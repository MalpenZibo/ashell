use crate::stream::channel;
use dbus::BrightnessCtrlProxy;
use futures::{SinkExt, Stream, channel::mpsc::Sender};
use log::{debug, info, warn};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::io::{Interest, unix::AsyncFd};

mod dbus;

const RETRY_DELAY: Duration = Duration::from_secs(5);

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct BrightnessData {
    pub current: u32,
    pub max: u32,
}

/// Set the backlight to this raw value, between 0 and [`BrightnessData::max`].
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct BrightnessCommand(pub u32);

#[derive(Debug, Clone)]
pub struct Brightness {
    conn: zbus::Connection,
    device_path: Arc<Mutex<Option<PathBuf>>>,
}

impl Brightness {
    pub async fn connect() -> anyhow::Result<Self> {
        let conn = zbus::Connection::system().await?;

        Ok(Self {
            conn,
            device_path: Arc::default(),
        })
    }

    /// Yields the current state first, then a snapshot whenever it changes.
    /// `None` while there is no backlight device: the stream waits for one to
    /// appear, and retries after errors.
    pub fn updates(&self) -> impl Stream<Item = Option<BrightnessData>> + Send + 'static {
        let device_path = self.device_path.clone();

        channel(100, |mut output| async move {
            let mut last = None;

            loop {
                let mut socket = match backlight_monitor_listener() {
                    Ok(socket) => socket,
                    Err(err) => {
                        warn!("Failed to listen for brightness events: {err}");
                        tokio::time::sleep(RETRY_DELAY).await;
                        continue;
                    }
                };

                // Enumerate once the monitor listens, so a device added in between is not missed
                let device = match find_backlight() {
                    Ok(device) => device,
                    Err(err) => {
                        warn!("Failed to enumerate backlight devices: {err}");
                        tokio::time::sleep(RETRY_DELAY).await;
                        continue;
                    }
                };
                if let Ok(mut slot) = device_path.lock() {
                    slot.clone_from(&device);
                }

                let Some(device) = device else {
                    if last != Some(None) {
                        info!("No backlight devices found, waiting for one");
                    }
                    if !send(&mut output, &mut last, None).await {
                        return;
                    }
                    wait_for_backlight(&mut socket).await;
                    continue;
                };

                let data = match initialize_data(&device) {
                    Ok(data) => data,
                    Err(err) => {
                        warn!("Failed to read brightness data: {err}");
                        tokio::time::sleep(RETRY_DELAY).await;
                        continue;
                    }
                };
                if !send(&mut output, &mut last, Some(data)).await {
                    return;
                }

                info!("Listening for brightness events");
                if !listen(&mut socket, &device, data, &mut output, &mut last).await {
                    return;
                }
            }
        })
    }

    pub async fn execute(&self, command: BrightnessCommand) -> anyhow::Result<()> {
        let device_path = self
            .device_path()
            .ok_or_else(|| anyhow::anyhow!("No backlight device"))?;
        let brightness_ctrl = BrightnessCtrlProxy::new(&self.conn).await?;
        let device_name = device_path
            .iter()
            .next_back()
            .and_then(|d| d.to_str())
            .unwrap_or_default();

        brightness_ctrl
            .set_brightness("backlight", device_name, command.0)
            .await?;

        Ok(())
    }

    /// Reads the current value straight from the device, to resync outside of
    /// [`updates`](Self::updates).
    pub fn brightness(&self) -> anyhow::Result<u32> {
        let device_path = self
            .device_path()
            .ok_or_else(|| anyhow::anyhow!("No backlight device"))?;

        get_brightness(&device_path)
    }

    fn device_path(&self) -> Option<PathBuf> {
        self.device_path.lock().ok().and_then(|slot| slot.clone())
    }
}

async fn send(
    output: &mut Sender<Option<BrightnessData>>,
    last: &mut Option<Option<BrightnessData>>,
    data: Option<BrightnessData>,
) -> bool {
    if *last == Some(data) {
        return true;
    }
    *last = Some(data);
    output.send(data).await.is_ok()
}

/// Follows `device` until it goes away or the monitor fails. Returns false
/// once nobody listens to `output`.
async fn listen(
    socket: &mut AsyncFd<udev::MonitorSocket>,
    device: &Path,
    mut data: BrightnessData,
    output: &mut Sender<Option<BrightnessData>>,
    last: &mut Option<Option<BrightnessData>>,
) -> bool {
    loop {
        debug!("Waiting for brightness events");

        let mut guard = match socket.readable_mut().await {
            Ok(guard) => guard,
            Err(err) => {
                warn!("Failed to get readable socket: {err}");
                tokio::time::sleep(RETRY_DELAY).await;
                return true;
            }
        };

        let mut changed = false;
        let mut removed = false;
        for evt in guard.get_inner().iter() {
            debug!("{:?}: {:?}", evt.event_type(), evt.device());

            if !is_backlight(&evt) {
                continue;
            }
            match evt.event_type() {
                udev::EventType::Change => {
                    debug!("Changed backlight device: {:?}", evt.syspath());
                    changed = true;
                }
                udev::EventType::Remove if evt.syspath() == device => removed = true,
                _ => debug!("Unhandled event type: {:?}", evt.event_type()),
            }
        }
        guard.clear_ready();

        if removed {
            info!("Backlight device removed: {device:?}");
            return true;
        }

        if changed
            && let Ok(current) = get_brightness(device)
            && current != data.current
        {
            data.current = current;
            if !send(output, last, Some(data)).await {
                return false;
            }
        }
    }
}

async fn wait_for_backlight(socket: &mut AsyncFd<udev::MonitorSocket>) {
    loop {
        let mut guard = match socket.readable_mut().await {
            Ok(guard) => guard,
            Err(err) => {
                warn!("Failed to get readable socket: {err}");
                tokio::time::sleep(RETRY_DELAY).await;
                return;
            }
        };

        let mut added = false;
        for evt in guard.get_inner().iter() {
            if is_backlight(&evt) && evt.event_type() == udev::EventType::Add {
                added = true;
            }
        }
        guard.clear_ready();

        if added {
            return;
        }
    }
}

fn is_backlight(evt: &udev::Event) -> bool {
    evt.device().subsystem().and_then(|s| s.to_str()) == Some("backlight")
}

fn find_backlight() -> anyhow::Result<Option<PathBuf>> {
    let mut enumerator = udev::Enumerator::new()?;
    enumerator.match_subsystem("backlight")?;

    Ok(enumerator
        .scan_devices()?
        .find(|d| d.subsystem().and_then(|s| s.to_str()) == Some("backlight"))
        .map(|d| d.syspath().to_path_buf()))
}

fn get_max_brightness(device_path: &Path) -> anyhow::Result<u32> {
    let max_brightness = fs::read_to_string(device_path.join("max_brightness"))?;
    let max_brightness = max_brightness.trim().parse::<u32>()?;

    Ok(max_brightness)
}

fn get_brightness(device_path: &Path) -> anyhow::Result<u32> {
    let brightness = fs::read_to_string(device_path.join("brightness"))?;
    let brightness = brightness.trim().parse::<u32>()?;
    Ok(brightness)
}

fn initialize_data(device_path: &Path) -> anyhow::Result<BrightnessData> {
    let max_brightness = get_max_brightness(device_path)?;
    let actual_brightness = get_brightness(device_path)?;
    Ok(BrightnessData {
        current: actual_brightness,
        max: max_brightness,
    })
}

fn backlight_monitor_listener() -> anyhow::Result<AsyncFd<udev::MonitorSocket>> {
    let socket = udev::MonitorBuilder::new()?
        .match_subsystem("backlight")?
        .listen()?;

    // SAFETY: the socket owns its udev monitor fd until it is dropped,
    // and `as_raw_fd` always returns that same fd.
    Ok(unsafe { AsyncFd::register_with_interest(socket, Interest::READABLE) }?)
}
