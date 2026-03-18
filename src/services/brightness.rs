use super::{ReadOnlyService, Service, ServiceEvent};
use crate::{services::throttle::ThrottleExt, utils::remote_value::Remote};
use iced::{
    Subscription, Task,
    futures::{SinkExt, StreamExt, channel::mpsc::Sender, stream::pending},
    stream::channel,
};
use log::{debug, error, info, warn};
use std::{
    any::TypeId,
    fs,
    ops::{Deref, DerefMut},
    path::{Path, PathBuf},
    time::Duration,
};
use tokio::{
    io::{Interest, unix::AsyncFd},
    sync::mpsc::{UnboundedReceiver, UnboundedSender},
};
use tokio_stream::wrappers::UnboundedReceiverStream;
use zbus::proxy;

#[derive(Debug, Clone, Default)]
pub struct BrightnessData {
    pub current: Remote<u32>,
    pub max: u32,
}

#[derive(Debug, Clone)]
pub struct BrightnessService {
    data: BrightnessData,
    commander: UnboundedSender<BrightnessCommand>,
}

impl Deref for BrightnessService {
    type Target = BrightnessData;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl DerefMut for BrightnessService {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}

impl BrightnessService {
    async fn get_max_brightness(device_path: &Path) -> anyhow::Result<u32> {
        let max_brightness = fs::read_to_string(device_path.join("max_brightness"))?;
        let max_brightness = max_brightness.trim().parse::<u32>()?;

        Ok(max_brightness)
    }

    async fn get_brightness(device_path: &Path) -> anyhow::Result<u32> {
        let brightness = fs::read_to_string(device_path.join("brightness"))?;
        let brightness = brightness.trim().parse::<u32>()?;
        Ok(brightness)
    }

    async fn initialize_data(device_path: &Path) -> anyhow::Result<BrightnessData> {
        let max_brightness = Self::get_max_brightness(device_path).await?;
        let actual_brightness = Self::get_brightness(device_path).await?;
        Ok(BrightnessData {
            current: Remote::new(actual_brightness),
            max: max_brightness,
        })
    }

    async fn init_service() -> anyhow::Result<(zbus::Connection, PathBuf)> {
        let backlight_devices = Self::backlight_enumerate()?;

        match backlight_devices
            .iter()
            .find(|d| d.subsystem().and_then(|s| s.to_str()) == Some("backlight"))
        {
            Some(device) => {
                let device_path = device.syspath().to_path_buf();

                let conn = zbus::Connection::system().await?;

                Ok((conn, device_path))
            }
            _ => {
                warn!("No backlight devices found");
                Err(anyhow::anyhow!("No backlight devices found"))
            }
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

    fn start_commander(
        conn: zbus::Connection,
        device_path: PathBuf,
        to_server_rx: UnboundedReceiver<BrightnessCommand>,
    ) {
        tokio::spawn(async move {
            let mut stream =
                UnboundedReceiverStream::new(to_server_rx).throttle(Duration::from_millis(100));
            while let Some(cmd) = stream.next().await {
                let _ = BrightnessService::set_brightness(&conn, &device_path, cmd.0).await;
            }
        });
    }

    async fn start_listening(state: State, output: &mut Sender<ServiceEvent<Self>>) -> State {
        match state {
            State::Init => match Self::init_service().await {
                Ok((conn, device_path)) => {
                    let data = BrightnessService::initialize_data(&device_path).await;

                    match data {
                        Ok(data) => {
                            let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
                            Self::start_commander(conn.clone(), device_path.clone(), rx);
                            let _ = output
                                .send(ServiceEvent::Init(BrightnessService {
                                    data,
                                    commander: tx,
                                }))
                                .await;

                            State::Active(device_path)
                        }
                        Err(err) => {
                            error!("Failed to initialize brightness data: {err}");

                            State::Error
                        }
                    }
                }
                Err(err) => {
                    error!("Failed to access to brightness files: {err}");

                    State::Error
                }
            },
            State::Active(device_path) => {
                info!("Listening for brightness events");
                let current_value = Self::get_brightness(&device_path).await.unwrap_or_default();

                match BrightnessService::backlight_monitor_listener().await {
                    Ok(mut socket) => {
                        loop {
                            debug!("Waiting for brightness events");

                            match socket.writable_mut().await {
                                Ok(mut socket) => {
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
                                                        Self::get_brightness(&device_path)
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
                                }
                                _ => {
                                    warn!("Failed to get writable socket");
                                    break;
                                }
                            }
                        }
                        State::Active(device_path)
                    }
                    Err(err) => {
                        error!("Failed to listen for brightness events: {err}");

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
        self.data.current.receive(event.0);
    }

    fn subscribe() -> Subscription<ServiceEvent<Self>> {
        let id = TypeId::of::<Self>();

        Subscription::run_with_id(
            id,
            channel(100, async |mut output| {
                let mut state = State::Init;

                loop {
                    state = BrightnessService::start_listening(state, &mut output).await;
                }
            }),
        )
    }
}

#[derive(Debug, Clone)]
pub struct BrightnessCommand(pub u32);

impl Service for BrightnessService {
    type Command = BrightnessCommand;

    fn command(&mut self, command: Self::Command) -> Task<ServiceEvent<Self>> {
        let _ = self.commander.send(command);
        Task::none()
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
