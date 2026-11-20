//! libpulse's main loop, on the thread [`run`] owns: it keeps the state the
//! snapshots are made of and applies the commands to it.

use super::{AudioCommand, AudioData, Device, Port, PortKind};
use libpulse_binding::{
    callbacks::ListResult,
    context::{
        self, Context, FlagSet,
        introspect::{
            Introspector, ServerInfo, SinkInfo, SinkPortInfo, SourceInfo, SourcePortInfo,
        },
        subscribe::InterestMaskSet,
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
    cell::RefCell,
    collections::HashMap,
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

/// What the main loop's callbacks fill in and the commands read.
#[derive(Default)]
struct State {
    data: AudioData,
    /// Each device's own channels, which a volume is scaled over.
    sink_channels: HashMap<String, ChannelVolumes>,
    source_channels: HashMap<String, ChannelVolumes>,
    /// The lists being received, each complete at its `End`.
    sinks: Vec<(Device, ChannelVolumes)>,
    sources: Vec<(Device, ChannelVolumes)>,
}

struct Shared {
    state: RefCell<State>,
    data: Rc<watch::Sender<AudioData>>,
}

impl Shared {
    fn publish(&self) {
        let state = self.state.borrow();
        self.data.send_if_modified(|current| {
            let changed = *current != state.data;
            if changed {
                current.clone_from(&state.data);
            }
            changed
        });
    }
}

struct Server {
    mainloop: Mainloop,
    context: Context,
}

impl Server {
    fn connect() -> anyhow::Result<Self> {
        let mut proplist =
            Proplist::new().ok_or_else(|| anyhow::anyhow!("failed to create a property list"))?;
        proplist
            .set_str(APPLICATION_NAME, APP_NAME)
            .map_err(|_| anyhow::anyhow!("failed to set the application name"))?;

        let mut mainloop =
            Mainloop::new().ok_or_else(|| anyhow::anyhow!("failed to create a main loop"))?;
        let mut context = Context::new_with_proplist(&mainloop, APP_NAME, &proplist)
            .ok_or_else(|| anyhow::anyhow!("failed to create a context"))?;

        context.connect(None, FlagSet::NOFLAGS, None)?;

        loop {
            if let IterateResult::Quit(_) | IterateResult::Err(_) = mainloop.iterate(true) {
                anyhow::bail!("the main loop failed while connecting");
            }

            match context.get_state() {
                context::State::Ready => break,
                context::State::Failed | context::State::Terminated => {
                    anyhow::bail!("the connection failed");
                }
                _ => {}
            }
        }

        Ok(Self { mainloop, context })
    }

    fn serve(
        mut self,
        commands: &Receiver<AudioCommand>,
        wake: RawFd,
        data: &Rc<watch::Sender<AudioData>>,
    ) -> anyhow::Result<Closed> {
        let shared = Rc::new(Shared {
            state: RefCell::default(),
            data: data.clone(),
        });

        self.context.subscribe(
            InterestMaskSet::SERVER | InterestMaskSet::SINK | InterestMaskSet::SOURCE,
            |subscribed| {
                if !subscribed {
                    error!("Failed to subscribe to PulseAudio events");
                }
            },
        );

        let introspector = self.context.introspect();
        let (server, sinks, sources) = fetch(&introspector, &shared);
        self.wait(&server)?;
        self.wait(&sinks)?;
        self.wait(&sources)?;

        let events = self.context.introspect();
        let fetching = shared.clone();
        self.context.set_subscribe_callback(Some(Box::new(
            move |_facility, _operation, _index| {
                fetch(&events, &fetching);
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

        let mut introspector = introspector;
        loop {
            if let IterateResult::Quit(_) | IterateResult::Err(_) = self.mainloop.iterate(true) {
                anyhow::bail!("the main loop failed");
            }

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
                    Ok(command) => {
                        let state = shared.state.borrow();
                        apply(command, &state, &mut self.context, &mut introspector);
                    }
                    Err(TryRecvError::Empty) => break,
                    Err(TryRecvError::Disconnected) => return Ok(Closed),
                }
            }
        }
    }

    fn wait<T: ?Sized>(&mut self, operation: &Operation<T>) -> anyhow::Result<()> {
        loop {
            if let IterateResult::Quit(_) | IterateResult::Err(_) = self.mainloop.iterate(true) {
                anyhow::bail!("the main loop failed");
            }

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

/// Asks for the defaults and both lists again: an event tells only that
/// something changed.
fn fetch(
    introspector: &Introspector,
    shared: &Rc<Shared>,
) -> (ServerOperation, SinksOperation, SourcesOperation) {
    let server = introspector.get_server_info({
        let shared = shared.clone();
        move |info| {
            {
                let data = &mut shared.state.borrow_mut().data;
                data.default_sink = info
                    .default_sink_name
                    .as_deref()
                    .unwrap_or_default()
                    .to_string();
                data.default_source = info
                    .default_source_name
                    .as_deref()
                    .unwrap_or_default()
                    .to_string();
            }
            shared.publish();
        }
    });

    let sinks = introspector.get_sink_info_list({
        let shared = shared.clone();
        move |item| match item {
            ListResult::Item(info) => {
                let kept = info.ports.is_empty()
                    || info.ports.iter().any(|p| p.available != PortAvailable::No)
                    || info.proplist.get_str("node.link-group").is_some();
                if kept {
                    let device = device(
                        info.name.as_deref(),
                        info.description.as_deref(),
                        info.proplist.get_str("device.description"),
                        info.volume,
                        info.mute,
                        info.proplist.get_str("node.link-group").is_some(),
                        sink_ports(&info.ports),
                    );
                    shared.state.borrow_mut().sinks.push((device, info.volume));
                }
            }
            ListResult::End => {
                {
                    let state = &mut *shared.state.borrow_mut();
                    let (devices, channels) = split(std::mem::take(&mut state.sinks));
                    state.data.sinks = devices;
                    state.sink_channels = channels;
                }
                shared.publish();
            }
            ListResult::Error => error!("Failed to list the PulseAudio sinks"),
        }
    });

    let sources = introspector.get_source_info_list({
        let shared = shared.clone();
        move |item| match item {
            ListResult::Item(info) => {
                if info
                    .name
                    .as_deref()
                    .is_some_and(|name| !name.contains("monitor"))
                {
                    let device = device(
                        info.name.as_deref(),
                        info.description.as_deref(),
                        info.proplist.get_str("device.description"),
                        info.volume,
                        info.mute,
                        info.proplist.get_str("node.link-group").is_some(),
                        source_ports(&info.ports),
                    );
                    shared
                        .state
                        .borrow_mut()
                        .sources
                        .push((device, info.volume));
                }
            }
            ListResult::End => {
                {
                    let state = &mut *shared.state.borrow_mut();
                    let (devices, channels) = split(std::mem::take(&mut state.sources));
                    state.data.sources = devices;
                    state.source_channels = channels;
                }
                shared.publish();
            }
            ListResult::Error => error!("Failed to list the PulseAudio sources"),
        }
    });

    (server, sinks, sources)
}

fn split(list: Vec<(Device, ChannelVolumes)>) -> (Vec<Device>, HashMap<String, ChannelVolumes>) {
    let channels = list
        .iter()
        .map(|(device, channels)| (device.name.clone(), *channels))
        .collect();
    let devices = list.into_iter().map(|(device, _)| device).collect();
    (devices, channels)
}

fn apply(
    command: AudioCommand,
    state: &State,
    context: &mut Context,
    introspector: &mut Introspector,
) {
    let data = &state.data;
    let sink = data.active_sink();
    let source = data.active_source();

    match command {
        AudioCommand::ToggleSinkMute => {
            if let Some(sink) = sink {
                introspector.set_sink_mute_by_name(&sink.name, !sink.is_mute, None);
            }
        }
        AudioCommand::SetSinkMute(mute) => {
            if let Some(sink) = sink {
                introspector.set_sink_mute_by_name(&sink.name, mute, None);
            }
        }
        AudioCommand::SinkVolume(volume) => {
            if let Some(sink) = sink
                && let Some(channels) = scaled(state.sink_channels.get(&sink.name), volume)
            {
                introspector.set_sink_volume_by_name(&sink.name, &channels, None);
            }
        }
        AudioCommand::ToggleSourceMute => {
            if let Some(source) = source {
                introspector.set_source_mute_by_name(&source.name, !source.is_mute, None);
            }
        }
        AudioCommand::SetSourceMute(mute) => {
            if let Some(source) = source {
                introspector.set_source_mute_by_name(&source.name, mute, None);
            }
        }
        AudioCommand::SourceVolume(volume) => {
            if let Some(source) = source
                && let Some(channels) = scaled(state.source_channels.get(&source.name), volume)
            {
                introspector.set_source_volume_by_name(&source.name, &channels, None);
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
fn scaled(channels: Option<&ChannelVolumes>, volume: u32) -> Option<ChannelVolumes> {
    let mut channels = *channels?;
    if channels.scale(Volume(volume)).is_none() {
        error!("Failed to scale the volume {channels} to {volume}");
        return None;
    }
    Some(channels)
}

#[allow(clippy::too_many_arguments)]
fn device(
    name: Option<&str>,
    description: Option<&str>,
    fallback_description: Option<String>,
    channels: ChannelVolumes,
    is_mute: bool,
    is_filter: bool,
    ports: Vec<Port>,
) -> Device {
    Device {
        name: name.unwrap_or_default().to_string(),
        description: description
            .map(str::to_string)
            .or(fallback_description)
            .unwrap_or_default(),
        volume: channels.max().0,
        is_mute,
        is_filter,
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
