//! PulseAudio (or PipeWire's pulse server) state and commands. libpulse is not
//! async: the connection lives on its own thread, owned by [`Audio`], and
//! [`AudioControl`] sends commands to that same thread.

use itertools::Either;
use libpulse_binding::volume::Volume;
use log::{error, warn};
use pulse::{
    PulseAudioCommand, PulseAudioServer, PulseAudioServerEvent, PulseAudioServerHandle, WakePipe,
};
use std::{
    fmt,
    sync::{Arc, Mutex, PoisonError},
    time::Duration,
};

mod pulse;

pub use libpulse_binding::{def::DevicePortType, volume::ChannelVolumes};

const RETRY_DELAY: Duration = Duration::from_secs(5);

#[derive(Debug, Clone, PartialEq)]
pub struct Device {
    pub name: String,
    pub description: String,
    pub volume: ChannelVolumes,
    pub is_mute: bool,
    pub is_filter: bool,
    pub ports: Vec<Port>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Port {
    pub name: String,
    pub description: String,
    pub device_type: DevicePortType,
}

#[derive(Debug, Copy, Clone)]
pub struct Route<'a> {
    pub device: &'a Device,
    pub port: Option<&'a Port>,
}

impl<'a> fmt::Display for Route<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.port {
            // The node description already names the port when it is the only one,
            // so the prefix is only needed to tell apart ports of the same node
            Some(port) if self.device.ports.len() > 1 => {
                write!(f, "{}: {}", port.description, self.device.description)
            }
            _ => write!(f, "{}", self.device.description),
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ServerInfo {
    pub default_sink: String,
    pub default_source: String,
}

pub trait ChannelVolumesExt {
    fn get_volume(&self) -> u32;

    fn scaled(&self, max: u32) -> Option<ChannelVolumes>;
}

impl ChannelVolumesExt for ChannelVolumes {
    fn get_volume(&self) -> u32 {
        self.max().0
    }

    fn scaled(&self, max: u32) -> Option<ChannelVolumes> {
        let mut cv = *self; // Implicit copy
        if cv.scale(Volume(max)).is_some() {
            Some(cv)
        } else {
            error!("Failed scaling volume: {cv}");
            None
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct AudioData {
    pub server_info: ServerInfo,
    sinks: Vec<Device>,
    sources: Vec<Device>,
}

impl AudioData {
    pub fn apply(&mut self, event: AudioEvent) {
        match event {
            AudioEvent::Sinks(sinks) => self.sinks = sinks,
            AudioEvent::Sources(sources) => self.sources = sources,
            AudioEvent::ServerInfo(info) => self.server_info = info,
            AudioEvent::Error => {}
        }
    }

    /// The default sink's volume, 0 when it is muted or there is none.
    pub fn sink_volume(&self) -> u32 {
        Self::volume(self.active_sink())
    }

    /// The default source's volume, 0 when it is muted or there is none.
    pub fn source_volume(&self) -> u32 {
        Self::volume(self.active_source())
    }

    pub fn active_sink(&self) -> Option<&Device> {
        self.sinks
            .iter()
            .find(|device| device.name == self.server_info.default_sink)
    }

    pub fn active_source(&self) -> Option<&Device> {
        self.sources
            .iter()
            .find(|device| device.name == self.server_info.default_source)
    }

    pub fn has_multiple_sources(&self) -> bool {
        Self::route_iter(&self.sources).nth(1).is_some()
    }

    pub fn has_multiple_sinks(&self) -> bool {
        Self::route_iter(&self.sinks).nth(1).is_some()
    }

    pub fn source_iter(&self) -> impl Iterator<Item = Route<'_>> {
        Self::route_iter(&self.sources)
    }

    pub fn sink_iter(&self) -> impl Iterator<Item = Route<'_>> {
        Self::route_iter(&self.sinks)
    }

    fn volume(device: Option<&Device>) -> u32 {
        device
            .map(|device| {
                if device.is_mute {
                    0
                } else {
                    device.volume.get_volume()
                }
            })
            .unwrap_or_default()
    }

    /// Iterates over all audio routes which user should be able to select
    /// This includes devices with multiple available hardware ports as well as
    /// [smart-filters](https://pipewire.pages.freedesktop.org/wireplumber/policies/smart_filters.html)
    fn route_iter(devices: &[Device]) -> impl Iterator<Item = Route<'_>> {
        devices.iter().flat_map(|device| {
            if device.is_filter {
                if device.ports.len() > 1 {
                    let device_name = device.name.as_str();
                    warn!("Unexpected multiple ports in a filter node: {device_name}")
                }
                Either::Left(std::iter::once(Route { device, port: None }))
            } else if device.ports.is_empty() {
                Either::Left(std::iter::once(Route { device, port: None }))
            } else {
                Either::Right(device.ports.iter().map(move |port| Route {
                    device,
                    port: Some(port),
                }))
            }
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum AudioEvent {
    Sinks(Vec<Device>),
    Sources(Vec<Device>),
    ServerInfo(ServerInfo),
    /// The PulseAudio connection was lost; the listener should reconnect.
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AudioCommand {
    ToggleSinkMute,
    ToggleSourceMute,
    SinkVolume(u32),
    SourceVolume(u32),
    DefaultSink(String, Option<String>),
    DefaultSource(String, Option<String>),
}

#[derive(Debug, Clone)]
struct Commander {
    sender: tokio::sync::mpsc::UnboundedSender<PulseAudioCommand>,
    wake: WakePipe,
}

/// Commands are resolved against the latest snapshot and sent to the
/// connection owned by [`Audio`].
#[derive(Debug, Clone)]
pub struct AudioControl {
    commander: Commander,
    data: Arc<Mutex<AudioData>>,
}

impl AudioControl {
    /// Returns whether the command was sent to the PulseAudio thread.
    pub fn execute(&self, command: AudioCommand) -> bool {
        let data = self.data.lock().unwrap_or_else(PoisonError::into_inner);

        let sent = match command {
            AudioCommand::ToggleSinkMute => data.active_sink().is_some_and(|sink| {
                self.commander
                    .sender
                    .send(PulseAudioCommand::SinkMute(
                        sink.name.clone(),
                        !sink.is_mute,
                    ))
                    .is_ok()
            }),
            AudioCommand::ToggleSourceMute => data.active_source().is_some_and(|source| {
                self.commander
                    .sender
                    .send(PulseAudioCommand::SourceMute(
                        source.name.clone(),
                        !source.is_mute,
                    ))
                    .is_ok()
            }),
            AudioCommand::SinkVolume(volume) => data
                .active_sink()
                .and_then(|sink| {
                    sink.volume.scaled(volume).map(|volume| {
                        self.commander
                            .sender
                            .send(PulseAudioCommand::SinkVolume(sink.name.clone(), volume))
                            .is_ok()
                    })
                })
                .unwrap_or(false),
            AudioCommand::SourceVolume(volume) => data
                .active_source()
                .and_then(|source| {
                    source.volume.scaled(volume).map(|volume| {
                        self.commander
                            .sender
                            .send(PulseAudioCommand::SourceVolume(source.name.clone(), volume))
                            .is_ok()
                    })
                })
                .unwrap_or(false),
            AudioCommand::DefaultSink(name, port) => self
                .commander
                .sender
                .send(PulseAudioCommand::DefaultSink(name, port))
                .is_ok(),
            AudioCommand::DefaultSource(name, port) => self
                .commander
                .sender
                .send(PulseAudioCommand::DefaultSource(name, port))
                .is_ok(),
        };

        if sent {
            self.commander.wake.wake();
        }

        sent
    }
}

/// A connection to PulseAudio (or PipeWire's pulse server).
///
/// Owns the server thread and yields the changes it reports. Dropping it closes
/// the connection; call [`Audio::connect`] again to reconnect.
pub struct Audio {
    handle: PulseAudioServerHandle,
    data: Arc<Mutex<AudioData>>,
}

impl Audio {
    pub async fn connect() -> anyhow::Result<Self> {
        let handle = PulseAudioServer::start().await?;

        Ok(Self {
            handle,
            data: Arc::new(Mutex::new(AudioData::default())),
        })
    }

    /// A cloneable handle for sending commands to this connection.
    pub fn control(&self) -> AudioControl {
        AudioControl {
            commander: Commander {
                sender: self.handle.sender.clone(),
                wake: self.handle.wake.clone(),
            },
            data: self.data.clone(),
        }
    }

    /// The next change from PulseAudio. `None` means the connection is gone and
    /// the caller should reconnect.
    pub async fn next_event(&mut self) -> Option<AudioEvent> {
        let event = match self.handle.receiver.recv().await? {
            PulseAudioServerEvent::Error => AudioEvent::Error,
            PulseAudioServerEvent::Sinks(sinks) => AudioEvent::Sinks(sinks),
            PulseAudioServerEvent::Sources(sources) => AudioEvent::Sources(sources),
            PulseAudioServerEvent::ServerInfo(info) => AudioEvent::ServerInfo(info),
        };
        self.data
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .apply(event.clone());
        Some(event)
    }

    pub fn retry_delay() -> Duration {
        RETRY_DELAY
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn muted_devices_report_zero_volume() {
        let data = AudioData {
            server_info: ServerInfo {
                default_sink: "sink".into(),
                default_source: "source".into(),
            },
            sinks: vec![Device {
                name: "sink".into(),
                description: String::new(),
                volume: ChannelVolumes::default(),
                is_mute: true,
                is_filter: false,
                ports: vec![],
            }],
            sources: vec![],
        };

        assert_eq!(data.sink_volume(), 0);
    }

    #[test]
    fn apply_replaces_the_reported_list() {
        let mut data = AudioData::default();
        data.apply(AudioEvent::ServerInfo(ServerInfo {
            default_sink: "sink".into(),
            default_source: "source".into(),
        }));
        assert_eq!(data.server_info.default_sink, "sink");
        assert!(data.active_sink().is_none());
    }
}
