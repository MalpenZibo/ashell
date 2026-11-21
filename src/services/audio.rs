use super::{ReadOnlyService, Service, ServiceEvent};
use crate::utils::remote_value::Remote;
use ashell_services::audio::Audio;
use iced::{
    Subscription, Task,
    futures::{SinkExt, StreamExt},
    stream::channel,
};
use log::error;
use std::{any::TypeId, ops::Deref, pin::pin};

pub use ashell_services::audio::{
    AudioCommand, AudioData, Device, NORMAL_VOLUME, Port, PortKind, Route,
};

#[derive(Debug, Clone)]
pub struct AudioService {
    data: AudioData,
    handle: Audio,
    /// The volumes the sliders show: what the user asks for while dragging,
    /// the server's once it answers.
    pub sink_slider: Remote<u32>,
    pub source_slider: Remote<u32>,
}

impl Deref for AudioService {
    type Target = AudioData;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

/// A muted device's slider rests at zero.
fn slider_volume(device: Option<&Device>) -> u32 {
    device
        .filter(|device| !device.is_mute)
        .map_or(0, |device| device.volume)
}

impl ReadOnlyService for AudioService {
    type UpdateEvent = AudioData;
    type Error = ();

    fn update(&mut self, data: Self::UpdateEvent) {
        self.data = data;
        self.sink_slider
            .receive(slider_volume(self.data.active_sink()));
        self.source_slider
            .receive(slider_volume(self.data.active_source()));
    }

    fn subscribe() -> Subscription<ServiceEvent<Self>> {
        Subscription::run_with(TypeId::of::<Self>(), |_| {
            channel(100, async |mut output| {
                let handle = match Audio::connect().await {
                    Ok(handle) => handle,
                    Err(err) => {
                        error!("Failed to start the audio service: {err}");
                        return;
                    }
                };

                let mut updates = pin!(handle.updates());
                let Some(data) = updates.next().await else {
                    return;
                };
                let mut service = AudioService {
                    data: AudioData::default(),
                    handle,
                    sink_slider: Remote::default(),
                    source_slider: Remote::default(),
                };
                service.update(data);
                let _ = output.send(ServiceEvent::Init(service)).await;

                while let Some(data) = updates.next().await {
                    let _ = output.send(ServiceEvent::Update(data)).await;
                }
            })
        })
    }
}

impl Service for AudioService {
    type Command = AudioCommand;

    fn command(&mut self, command: Self::Command) -> Task<ServiceEvent<Self>> {
        if let Err(err) = self.handle.execute(command) {
            error!("Failed to send an audio command: {err}");
        }
        Task::none()
    }
}
