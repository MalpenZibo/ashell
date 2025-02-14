use super::{ReadOnlyService, Service, ServiceEvent};
use iced::{
    futures::{channel::mpsc::Sender, stream::pending, SinkExt, StreamExt},
    stream::channel,
    Subscription, Task,
};
use log::{debug, error, info, warn};
use std::{
    any::TypeId,
    fs,
    ops::Deref,
    path::{Path, PathBuf},
};
use tokio::io::{unix::AsyncFd, Interest};
use zbus::proxy;

#[derive(Debug, Clone, Default)]
pub struct BrightnessData {
    pub device_name: String,
    pub current: u32,
    pub max: u32,
}

#[derive(Debug, Clone)]
pub struct BrightnessService {
    data: BrightnessData,
    conn: zbus::Connection,
}

impl Deref for BrightnessService {
    type Target = BrightnessData;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl BrightnessService {
    async fn get_max_brightness(device_path: &Path) -> anyhow::Result<u32> {
        let max_brightness = fs::read_to_string(device_path.join("max_brightness"))?;
        let max_brightness = max_brightness.trim().parse::<u32>()?;

        Ok(max_brightness)
    }

    async fn get_actual_brightness(device_path: &Path) -> anyhow::Result<u32> {
        let actual_brightness = fs::read_to_string(device_path.join("actual_brightness"))?;
        let actual_brightness = actual_brightness.trim().parse::<u32>()?;

        Ok(actual_brightness)
    }

    async fn initialize_data(device_path: &Path) -> anyhow::Result<BrightnessData> {
        let max_brightness = Self::get_max_brightness(device_path).await?;
        let actual_brightness = Self::get_actual_brightness(device_path).await?;

        debug!(
            "Max brightness: {}, current brightness: {}",
            max_brightness, actual_brightness
        );

        Ok(BrightnessData {
            device_name: device_path
                .iter()
                .last()
                .and_then(|d| d.to_str().map(|s| s.to_string()))
                .unwrap_or_default(),
            current: actual_brightness,
            max: max_brightness,
        })
    }

    async fn init_service() -> anyhow::Result<(zbus::Connection, PathBuf)> {
        let backlight_devices = Self::backlight_enumerate()?;

        if let Some(device) = backlight_devices
            .iter()
            .find(|d| d.subsystem().and_then(|s| s.to_str()) == Some("backlight"))
        {
            let device_path = device.syspath().to_path_buf();

            let conn = zbus::Connection::system().await?;

            Ok((conn, device_path))
        } else {
            warn!("No backlight devices found");
            Err(anyhow::anyhow!("No backlight devices found"))
        }
    }

    pub async fn backlight_monitor_listener() -> anyhow::Result<AsyncFd<udev::MonitorSocket>> {
        let socket = udev::MonitorBuilder::new()?
            .match_subsystem("backlight")?
            .listen()?;

        Ok(AsyncFd::with_interest(
            socket,
            Interest::READABLE | Interest::WRITABLE,
        )?)
    }

    fn backlight_enumerate() -> anyhow::Result<Vec<udev::Device>> {
        let mut enumerator = udev::Enumerator::new()?;
        enumerator.match_subsystem("backlight")?;
        Ok(enumerator.scan_devices()?.collect())
    }

    async fn start_listening(state: State, output: &mut Sender<ServiceEvent<Self>>) -> State {
        match state {
            State::Init => match Self::init_service().await {
                Ok((conn, device_path)) => {
                    let data = BrightnessService::initialize_data(&device_path).await;

                    match data {
                        Ok(data) => {
                            let _ = output
                                .send(ServiceEvent::Init(BrightnessService { data, conn }))
                                .await;

                            State::Active(device_path)
                        }
                        Err(err) => {
                            error!("Failed to initialize brightness data: {}", err);

                            State::Error
                        }
                    }
                }
                Err(err) => {
                    error!("Failed to access to brightness files: {}", err);

                    State::Error
                }
            },
            State::Active(device_path) => {
                info!("Listening for brightness events");
                let current_value = Self::get_actual_brightness(&device_path)
                    .await
                    .unwrap_or_default();

                match BrightnessService::backlight_monitor_listener().await {
                    Ok(mut socket) => {
                        loop {
                            if let Ok(mut socket) = socket.writable_mut().await {
                                for evt in socket.get_inner().iter() {
                                    debug!("{:?}: {:?}", evt.event_type(), evt.device());

                                    if evt.device().subsystem().and_then(|s| s.to_str())
                                        == Some("backlight")
                                    {
                                        match evt.event_type() {
                                            udev::EventType::Change => {
                                                debug!(
                                                    "Changed backlight device: {:?}",
                                                    evt.syspath()
                                                );
                                                let new_value =
                                                    Self::get_actual_brightness(&device_path)
                                                        .await
                                                        .unwrap_or_default();

                                                if new_value != current_value {
                                                    let _ = output
                                                        .send(ServiceEvent::Update(
                                                            BrightnessEvent(new_value),
                                                        ))
                                                        .await;
                                                }

                                                break;
                                            }
                                            _ => {
                                                debug!(
                                                    "Unhadled event type: {:?}",
                                                    evt.event_type()
                                                );
                                            }
                                        }
                                    }
                                }
                                socket.clear_ready();
                            } else {
                                warn!("Failed to get writable socket");
                                break;
                            }
                        }
                        State::Active(device_path)
                    }
                    Err(err) => {
                        error!("Failed to listen for brightness events: {}", err);

                        State::Error
                    }
                }
            }
            State::Error => {
                error!("Brightness service error");

                let _ = pending::<u8>().next().await;
                State::Error
            }
        }
    }

    async fn set_brightness(
        conn: &zbus::Connection,
        device: &str,
        value: u32,
    ) -> anyhow::Result<()> {
        let brightness_ctrl = BrightnessCtrlProxy::new(conn).await?;

        brightness_ctrl
            .set_brightness("backlight", device, value)
            .await?;

        Ok(())
    }
}

enum State {
    Init,
    Active(PathBuf),
    Error,
}

#[derive(Debug, Clone)]
pub struct BrightnessEvent(u32);

impl ReadOnlyService for BrightnessService {
    type UpdateEvent = BrightnessEvent;
    type Error = ();

    fn update(&mut self, event: Self::UpdateEvent) {
        self.data.current = event.0;
    }

    fn subscribe() -> Subscription<ServiceEvent<Self>> {
        let id = TypeId::of::<Self>();

        Subscription::run_with_id(
            id,
            channel(100, |mut output| async move {
                let mut state = State::Init;

                loop {
                    state = BrightnessService::start_listening(state, &mut output).await;
                }
            }),
        )
    }
}

#[derive(Debug, Clone)]
pub enum BrightnessCommand {
    Set(u32),
}

impl Service for BrightnessService {
    type Command = BrightnessCommand;

    fn command(&mut self, command: Self::Command) -> Task<ServiceEvent<Self>> {
        Task::perform(
            {
                let conn = self.conn.clone();
                let device_name = self.device_name.clone();

                async move {
                    match command {
                        BrightnessCommand::Set(v) => {
                            debug!("Setting brightness to {}", v);
                            let _ = BrightnessService::set_brightness(&conn, &device_name, v).await;

                            v
                        }
                    }
                }
            },
            |v| ServiceEvent::Update(BrightnessEvent(v)),
        )
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
