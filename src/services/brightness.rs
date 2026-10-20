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
    type UpdateEvent = BrightnessData;
    type Error = ();

    fn update(&mut self, event: Self::UpdateEvent) {
        self.max = event.max;
        self.current.receive(event.current);
    }

    fn subscribe() -> Subscription<ServiceEvent<Self>> {
        Subscription::run_with(TypeId::of::<Self>(), |_| {
            channel(100, async |mut output| {
                let handle = match Brightness::connect().await {
                    Ok(handle) => handle,
                    Err(err) => {
                        error!("Failed to access to brightness files: {err}");
                        return;
                    }
                };

                let mut updates = pin!(handle.updates());
                let Some(data) = updates.next().await else {
                    return;
                };

                let (commander, commands) = tokio::sync::mpsc::unbounded_channel();
                Self::start_commander(handle.clone(), commands);

                let service = BrightnessService {
                    handle,
                    current: Remote::new(data.current),
                    max: data.max,
                    commander,
                };
                let _ = output.send(ServiceEvent::Init(service)).await;

                while let Some(data) = updates.next().await {
                    let _ = output.send(ServiceEvent::Update(data)).await;
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
