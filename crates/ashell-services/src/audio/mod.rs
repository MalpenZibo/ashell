//! The PulseAudio server, PipeWire's included: its sinks and sources as
//! snapshots, and commands on the default ones.

mod pulse;

use crate::snapshot::snapshots;
use futures::Stream;
use std::{
    os::fd::{AsRawFd, FromRawFd, OwnedFd},
    sync::{Arc, mpsc},
    thread,
};
use tokio::sync::watch;

/// The volume PulseAudio calls 100%: above it the signal is amplified.
pub const NORMAL_VOLUME: u32 = 0x10000;

#[derive(Debug, Clone, PartialEq)]
pub struct Device {
    pub name: String,
    pub description: String,
    /// The loudest channel's.
    pub volume: u32,
    pub is_mute: bool,
    /// The available ones; none for a WirePlumber smart filter, which is
    /// chosen as a whole.
    pub ports: Vec<Port>,
    /// The name of the port in use.
    pub active_port: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Port {
    pub name: String,
    pub description: String,
    pub kind: PortKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PortKind {
    Unknown,
    Aux,
    Speaker,
    Headphones,
    Line,
    Mic,
    Headset,
    Handset,
    Earpiece,
    Spdif,
    Hdmi,
    Tv,
    Radio,
    Video,
    Usb,
    Bluetooth,
    Portable,
    Handsfree,
    Car,
    HiFi,
    Phone,
    Network,
    Analog,
}

/// What can be chosen as the default sink or source: a device, through one
/// of its ports when it has some.
#[derive(Debug, Copy, Clone)]
pub struct Route<'a> {
    pub device: &'a Device,
    pub port: Option<&'a Port>,
}

impl Route<'_> {
    /// Its port is the one its device uses, or it has none: with the device
    /// the default, it is the route in use.
    pub fn is_active(&self) -> bool {
        self.port
            .is_none_or(|port| self.device.active_port.as_deref() == Some(port.name.as_str()))
    }
}

/// The server's sinks and sources, the monitors left out. Empty while there
/// is no server to talk to.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct AudioData {
    pub default_sink: String,
    pub default_source: String,
    pub sinks: Vec<Device>,
    pub sources: Vec<Device>,
}

impl AudioData {
    pub fn active_sink(&self) -> Option<&Device> {
        self.sinks.iter().find(|d| d.name == self.default_sink)
    }

    pub fn active_source(&self) -> Option<&Device> {
        self.sources.iter().find(|d| d.name == self.default_source)
    }

    pub fn sink_routes(&self) -> impl Iterator<Item = Route<'_>> {
        routes(&self.sinks)
    }

    pub fn source_routes(&self) -> impl Iterator<Item = Route<'_>> {
        routes(&self.sources)
    }

    pub fn has_multiple_sinks(&self) -> bool {
        self.sink_routes().nth(1).is_some()
    }

    pub fn has_multiple_sources(&self) -> bool {
        self.source_routes().nth(1).is_some()
    }
}

/// Every route a user can choose: each port of a device, or the device
/// itself when it has none.
fn routes(devices: &[Device]) -> impl Iterator<Item = Route<'_>> {
    devices.iter().flat_map(|device| {
        let whole = device.ports.is_empty().then_some(None);
        whole
            .into_iter()
            .chain(device.ports.iter().map(Some))
            .map(move |port| Route { device, port })
    })
}

/// Each acts on the default sink or source, as the server has it when the
/// command runs.
#[derive(Debug, Clone)]
pub enum AudioCommand {
    ToggleSinkMute,
    SetSinkMute(bool),
    /// Scales every channel, keeping their balance.
    SinkVolume(u32),
    ToggleSourceMute,
    SetSourceMute(bool),
    SourceVolume(u32),
    /// A sink and, when it has several, which port.
    DefaultSink(String, Option<String>),
    DefaultSource(String, Option<String>),
}

/// The connection to the server, on a thread of its own that libpulse's
/// main loop runs on. It reconnects by itself when the server goes away,
/// e.g. on a restart of PipeWire.
#[derive(Debug, Clone)]
pub struct Audio {
    commands: Arc<Commands>,
    data: watch::Receiver<AudioData>,
}

#[derive(Debug)]
struct Commands {
    sender: mpsc::Sender<AudioCommand>,
    wake: OwnedFd,
}

impl Commands {
    /// Unblocks the main loop's poll. A full pipe means it is awake already.
    fn wake(&self) {
        // SAFETY: the fd is the open write end of the pipe, owned by `self`.
        unsafe { libc::write(self.wake.as_raw_fd(), [1u8].as_ptr().cast(), 1) };
    }
}

impl Drop for Commands {
    /// With the last handle gone, the thread finds the channel closed.
    fn drop(&mut self) {
        self.wake();
    }
}

impl Audio {
    pub async fn connect() -> anyhow::Result<Self> {
        let mut fds = [0; 2];
        // SAFETY: `fds` has room for the two ends `pipe2` writes.
        if unsafe { libc::pipe2(fds.as_mut_ptr(), libc::O_CLOEXEC | libc::O_NONBLOCK) } != 0 {
            anyhow::bail!("failed to create the wake pipe");
        }
        // SAFETY: `pipe2` succeeded, so both fds are open and owned by nothing else.
        let (wake_read, wake_write) =
            unsafe { (OwnedFd::from_raw_fd(fds[0]), OwnedFd::from_raw_fd(fds[1])) };

        let (sender, commands) = mpsc::channel();
        let (data_tx, data) = watch::channel(AudioData::default());
        thread::Builder::new()
            .name("pulseaudio".into())
            .spawn(move || pulse::run(commands, wake_read, data_tx))?;

        Ok(Self {
            commands: Arc::new(Commands {
                sender,
                wake: wake_write,
            }),
            data,
        })
    }

    /// The current state, then a snapshot per change. A consumer that falls
    /// behind gets the latest one.
    pub fn updates(&self) -> impl Stream<Item = AudioData> + Send + 'static {
        snapshots(self.data.clone())
    }

    /// Queues `command` for the server's thread and wakes it.
    pub fn execute(&self, command: AudioCommand) -> anyhow::Result<()> {
        self.commands.sender.send(command)?;
        self.commands.wake();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn device(name: &str, ports: &[&str]) -> Device {
        Device {
            name: name.to_string(),
            description: name.to_uppercase(),
            volume: NORMAL_VOLUME,
            is_mute: false,
            ports: ports
                .iter()
                .map(|port| Port {
                    name: port.to_string(),
                    description: port.to_uppercase(),
                    kind: PortKind::Unknown,
                })
                .collect(),
            active_port: ports.first().map(|port| port.to_string()),
        }
    }

    #[test]
    fn a_route_per_port_or_one_for_the_device() {
        let data = AudioData {
            sinks: vec![
                device("card", &["speaker", "headphones"]),
                device("hdmi", &[]),
            ],
            ..AudioData::default()
        };

        let routes: Vec<_> = data
            .sink_routes()
            .map(|r| (r.device.name.as_str(), r.port.map(|p| p.name.as_str())))
            .collect();

        assert_eq!(
            routes,
            [
                ("card", Some("speaker")),
                ("card", Some("headphones")),
                ("hdmi", None)
            ]
        );
        assert!(data.has_multiple_sinks());
        assert!(!data.has_multiple_sources());
    }

    #[test]
    fn the_route_in_use_is_the_active_port_or_a_device_without_ports() {
        let data = AudioData {
            sinks: vec![
                device("card", &["speaker", "headphones"]),
                device("hdmi", &[]),
            ],
            ..AudioData::default()
        };

        let active: Vec<_> = data
            .sink_routes()
            .map(|r| {
                (
                    r.device.name.as_str(),
                    r.port.map(|p| p.name.as_str()),
                    r.is_active(),
                )
            })
            .collect();

        assert_eq!(
            active,
            [
                ("card", Some("speaker"), true),
                ("card", Some("headphones"), false),
                ("hdmi", None, true)
            ]
        );
    }

    #[test]
    fn the_active_device_is_the_default_one() {
        let data = AudioData {
            default_sink: "hdmi".to_string(),
            sinks: vec![device("card", &[]), device("hdmi", &[])],
            ..AudioData::default()
        };

        assert_eq!(data.active_sink().map(|d| d.name.as_str()), Some("hdmi"));
        assert_eq!(data.active_source(), None);
    }
}
