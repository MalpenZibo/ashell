use super::{ReadOnlyService, Service, ServiceEvent};
use crate::utils::remote_value::Remote;
use ashell_services::audio::{Audio, AudioControl};
use iced::{Subscription, Task, futures::SinkExt, stream::channel};
use log::error;
use std::{
    any::TypeId,
    ops::{Deref, DerefMut},
};

pub use ashell_services::audio::{
    AudioCommand, AudioData, AudioEvent, ChannelVolumesExt, DevicePortType, Port,
};

#[derive(Debug, Clone)]
pub struct AudioService {
    data: AudioData,
    pub sink_slider: Remote<u32>,
    pub source_slider: Remote<u32>,
    control: AudioControl,
}

impl Deref for AudioService {
    type Target = AudioData;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl DerefMut for AudioService {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}

enum State {
    Init,
    Active(Audio),
    Error,
}

impl AudioService {
    fn new(control: AudioControl) -> Self {
        Self {
            data: AudioData::default(),
            sink_slider: Remote::new(0),
            source_slider: Remote::new(0),
            control,
        }
    }

    fn update_volumes(&mut self) {
        self.sink_slider.receive(self.data.sink_volume());
        self.source_slider.receive(self.data.source_volume());
    }

    async fn start_listening(
        state: State,
        output: &mut iced::futures::channel::mpsc::Sender<ServiceEvent<Self>>,
    ) -> State {
        match state {
            State::Init => match Audio::connect().await {
                Ok(audio) => {
                    let _ = output
                        .send(ServiceEvent::Init(AudioService::new(audio.control())))
                        .await;
                    State::Active(audio)
                }
                Err(err) => {
                    error!("Failed to initialize audio service: {err}");
                    State::Error
                }
            },
            State::Active(mut audio) => match audio.next_event().await {
                Some(AudioEvent::Error) | None => {
                    error!("Audio service error, reconnecting in 5 seconds");
                    State::Error
                }
                Some(event) => {
                    let _ = output.send(ServiceEvent::Update(event)).await;
                    State::Active(audio)
                }
            },
            State::Error => {
                tokio::time::sleep(Audio::retry_delay()).await;
                State::Init
            }
        }
    }
}

impl ReadOnlyService for AudioService {
    type UpdateEvent = AudioEvent;
    type Error = ();

    fn update(&mut self, event: Self::UpdateEvent) {
        self.data.apply(event);
        self.update_volumes();
    }

    fn subscribe() -> Subscription<ServiceEvent<Self>> {
        Subscription::run_with(TypeId::of::<Self>(), |_| {
            channel(100, async |mut output| {
                let mut state = State::Init;

                loop {
                    state = AudioService::start_listening(state, &mut output).await;
                }
            })
        })
    }
}

impl Service for AudioService {
    type Command = AudioCommand;

    fn command(&mut self, command: Self::Command) -> Task<ServiceEvent<Self>> {
        self.control.execute(command);
        Task::none()
    }
}
