use crate::stream::channel;
use dbus::BrightnessCtrlProxy;
use futures::{SinkExt, Stream};
use log::{debug, error, info, warn};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio::io::{Interest, unix::AsyncFd};

mod dbus;

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
    device_path: Arc<PathBuf>,
}

impl Brightness {
    /// Fails when there is no backlight device.
    pub async fn connect() -> anyhow::Result<Self> {
        let backlight_devices = backlight_enumerate()?;

        match backlight_devices
            .iter()
            .find(|d| d.subsystem().and_then(|s| s.to_str()) == Some("backlight"))
        {
            Some(device) => {
                let device_path = device.syspath().to_path_buf();

                let conn = zbus::Connection::system().await?;

                Ok(Self {
                    conn,
                    device_path: Arc::new(device_path),
                })
            }
            _ => {
                warn!("No backlight devices found");
                Err(anyhow::anyhow!("No backlight devices found"))
            }
        }
    }

    /// Yields the current state first, then a snapshot whenever the backlight changes.
    pub fn updates(&self) -> impl Stream<Item = BrightnessData> + Send + 'static {
        let device_path = self.device_path.clone();

        channel(100, |mut output| async move {
            let mut data = match initialize_data(&device_path) {
                Ok(data) => data,
                Err(err) => {
                    error!("Failed to initialize brightness data: {err}");
                    return;
                }
            };
            if output.send(data).await.is_err() {
                return;
            }

            loop {
                info!("Listening for brightness events");
                data.current = get_brightness(&device_path).unwrap_or_default();

                let mut socket = match backlight_monitor_listener() {
                    Ok(socket) => socket,
                    Err(err) => {
                        error!("Failed to listen for brightness events: {err}");
                        return;
                    }
                };

                loop {
                    debug!("Waiting for brightness events");

                    match socket.readable_mut().await {
                        Ok(mut socket) => {
                            for evt in socket.get_inner().iter() {
                                debug!("{:?}: {:?}", evt.event_type(), evt.device());

                                if evt.device().subsystem().and_then(|s| s.to_str())
                                    == Some("backlight")
                                {
                                    match evt.event_type() {
                                        udev::EventType::Change => {
                                            debug!("Changed backlight device: {:?}", evt.syspath());
                                            if let Ok(new_value) = get_brightness(&device_path)
                                                && new_value != data.current
                                            {
                                                data.current = new_value;
                                                if output.send(data).await.is_err() {
                                                    return;
                                                }
                                            }

                                            break;
                                        }
                                        _ => {
                                            debug!("Unhadled event type: {:?}", evt.event_type());
                                        }
                                    }
                                }
                            }
                            socket.clear_ready();
                        }
                        _ => {
                            warn!("Failed to get readable socket");
                            break;
                        }
                    }
                }
            }
        })
    }

    pub async fn execute(&self, command: BrightnessCommand) -> anyhow::Result<()> {
        let brightness_ctrl = BrightnessCtrlProxy::new(&self.conn).await?;
        let device_name = self
            .device_path
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
        get_brightness(&self.device_path)
    }
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

fn backlight_enumerate() -> anyhow::Result<Vec<udev::Device>> {
    let mut enumerator = udev::Enumerator::new()?;
    enumerator.match_subsystem("backlight")?;

    Ok(enumerator.scan_devices()?.collect())
}
