use super::{ReadOnlyService, Service, ServiceEvent};
use iced::{
    futures::{channel::mpsc::Sender, stream::pending, SinkExt, Stream, StreamExt},
    subscription::channel,
    Command,
};
use inotify::{Inotify, WatchMask};
use log::{debug, error, info, warn};
use std::{
    any::TypeId,
    fs,
    ops::Deref,
    path::{Path, PathBuf},
};
use zbus::proxy;

const DEVICES_FOLDER: &str = "/sys/class/backlight";

#[derive(Debug, Clone, Default)]
pub struct BrightnessData {
    pub current: u32,
    pub max: u32,
}

#[derive(Debug, Clone)]
pub struct BrightnessService {
    data: BrightnessData,
    device_name: String,
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
            current: actual_brightness * 100 / max_brightness,
            max: max_brightness,
        })
    }

    async fn init_service() -> anyhow::Result<(zbus::Connection, String, PathBuf)> {
        let device_folder = fs::read_dir(DEVICES_FOLDER)
            .ok()
            .and_then(|mut d| d.next().and_then(|entry| entry.ok()));

        if let Some(device_folder) = device_folder {
            let device_name = device_folder.file_name().into_string().unwrap();

            let conn = zbus::Connection::system().await?;

            Ok((conn, device_name, device_folder.path()))
        } else {
            warn!("No backlight devices found");
            Err(anyhow::anyhow!("No backlight devices found"))
        }
    }

    async fn events(device_path: &Path) -> anyhow::Result<impl Stream<Item = BrightnessEvent>> {
        let actual_brightness_file = device_path.join("actual_brightness");
        let inotify = Inotify::init()?;

        inotify
            .watches()
            .add(&actual_brightness_file, WatchMask::MODIFY)?;

        let buffer = [0; 512];
        let current_value = Self::get_actual_brightness(device_path).await?;

        Ok(inotify
            .into_event_stream(buffer)?
            .filter_map({
                let device_path = device_path.to_owned();
                move |_| {
                    let device_path = device_path.clone();
                    async move {
                        let new_value = Self::get_actual_brightness(&device_path)
                            .await
                            .unwrap_or_default();

                        if new_value != current_value {
                            Some(BrightnessEvent(new_value))
                        } else {
                            None
                        }
                    }
                }
            })
            .boxed())
    }

    async fn start_listening(state: State, output: &mut Sender<ServiceEvent<Self>>) -> State {
        match state {
            State::Init => match Self::init_service().await {
                Ok((conn, device_name, device_path)) => {
                    let data = BrightnessService::initialize_data(&device_path).await;

                    match data {
                        Ok(data) => {
                            let _ = output
                                .send(ServiceEvent::Init(BrightnessService {
                                    data,
                                    device_name,
                                    conn,
                                }))
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

                match BrightnessService::events(&device_path).await {
                    Ok(mut events) => {
                        while let Some(event) = events.next().await {
                            let _ = output.send(ServiceEvent::Update(event)).await;
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

    fn subscribe() -> iced::Subscription<ServiceEvent<Self>> {
        let id = TypeId::of::<Self>();

        channel(id, 100, |mut output| async move {
            let mut state = State::Init;

            loop {
                state = BrightnessService::start_listening(state, &mut output).await;
            }
        })
    }
}

#[derive(Debug, Clone)]
pub enum BrightnessCommand {
    Set(u32),
}

impl Service for BrightnessService {
    type Command = BrightnessCommand;

    fn command(&mut self, command: Self::Command) -> Command<ServiceEvent<Self>> {
        iced::Command::perform(
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
