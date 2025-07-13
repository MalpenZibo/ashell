use super::{ReadOnlyService, Service, ServiceEvent};
use dbus::{BatteryProxy, BluetoothDbus};
use iced::{
    Subscription, Task,
    futures::{
        SinkExt, Stream, StreamExt,
        channel::mpsc::Sender,
        stream::{pending, select_all},
        stream_select,
    },
    stream::channel,
};
use inotify::{Inotify, WatchMask};
use log::{debug, error, info};
use std::{any::TypeId, ops::Deref};
use tokio::process::Command;
use zbus::zvariant::OwnedObjectPath;

mod dbus;

#[derive(PartialEq, Eq, Debug, Clone)]
pub enum BluetoothState {
    Unavailable,
    Active,
    Inactive,
}

#[derive(Debug, Clone)]
pub struct BluetoothDevice {
    pub name: String,
    pub battery: Option<u8>,
    pub path: OwnedObjectPath,
}

#[derive(Debug, Clone)]
pub struct BluetoothData {
    pub state: BluetoothState,
    pub devices: Vec<BluetoothDevice>,
}

#[derive(Debug, Clone)]
pub struct BluetoothService {
    conn: zbus::Connection,
    data: BluetoothData,
}

impl Deref for BluetoothService {
    type Target = BluetoothData;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

#[derive(Debug, Clone)]
pub enum BluetoothCommand {
    Toggle,
}

enum State {
    Init,
    Active(zbus::Connection),
    Error,
}

impl BluetoothService {
    async fn initialize_data(conn: &zbus::Connection) -> anyhow::Result<BluetoothData> {
        let bluetooth = BluetoothDbus::new(conn).await?;

        let state = bluetooth.state().await?;
        let rfkill_soft_block = BluetoothService::check_rfkill_soft_block().await?;

        let state = match state {
            BluetoothState::Unavailable => BluetoothState::Unavailable,
            BluetoothState::Active if rfkill_soft_block => BluetoothState::Inactive,
            state => state,
        };
        let devices = bluetooth.devices().await?;

        Ok(BluetoothData { state, devices })
    }

    async fn events(conn: &zbus::Connection) -> anyhow::Result<impl Stream<Item = ()> + use<>> {
        let bluetooth = BluetoothDbus::new(conn).await?;

        let interface_changed = stream_select!(
            bluetooth
                .bluez
                .receive_interfaces_added()
                .await?
                .map(|_| {}),
            bluetooth
                .bluez
                .receive_interfaces_removed()
                .await?
                .map(|_| {}),
        )
        .boxed();

        let combined = match bluetooth.adapter.as_ref() {
            Some(adapter) => {
                let powered = adapter.receive_powered_changed().await.map(|_| {});
                let rfkill = BluetoothService::listen_rfkill_soft_block_changes().await?;
                let devices = bluetooth.devices().await?;

                let mut batteries = Vec::with_capacity(devices.len());
                for device in devices {
                    let battery = BatteryProxy::builder(bluetooth.bluez.inner().connection())
                        .path(device.path)?
                        .build()
                        .await?;
                    batteries.push(battery.receive_percentage_changed().await.map(|_| {}));
                }

                stream_select!(interface_changed, powered, rfkill, select_all(batteries)).boxed()
            }
            _ => interface_changed,
        };

        Ok(combined)
    }

    async fn start_listening(state: State, output: &mut Sender<ServiceEvent<Self>>) -> State {
        match state {
            State::Init => match zbus::Connection::system().await {
                Ok(conn) => {
                    let data = BluetoothService::initialize_data(&conn).await;

                    match data {
                        Ok(data) => {
                            info!("Bluetooth service initialized");

                            let _ = output
                                .send(ServiceEvent::Init(BluetoothService {
                                    data,
                                    conn: conn.clone(),
                                }))
                                .await;

                            State::Active(conn)
                        }
                        Err(err) => {
                            error!("Failed to initialize bluetooth service: {err}");

                            State::Error
                        }
                    }
                }
                Err(err) => {
                    error!("Failed to connect to system bus: {err}");

                    State::Error
                }
            },
            State::Active(conn) => {
                info!("Listening for bluetooth events");

                match BluetoothService::events(&conn).await {
                    Ok(mut events) => {
                        while events.next().await.is_some() {
                            if let Ok(data) = BluetoothService::initialize_data(&conn).await {
                                let _ = output.send(ServiceEvent::Update(data)).await;
                            }
                        }

                        State::Active(conn)
                    }
                    Err(err) => {
                        error!("Failed to listen for bluetooth events: {err}");
                        State::Error
                    }
                }
            }
            State::Error => {
                error!("Bluetooth service error");

                let _ = pending::<u8>().next().await;
                State::Error
            }
        }
    }

    pub async fn check_rfkill_soft_block() -> anyhow::Result<bool> {
        let output = Command::new("rfkill")
            .arg("list")
            .arg("bluetooth")
            .output()
            .await?;

        let output = String::from_utf8(output.stdout)?;

        Ok(output.contains("Soft blocked: yes"))
    }

    pub async fn listen_rfkill_soft_block_changes() -> anyhow::Result<impl Stream<Item = ()>> {
        let inotify = Inotify::init()?;

        inotify.watches().add("/dev/rfkill", WatchMask::MODIFY)?;

        let buffer = [0; 512];
        Ok(inotify.into_event_stream(buffer)?.map(|_| {}))
    }

    async fn toggle_power(conn: &zbus::Connection, power: bool) -> anyhow::Result<()> {
        let bluetooth = BluetoothDbus::new(conn).await?;

        bluetooth.set_powered(power).await?;

        Ok(())
    }
}

impl ReadOnlyService for BluetoothService {
    type UpdateEvent = BluetoothData;
    type Error = ();

    fn update(&mut self, event: Self::UpdateEvent) {
        self.data = event;
    }

    fn subscribe() -> Subscription<ServiceEvent<Self>> {
        let id = TypeId::of::<Self>();

        Subscription::run_with_id(
            id,
            channel(100, async |mut output| {
                let mut state = State::Init;

                loop {
                    state = BluetoothService::start_listening(state, &mut output).await;
                }
            }),
        )
    }
}

impl Service for BluetoothService {
    type Command = BluetoothCommand;

    fn command(&mut self, command: Self::Command) -> Task<ServiceEvent<Self>> {
        match command {
            BluetoothCommand::Toggle => {
                let conn = self.conn.clone();

                if self.data.state == BluetoothState::Unavailable {
                    Task::none()
                } else {
                    let mut data = self.data.clone();

                    Task::perform(
                        async move {
                            let powered = data.state == BluetoothState::Active;
                            debug!("Toggling bluetooth power to: {}", !powered);
                            let res = BluetoothService::toggle_power(&conn, !powered).await;

                            if res.is_ok() {
                                data.state = if powered {
                                    BluetoothState::Inactive
                                } else {
                                    BluetoothState::Active
                                }
                            }

                            data
                        },
                        ServiceEvent::Update,
                    )
                }
            }
        }
    }
}
