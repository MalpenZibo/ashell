//! libpulse's main loop, on the thread [`run`] owns. The snapshot it
//! publishes is also the state the commands are resolved against.

use super::{AudioCommand, AudioData, Device, Port, PortKind};
use libpulse_binding::{
    callbacks::ListResult,
    context::{
        self, Context, FlagSet,
        introspect::{
            Introspector, ServerInfo, SinkInfo, SinkPortInfo, SourceInfo, SourcePortInfo,
        },
        subscribe::{Facility, InterestMaskSet},
    },
    def::{DevicePortType, PortAvailable},
    mainloop::{
        api::Mainloop as _,
        events::io::FlagSet as IoFlagSet,
        standard::{IterateResult, Mainloop},
    },
    operation::{self, Operation},
    proplist::{Proplist, properties::APPLICATION_NAME},
    volume::{ChannelVolumes, Volume},
};
use log::{debug, error, info, warn};
use std::{
    cell::{Cell, RefCell},
    os::fd::{AsRawFd, OwnedFd, RawFd},
    rc::Rc,
    sync::mpsc::{Receiver, RecvTimeoutError, TryRecvError},
    time::Duration,
};
use tokio::sync::watch;

const RETRY_DELAY: Duration = Duration::from_secs(5);
const APP_NAME: &str = "ashell";

/// Serves the server until every handle is gone, connecting again after
/// `RETRY_DELAY` whenever it is lost.
pub(super) fn run(commands: Receiver<AudioCommand>, wake: OwnedFd, data: watch::Sender<AudioData>) {
    let data = Rc::new(data);
    let mut failing = false;

    loop {
        match Server::connect() {
            Ok(server) => {
                failing = false;
                info!("Connected to the PulseAudio server");
                match server.serve(&commands, wake.as_raw_fd(), &data) {
                    Ok(Closed) => return,
                    Err(err) => warn!("PulseAudio server lost: {err}"),
                }
            }
            Err(err) if failing => debug!("PulseAudio is still not available: {err}"),
            Err(err) => {
                failing = true;
                warn!(
                    "PulseAudio is not available, retrying every {}s: {err}",
                    RETRY_DELAY.as_secs()
                );
            }
        }

        data.send_if_modified(|current| {
            let had_devices = *current != AudioData::default();
            *current = AudioData::default();
            had_devices
        });

        // Commands for a server that is not there are dropped
        loop {
            match commands.recv_timeout(RETRY_DELAY) {
                Ok(_) => continue,
                Err(RecvTimeoutError::Timeout) => break,
                Err(RecvTimeoutError::Disconnected) => return,
            }
        }
    }
}

/// Every handle is gone.
struct Closed;

/// What the main loop's callbacks write to.
struct Shared {
    data: Rc<watch::Sender<AudioData>>,
    /// The lists being received, each complete at its `End`.
    sinks: RefCell<Vec<Device>>,
    sources: RefCell<Vec<Device>>,
    /// Off until the first round of answers is in, so the first snapshot
    /// sent is a whole one.
    ready: Cell<bool>,
}

impl Shared {
    /// Applies `change` and tells the consumers if it changed anything.
    fn update(&self, change: impl FnOnce(&mut AudioData) -> bool) {
        let ready = self.ready.get();
        self.data.send_if_modified(|data| change(data) && ready);
    }
}

/// The context goes before the main loop: closing it frees the I/O events
/// it has on the loop.
struct Server {
    context: Context,
    mainloop: Mainloop,
}

impl Server {
    fn connect() -> anyhow::Result<Self> {
        let mut proplist =
            Proplist::new().ok_or_else(|| anyhow::anyhow!("failed to create a property list"))?;
        proplist
            .set_str(APPLICATION_NAME, APP_NAME)
            .map_err(|_| anyhow::anyhow!("failed to set the application name"))?;

        let mainloop =
            Mainloop::new().ok_or_else(|| anyhow::anyhow!("failed to create a main loop"))?;
        let context = Context::new_with_proplist(&mainloop, APP_NAME, &proplist)
            .ok_or_else(|| anyhow::anyhow!("failed to create a context"))?;
        let mut server = Self { context, mainloop };

        server.context.connect(None, FlagSet::NOFLAGS, None)?;

        loop {
            server.iterate()?;

            match server.context.get_state() {
                context::State::Ready => return Ok(server),
                context::State::Failed | context::State::Terminated => {
                    anyhow::bail!("the connection failed");
                }
                _ => {}
            }
        }
    }

    fn serve(
        mut self,
        commands: &Receiver<AudioCommand>,
        wake: RawFd,
        data: &Rc<watch::Sender<AudioData>>,
    ) -> anyhow::Result<Closed> {
        let shared = Rc::new(Shared {
            data: data.clone(),
            sinks: RefCell::default(),
            sources: RefCell::default(),
            ready: Cell::new(false),
        });

        self.context.subscribe(
            InterestMaskSet::SERVER | InterestMaskSet::SINK | InterestMaskSet::SOURCE,
            |subscribed| {
                if !subscribed {
                    error!("Failed to subscribe to PulseAudio events");
                }
            },
        );

        let mut introspector = self.context.introspect();
        self.wait(&fetch_server(&introspector, &shared))?;
        self.wait(&fetch_sinks(&introspector, &shared))?;
        self.wait(&fetch_sources(&introspector, &shared))?;
        shared.ready.set(true);
        data.send_modify(|_| {});

        // An event names only what changed, which is asked for again
        let events = self.context.introspect();
        let fetching = shared.clone();
        self.context
            .set_subscribe_callback(Some(Box::new(
                move |facility, _operation, _index| match facility {
                    Some(Facility::Server) => drop(fetch_server(&events, &fetching)),
                    Some(Facility::Sink) => drop(fetch_sinks(&events, &fetching)),
                    Some(Facility::Source) => drop(fetch_sources(&events, &fetching)),
                    _ => {}
                },
            )));

        // A command wakes the main loop through the pipe; reading empties it
        let _wake = self.mainloop.new_io_event(
            wake,
            IoFlagSet::INPUT,
            Box::new(move |_event, fd, _flags| {
                let mut buf = [0u8; 64];
                // SAFETY: `fd` is the pipe's open read end and `buf` is writable.
                unsafe { libc::read(fd, buf.as_mut_ptr().cast(), buf.len()) };
            }),
        );

        loop {
            self.iterate()?;

            // A server restart (e.g. `systemctl --user restart pipewire`) drops the
            // connection without making `iterate` fail: the context just moves out of
            // `Ready`. Every call on it would then get a null operation back from
            // libpulse and panic, so the connection is made again instead.
            let state = self.context.get_state();
            if state != context::State::Ready {
                anyhow::bail!("the context is {state:?}");
            }

            loop {
                match commands.try_recv() {
                    Ok(command) => apply(
                        command,
                        &data.borrow(),
                        &mut self.context,
                        &mut introspector,
                    ),
                    Err(TryRecvError::Empty) => break,
                    Err(TryRecvError::Disconnected) => return Ok(Closed),
                }
            }
        }
    }

    fn iterate(&mut self) -> anyhow::Result<()> {
        match self.mainloop.iterate(true) {
            IterateResult::Quit(_) | IterateResult::Err(_) => anyhow::bail!("the main loop failed"),
            IterateResult::Success(_) => Ok(()),
        }
    }

    fn wait<T: ?Sized>(&mut self, operation: &Operation<T>) -> anyhow::Result<()> {
        loop {
            self.iterate()?;

            match operation.get_state() {
                operation::State::Done => return Ok(()),
                // Losing the connection cancels every pending operation, whose
                // `Done` would never come
                operation::State::Cancelled => anyhow::bail!("an operation was cancelled"),
                operation::State::Running => {}
            }
        }
    }
}

type ServerOperation = Operation<dyn FnMut(&ServerInfo<'_>)>;
type SinksOperation = Operation<dyn FnMut(ListResult<&SinkInfo<'_>>)>;
type SourcesOperation = Operation<dyn FnMut(ListResult<&SourceInfo<'_>>)>;

fn fetch_server(introspector: &Introspector, shared: &Rc<Shared>) -> ServerOperation {
    let shared = shared.clone();
    introspector.get_server_info(move |info| {
        let sink = info.default_sink_name.as_deref().unwrap_or_default();
        let source = info.default_source_name.as_deref().unwrap_or_default();
        shared.update(|data| {
            let changed = data.default_sink != sink || data.default_source != source;
            if changed {
                sink.clone_into(&mut data.default_sink);
                source.clone_into(&mut data.default_source);
            }
            changed
        });
    })
}

fn fetch_sinks(introspector: &Introspector, shared: &Rc<Shared>) -> SinksOperation {
    let shared = shared.clone();
    introspector.get_sink_info_list(move |item| match item {
        ListResult::Item(info) => {
            let filter = info.proplist.get_str("node.link-group").is_some();
            let available = info.ports.iter().any(|p| p.available != PortAvailable::No);
            if info.ports.is_empty() || available || filter {
                let ports = sink_ports(&info.ports);
                shared.sinks.borrow_mut().push(device(
                    info.name.as_deref(),
                    info.description.as_deref(),
                    &info.proplist,
                    &info.volume,
                    info.mute,
                    ports,
                ));
            }
        }
        ListResult::End => {
            let sinks = shared.sinks.take();
            shared.update(|data| replace(&mut data.sinks, sinks));
        }
        ListResult::Error => error!("Failed to list the PulseAudio sinks"),
    })
}

fn fetch_sources(introspector: &Introspector, shared: &Rc<Shared>) -> SourcesOperation {
    let shared = shared.clone();
    introspector.get_source_info_list(move |item| match item {
        ListResult::Item(info) => {
            if info.monitor_of_sink.is_none() {
                let ports = source_ports(&info.ports);
                shared.sources.borrow_mut().push(device(
                    info.name.as_deref(),
                    info.description.as_deref(),
                    &info.proplist,
                    &info.volume,
                    info.mute,
                    ports,
                ));
            }
        }
        ListResult::End => {
            let sources = shared.sources.take();
            shared.update(|data| replace(&mut data.sources, sources));
        }
        ListResult::Error => error!("Failed to list the PulseAudio sources"),
    })
}

fn replace(devices: &mut Vec<Device>, new: Vec<Device>) -> bool {
    let changed = *devices != new;
    if changed {
        *devices = new;
    }
    changed
}

fn apply(
    command: AudioCommand,
    data: &AudioData,
    context: &mut Context,
    introspector: &mut Introspector,
) {
    let sink = data.active_sink().map(|sink| sink.name.as_str());
    let source = data.active_source().map(|source| source.name.as_str());

    match command {
        AudioCommand::ToggleSinkMute => {
            if let Some(sink) = data.active_sink() {
                introspector.set_sink_mute_by_name(&sink.name, !sink.is_mute, None);
            }
        }
        AudioCommand::SetSinkMute(mute) => {
            if let Some(sink) = sink {
                introspector.set_sink_mute_by_name(sink, mute, None);
            }
        }
        // Scaled over the channels the server has now, so their balance stays
        AudioCommand::SinkVolume(volume) => {
            if let Some(sink) = sink {
                let mut setter = context.introspect();
                introspector.get_sink_info_by_name(sink, move |item| {
                    if let ListResult::Item(info) = item
                        && let Some(name) = info.name.as_deref()
                        && let Some(channels) = scaled(info.volume, volume)
                    {
                        setter.set_sink_volume_by_name(name, &channels, None);
                    }
                });
            }
        }
        AudioCommand::ToggleSourceMute => {
            if let Some(source) = data.active_source() {
                introspector.set_source_mute_by_name(&source.name, !source.is_mute, None);
            }
        }
        AudioCommand::SetSourceMute(mute) => {
            if let Some(source) = source {
                introspector.set_source_mute_by_name(source, mute, None);
            }
        }
        AudioCommand::SourceVolume(volume) => {
            if let Some(source) = source {
                let mut setter = context.introspect();
                introspector.get_source_info_by_name(source, move |item| {
                    if let ListResult::Item(info) = item
                        && let Some(name) = info.name.as_deref()
                        && let Some(channels) = scaled(info.volume, volume)
                    {
                        setter.set_source_volume_by_name(name, &channels, None);
                    }
                });
            }
        }
        AudioCommand::DefaultSink(name, port) => {
            context.set_default_sink(&name, |_| {});
            if let Some(port) = port {
                introspector.set_sink_port_by_name(&name, &port, None);
            }
        }
        AudioCommand::DefaultSource(name, port) => {
            context.set_default_source(&name, |_| {});
            if let Some(port) = port {
                introspector.set_source_port_by_name(&name, &port, None);
            }
        }
    }
}

/// `channels` with the loudest at `volume`, the others in proportion.
fn scaled(mut channels: ChannelVolumes, volume: u32) -> Option<ChannelVolumes> {
    if channels.scale(Volume(volume)).is_none() {
        error!("Failed to scale the volume {channels} to {volume}");
        return None;
    }
    Some(channels)
}

fn device(
    name: Option<&str>,
    description: Option<&str>,
    proplist: &Proplist,
    channels: &ChannelVolumes,
    is_mute: bool,
    ports: Vec<Port>,
) -> Device {
    let name = name.unwrap_or_default().to_string();

    // A WirePlumber smart filter is chosen as a whole, not by port
    let ports = if proplist.get_str("node.link-group").is_some() {
        if ports.len() > 1 {
            warn!("Unexpected multiple ports in the filter node {name}");
        }
        Vec::new()
    } else {
        ports
    };

    Device {
        description: description
            .map(str::to_string)
            .or_else(|| proplist.get_str("device.description"))
            .unwrap_or_default(),
        name,
        volume: channels.max().0,
        is_mute,
        ports,
    }
}

fn sink_ports(ports: &[SinkPortInfo<'_>]) -> Vec<Port> {
    ports
        .iter()
        .filter(|p| p.available != PortAvailable::No)
        .map(|p| port(p.name.as_deref(), p.description.as_deref(), p.r#type))
        .collect()
}

fn source_ports(ports: &[SourcePortInfo<'_>]) -> Vec<Port> {
    ports
        .iter()
        .filter(|p| p.available != PortAvailable::No)
        .map(|p| port(p.name.as_deref(), p.description.as_deref(), p.r#type))
        .collect()
}

fn port(name: Option<&str>, description: Option<&str>, kind: DevicePortType) -> Port {
    Port {
        name: name.unwrap_or_default().to_string(),
        description: description.unwrap_or_default().to_string(),
        kind: match kind {
            DevicePortType::Unknown => PortKind::Unknown,
            DevicePortType::Aux => PortKind::Aux,
            DevicePortType::Speaker => PortKind::Speaker,
            DevicePortType::Headphones => PortKind::Headphones,
            DevicePortType::Line => PortKind::Line,
            DevicePortType::Mic => PortKind::Mic,
            DevicePortType::Headset => PortKind::Headset,
            DevicePortType::Handset => PortKind::Handset,
            DevicePortType::Earpiece => PortKind::Earpiece,
            DevicePortType::SPDIF => PortKind::Spdif,
            DevicePortType::HDMI => PortKind::Hdmi,
            DevicePortType::TV => PortKind::Tv,
            DevicePortType::Radio => PortKind::Radio,
            DevicePortType::Video => PortKind::Video,
            DevicePortType::USB => PortKind::Usb,
            DevicePortType::Bluetooth => PortKind::Bluetooth,
            DevicePortType::Portable => PortKind::Portable,
            DevicePortType::Handsfree => PortKind::Handsfree,
            DevicePortType::Car => PortKind::Car,
            DevicePortType::HiFi => PortKind::HiFi,
            DevicePortType::Phone => PortKind::Phone,
            DevicePortType::Network => PortKind::Network,
            DevicePortType::Analog => PortKind::Analog,
        },
    }
}
