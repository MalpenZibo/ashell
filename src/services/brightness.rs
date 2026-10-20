use super::{ReadOnlyService, Service, ServiceEvent};
use crate::utils::remote_value::Remote;
use ashell_services::{brightness::Brightness, throttle::ThrottleExt};
use iced::{
    Subscription, Task,
    futures::{SinkExt, StreamExt},
    stream::channel,
};
use log::error;
use std::{any::TypeId, pin::pin, time::Duration};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio_stream::wrappers::UnboundedReceiverStream;

pub use ashell_services::brightness::{BrightnessCommand, BrightnessData};

#[derive(Debug, Clone)]
pub struct BrightnessService {
    handle: Brightness,
    pub current: Remote<u32>,
    pub max: u32,
    commander: UnboundedSender<BrightnessCommand>,
}

impl BrightnessService {
    pub fn sync_brightness(&mut self) {
        if let Ok(value) = self.handle.brightness() {
            self.current.receive(value);
        }
    }

    // Slider drags send a value per frame: forward them to logind at most every 100ms
    fn start_commander(handle: Brightness, commands: UnboundedReceiver<BrightnessCommand>) {
        tokio::spawn(async move {
            let mut commands =
                UnboundedReceiverStream::new(commands).throttle(Duration::from_millis(100));
            while let Some(command) = commands.next().await {
                if let Err(err) = handle.execute(command).await {
                    error!("Failed to set brightness: {err}");
                }
            }
        });
    }
}

impl ReadOnlyService for BrightnessService {
    /// `None` when the backlight device went away.
    type UpdateEvent = Option<BrightnessData>;
    type Error = ();

    fn update(&mut self, event: Self::UpdateEvent) {
        if let Some(data) = event {
            self.max = data.max;
            self.current.receive(data.current);
        }
    }

    fn subscribe() -> Subscription<ServiceEvent<Self>> {
        Subscription::run_with(TypeId::of::<Self>(), |_| {
            channel(100, async |mut output| {
                let handle = match Brightness::connect().await {
                    Ok(handle) => handle,
                    Err(err) => {
                        error!("Failed to connect to system bus for brightness: {err}");
                        return;
                    }
                };

                let (commander, commands) = tokio::sync::mpsc::unbounded_channel();
                Self::start_commander(handle.clone(), commands);

                // A service exists only while there is a backlight device
                let mut present = false;
                let mut updates = pin!(handle.updates());
                while let Some(data) = updates.next().await {
                    let event = match data {
                        Some(data) if !present => ServiceEvent::Init(BrightnessService {
                            handle: handle.clone(),
                            current: Remote::new(data.current),
                            max: data.max,
                            commander: commander.clone(),
                        }),
                        None if !present => continue,
                        data => ServiceEvent::Update(data),
                    };
                    present = data.is_some();
                    let _ = output.send(event).await;
                }
            })
        })
    }
}

impl Service for BrightnessService {
    type Command = BrightnessCommand;

    fn command(&mut self, command: Self::Command) -> Task<ServiceEvent<Self>> {
        let _ = self.commander.send(command);
        Task::none()
    }
}
