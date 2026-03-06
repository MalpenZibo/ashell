use crate::components::icons::StaticIcon;
use guido::prelude::*;
use libpulse_binding::{
    callbacks::ListResult,
    context::{
        self, Context, FlagSet,
        introspect::{Introspector, SinkInfo, SourceInfo},
        subscribe::InterestMaskSet,
    },
    def::{DevicePortType, PortAvailable},
    mainloop::standard::{IterateResult, Mainloop},
    operation::{self, Operation},
    proplist::{Proplist, properties::APPLICATION_NAME},
    volume::ChannelVolumes,
};
use log::{debug, error, trace};
use std::{
    any::TypeId,
    cell::RefCell,
    rc::Rc,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread::{self, JoinHandle},
};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

#[derive(Debug, Clone, PartialEq)]
pub struct Device {
    pub name: String,
    pub description: String,
    pub volume: ChannelVolumes,
    pub is_mute: bool,
    pub ports: Vec<Port>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Port {
    pub name: String,
    pub description: String,
    pub device_type: DeviceType,
    pub active: bool,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum DeviceType {
    Headphones,
    Speaker,
    Headset,
    Hdmi,
}

impl DeviceType {
    pub fn get_icon(&self) -> StaticIcon {
        match self {
            DeviceType::Speaker => StaticIcon::Speaker3,
            DeviceType::Headphones => StaticIcon::Headphones1,
            DeviceType::Headset => StaticIcon::Headset,
            DeviceType::Hdmi => StaticIcon::MonitorSpeaker,
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct ServerInfo {
    pub default_sink: String,
    pub default_source: String,
}

pub trait Volume {
    fn get_volume(&self) -> f64;
    fn scale_volume(&mut self, max: f64) -> Option<&mut ChannelVolumes>;
}

impl Volume for ChannelVolumes {
    fn get_volume(&self) -> f64 {
        self.avg().0 as f64 / libpulse_binding::volume::Volume::NORMAL.0 as f64
    }

    fn scale_volume(&mut self, max: f64) -> Option<&mut ChannelVolumes> {
        let max = max.clamp(0.0, 1.0);
        self.scale(libpulse_binding::volume::Volume(
            (libpulse_binding::volume::Volume::NORMAL.0 as f64 * max) as u32,
        ))
    }
}

pub trait Sinks {
    fn get_icon(&self, default_sink: &str) -> StaticIcon;
}

impl Sinks for Vec<Device> {
    fn get_icon(&self, default_sink: &str) -> StaticIcon {
        match self.iter().find_map(|s| {
            if s.ports.iter().any(|p| p.active) && s.name == default_sink {
                Some((s.is_mute, s.volume.get_volume()))
            } else {
                None
            }
        }) {
            Some((true, _)) => StaticIcon::Speaker0,
            Some((false, volume)) => {
                if volume > 0.66 {
                    StaticIcon::Speaker3
                } else if volume > 0.33 {
                    StaticIcon::Speaker2
                } else {
                    StaticIcon::Speaker1
                }
            }
            None => StaticIcon::Speaker0,
        }
    }
}

pub trait Sources {
    fn get_icon(&self, default_source: &str) -> StaticIcon;
}

impl Sources for Vec<Device> {
    fn get_icon(&self, default_source: &str) -> StaticIcon {
        match self.iter().find_map(|s| {
            if s.ports.iter().any(|p| p.active) && s.name == default_source {
                Some(s.is_mute)
            } else {
                None
            }
        }) {
            Some(false) => StaticIcon::Mic1,
            _ => StaticIcon::Mic0,
        }
    }
}

fn compute_volume_pct(devices: &[Device], default_name: &str) -> i32 {
    (devices
        .iter()
        .find_map(|d| {
            if d.ports.iter().any(|p| p.active) && d.name == default_name {
                Some(if d.is_mute { 0. } else { d.volume.get_volume() })
            } else {
                None
            }
        })
        .unwrap_or_default()
        * 100.) as i32
}

#[derive(Clone, PartialEq, guido::SignalFields)]
pub struct AudioData {
    pub server_info: ServerInfo,
    pub sinks: Vec<Device>,
    pub sources: Vec<Device>,
    pub cur_sink_volume: i32,
    pub cur_source_volume: i32,
}

impl Default for AudioData {
    fn default() -> Self {
        Self {
            server_info: ServerInfo::default(),
            sinks: Vec::new(),
            sources: Vec::new(),
            cur_sink_volume: 0,
            cur_source_volume: 0,
        }
    }
}

#[derive(Clone)]
pub enum AudioCmd {
    ToggleSinkMute,
    ToggleSourceMute,
    SinkVolume(i32),
    SourceVolume(i32),
    DefaultSink(String, String),
    DefaultSource(String, String),
}

pub fn create() -> (AudioDataSignals, Service<AudioCmd>) {
    let data = AudioDataSignals::new(AudioData::default());
    let svc = start_audio_service(data.writers());
    (data, svc)
}

// --- PulseAudio internals (copied from ashell, minimal changes) ---

enum PulseAudioServerEvent {
    Error,
    Sinks(Vec<Device>),
    Sources(Vec<Device>),
    ServerInfo(ServerInfo),
}

#[derive(Clone)]
enum PulseAudioCommand {
    SinkMute(String, bool),
    SourceMute(String, bool),
    SinkVolume(String, ChannelVolumes),
    SourceVolume(String, ChannelVolumes),
    DefaultSink(String, String),
    DefaultSource(String, String),
}

struct PulseAudioServerHandle {
    listener: JoinHandle<()>,
    commander: JoinHandle<()>,
    receiver: UnboundedReceiver<PulseAudioServerEvent>,
    sender: UnboundedSender<PulseAudioCommand>,
    listener_running: Arc<AtomicBool>,
}

impl PulseAudioServerHandle {
    /// Signal both PulseAudio threads to stop and wait for them to exit.
    fn shutdown(self) {
        self.listener_running.store(false, Ordering::SeqCst);
        // Dropping sender causes commander's recv() to return None → break
        drop(self.sender);
        drop(self.receiver);
        let _ = self.commander.join();
        let _ = self.listener.join();
    }
}

struct PulseAudioServer {
    // Field order matters for drop order: introspector → context → mainloop.
    // PulseAudio requires context to be dropped before mainloop.
    introspector: Introspector,
    context: Context,
    mainloop: Mainloop,
}

impl PulseAudioServer {
    fn new() -> anyhow::Result<Self> {
        let name = format!("{:?}", TypeId::of::<Self>());
        let mut proplist = Proplist::new().unwrap();
        proplist
            .set_str(APPLICATION_NAME, name.as_str())
            .map_err(|_| anyhow::anyhow!("Failed to set application name"))?;

        let mut mainloop = Mainloop::new().map_or_else(
            || Err(anyhow::anyhow!("Failed to create Pulse audio main loop")),
            Ok,
        )?;

        let mut context = Context::new_with_proplist(&mainloop, name.as_str(), &proplist)
            .map_or_else(
                || Err(anyhow::anyhow!("Failed to create Pulse audio context")),
                Ok,
            )?;

        context.connect(None, FlagSet::NOFLAGS, None)?;

        loop {
            match mainloop.iterate(true) {
                IterateResult::Quit(_) | IterateResult::Err(_) => {
                    panic!("PulseAudio: iterate state was not success")
                }
                IterateResult::Success(_) => {
                    if context.get_state() == context::State::Ready {
                        break;
                    }
                }
            }
        }

        let introspector = context.introspect();
        Ok(Self {
            mainloop,
            context,
            introspector,
        })
    }

    async fn start() -> anyhow::Result<PulseAudioServerHandle> {
        let (from_server_tx, from_server_rx) = tokio::sync::mpsc::unbounded_channel();
        let (to_server_tx, to_server_rx) = tokio::sync::mpsc::unbounded_channel();

        let listener_running = Arc::new(AtomicBool::new(true));
        let listener =
            Self::start_listener(from_server_tx.clone(), listener_running.clone()).await?;
        let commander = Self::start_commander(from_server_tx.clone(), to_server_rx).await?;

        Ok(PulseAudioServerHandle {
            listener,
            commander,
            receiver: from_server_rx,
            sender: to_server_tx,
            listener_running,
        })
    }

    async fn start_listener(
        from_server_tx: UnboundedSender<PulseAudioServerEvent>,
        running: Arc<AtomicBool>,
    ) -> anyhow::Result<JoinHandle<()>> {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

        let handle = thread::spawn({
            let from_server_tx = from_server_tx.clone();
            move || match Self::new() {
                Ok(mut server) => {
                    let _ = tx.send(true);

                    server.context.subscribe(
                        InterestMaskSet::SERVER
                            .union(InterestMaskSet::SINK)
                            .union(InterestMaskSet::SOURCE),
                        |res| {
                            if !res {
                                error!("Audio subscription failed!");
                            }
                        },
                    );

                    match server.wait_for_response(server.introspector.get_server_info({
                        let tx = from_server_tx.clone();
                        move |info| {
                            Self::send_server_info(info, &tx);
                        }
                    })) {
                        Ok(_) => {}
                        Err(e) => {
                            error!("Failed to get server info: {e}");
                            let _ = from_server_tx.send(PulseAudioServerEvent::Error);
                        }
                    };

                    let sinks = Rc::new(RefCell::new(Vec::new()));
                    match server.wait_for_response(server.introspector.get_sink_info_list({
                        let tx = from_server_tx.clone();
                        let sinks = sinks.clone();
                        move |info| {
                            Self::populate_and_send_sinks(info, &tx, &mut sinks.borrow_mut());
                        }
                    })) {
                        Ok(_) => {}
                        Err(e) => {
                            error!("Failed to get sink info: {e}");
                            let _ = from_server_tx.send(PulseAudioServerEvent::Error);
                        }
                    };

                    let sources = Rc::new(RefCell::new(Vec::new()));
                    match server.wait_for_response(server.introspector.get_source_info_list({
                        let tx = from_server_tx.clone();
                        let sources = sources.clone();
                        move |info| {
                            Self::populate_and_send_sources(info, &tx, &mut sources.borrow_mut());
                        }
                    })) {
                        Ok(_) => {}
                        Err(e) => {
                            error!("Failed to get source info: {e}");
                            let _ = from_server_tx.send(PulseAudioServerEvent::Error);
                        }
                    };

                    let introspector = server.context.introspect();
                    server.context.set_subscribe_callback(Some(Box::new(
                        move |_facility, _operation, _idx| {
                            server.introspector.get_server_info({
                                let tx = from_server_tx.clone();
                                move |info| {
                                    Self::send_server_info(info, &tx);
                                }
                            });
                            introspector.get_sink_info_list({
                                let tx = from_server_tx.clone();
                                let sinks = sinks.clone();
                                move |info| {
                                    Self::populate_and_send_sinks(
                                        info,
                                        &tx,
                                        &mut sinks.borrow_mut(),
                                    );
                                }
                            });
                            introspector.get_source_info_list({
                                let tx = from_server_tx.clone();
                                let sources = sources.clone();
                                move |info| {
                                    Self::populate_and_send_sources(
                                        info,
                                        &tx,
                                        &mut sources.borrow_mut(),
                                    );
                                }
                            });
                        },
                    )));

                    while running.load(Ordering::SeqCst) {
                        let data = server.mainloop.iterate(false);
                        if let IterateResult::Quit(_) | IterateResult::Err(_) = data {
                            error!("PulseAudio mainloop error");
                            break;
                        }
                        std::thread::sleep(std::time::Duration::from_millis(5));
                    }

                    // Disconnect before dropping to avoid PulseAudio assertion failure
                    server.context.disconnect();
                    server.context.set_subscribe_callback(None);
                }
                Err(e) => {
                    error!("Failed to start PulseAudio listener thread: {e}");
                    let _ = tx.send(false);
                }
            }
        });

        match rx.recv().await {
            Some(true) => Ok(handle),
            _ => Err(anyhow::anyhow!(
                "Failed to start PulseAudio listener thread"
            )),
        }
    }

    async fn start_commander(
        from_server_tx: UnboundedSender<PulseAudioServerEvent>,
        mut to_server_rx: UnboundedReceiver<PulseAudioCommand>,
    ) -> anyhow::Result<JoinHandle<()>> {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

        let handle = thread::spawn(move || {
            // Use a simple block_on since this thread just loops on commands
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("Failed to build tokio runtime for PA commander");
            rt.block_on(async move {
                match Self::new() {
                    Ok(mut server) => {
                        let _ = tx.send(true);
                        loop {
                            match to_server_rx.recv().await {
                                Some(PulseAudioCommand::SinkMute(name, mute)) => {
                                    let _ = server.set_sink_mute(&name, mute);
                                }
                                Some(PulseAudioCommand::SourceMute(name, mute)) => {
                                    let _ = server.set_source_mute(&name, mute);
                                }
                                Some(PulseAudioCommand::SinkVolume(name, volume)) => {
                                    let _ = server.set_sink_volume(&name, &volume);
                                }
                                Some(PulseAudioCommand::SourceVolume(name, volume)) => {
                                    let _ = server.set_source_volume(&name, &volume);
                                }
                                Some(PulseAudioCommand::DefaultSink(name, port)) => {
                                    let _ = server.set_default_sink(&name, &port);
                                }
                                Some(PulseAudioCommand::DefaultSource(name, port)) => {
                                    let _ = server.set_default_source(&name, &port);
                                }
                                None => break,
                            }
                        }
                        // Disconnect before dropping to avoid PulseAudio assertion failure
                        server.context.disconnect();
                    }
                    Err(e) => {
                        error!("Failed to start PulseAudio server: {e}");
                        let _ = from_server_tx.send(PulseAudioServerEvent::Error);
                    }
                }
            })
        });

        match rx.recv().await {
            Some(true) => Ok(handle),
            _ => Err(anyhow::anyhow!(
                "Failed to start PulseAudio commander thread"
            )),
        }
    }

    fn wait_for_response<T: ?Sized>(&mut self, operation: Operation<T>) -> anyhow::Result<()> {
        loop {
            match self.mainloop.iterate(true) {
                IterateResult::Quit(_) | IterateResult::Err(_) => {
                    error!("PulseAudio: iterate state was not success");
                    return Err(anyhow::anyhow!("PulseAudio: iterate state was not success"));
                }
                IterateResult::Success(_) => {
                    if operation.get_state() == operation::State::Done {
                        break;
                    }
                }
            }
        }
        Ok(())
    }

    fn send_server_info(
        info: &libpulse_binding::context::introspect::ServerInfo<'_>,
        tx: &UnboundedSender<PulseAudioServerEvent>,
    ) {
        let _ = tx.send(PulseAudioServerEvent::ServerInfo(info.into()));
    }

    fn populate_and_send_sinks(
        info: ListResult<&SinkInfo<'_>>,
        tx: &UnboundedSender<PulseAudioServerEvent>,
        sinks: &mut Vec<Device>,
    ) {
        match info {
            ListResult::Item(data) => {
                if data
                    .ports
                    .iter()
                    .any(|port| port.available != PortAvailable::No)
                {
                    debug!("Adding sink data: {data:?}");
                    sinks.push(data.into());
                }
            }
            ListResult::End => {
                debug!("New sink list {sinks:?}");
                let _ = tx.send(PulseAudioServerEvent::Sinks(sinks.clone()));
                sinks.clear();
            }
            ListResult::Error => error!("Error during sink list population"),
        }
    }

    fn populate_and_send_sources(
        info: ListResult<&SourceInfo<'_>>,
        tx: &UnboundedSender<PulseAudioServerEvent>,
        sources: &mut Vec<Device>,
    ) {
        match info {
            ListResult::Item(data) => {
                trace!("Received source data: {data:?}");
                if data
                    .name
                    .as_ref()
                    .map(|name| !name.contains("monitor"))
                    .unwrap_or_default()
                {
                    debug!("Adding source data: {data:?}");
                    sources.push(data.into());
                }
            }
            ListResult::End => {
                debug!("New sources list {sources:?}");
                let _ = tx.send(PulseAudioServerEvent::Sources(sources.clone()));
                sources.clear();
            }
            ListResult::Error => error!("Error during sources list population"),
        }
    }

    fn set_sink_mute(&mut self, name: &str, mute: bool) -> anyhow::Result<()> {
        let op = self.introspector.set_sink_mute_by_name(name, mute, None);
        self.wait_for_response(op)
    }

    fn set_source_mute(&mut self, name: &str, mute: bool) -> anyhow::Result<()> {
        let op = self.introspector.set_source_mute_by_name(name, mute, None);
        self.wait_for_response(op)
    }

    fn set_sink_volume(&mut self, name: &str, volume: &ChannelVolumes) -> anyhow::Result<()> {
        let op = self
            .introspector
            .set_sink_volume_by_name(name, volume, None);
        self.wait_for_response(op)
    }

    fn set_source_volume(&mut self, name: &str, volume: &ChannelVolumes) -> anyhow::Result<()> {
        let op = self
            .introspector
            .set_source_volume_by_name(name, volume, None);
        self.wait_for_response(op)
    }

    fn set_default_sink(&mut self, name: &str, port: &str) -> anyhow::Result<()> {
        let op = self.context.set_default_sink(name, |_| {});
        self.wait_for_response(op)?;
        let op = self.introspector.set_sink_port_by_name(name, port, None);
        self.wait_for_response(op)
    }

    fn set_default_source(&mut self, name: &str, port: &str) -> anyhow::Result<()> {
        let op = self.context.set_default_source(name, |_| {});
        self.wait_for_response(op)?;
        let op = self.introspector.set_source_port_by_name(name, port, None);
        self.wait_for_response(op)
    }
}

// --- From impls ---

impl<'a> From<&'a libpulse_binding::context::introspect::ServerInfo<'a>> for ServerInfo {
    fn from(value: &'a libpulse_binding::context::introspect::ServerInfo<'a>) -> Self {
        Self {
            default_sink: value
                .default_sink_name
                .as_ref()
                .map_or_else(String::default, |s| s.to_string()),
            default_source: value
                .default_source_name
                .as_ref()
                .map_or_else(String::default, |s| s.to_string()),
        }
    }
}

impl From<&SinkInfo<'_>> for Device {
    fn from(value: &SinkInfo<'_>) -> Self {
        Self {
            name: value
                .name
                .as_ref()
                .map_or_else(String::default, |n| n.to_string()),
            description: value
                .proplist
                .get_str("device.description")
                .map_or_else(String::default, |d| d.to_string()),
            volume: value.volume,
            is_mute: value.mute,
            ports: value
                .ports
                .iter()
                .filter_map(|port| {
                    if port.available != PortAvailable::No {
                        Some(Port {
                            name: port
                                .name
                                .as_ref()
                                .map_or_else(String::default, |n| n.to_string()),
                            description: port.description.as_ref().unwrap().to_string(),
                            device_type: match port.r#type {
                                DevicePortType::Headphones => DeviceType::Headphones,
                                DevicePortType::Speaker => DeviceType::Speaker,
                                DevicePortType::Headset => DeviceType::Headset,
                                DevicePortType::HDMI => DeviceType::Hdmi,
                                _ => DeviceType::Speaker,
                            },
                            active: value.active_port.as_ref().and_then(|p| p.name.as_ref())
                                == port.name.as_ref(),
                        })
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>(),
        }
    }
}

impl From<&SourceInfo<'_>> for Device {
    fn from(value: &SourceInfo<'_>) -> Self {
        Self {
            name: value
                .name
                .as_ref()
                .map_or_else(String::default, |n| n.to_string()),
            description: value
                .proplist
                .get_str("device.description")
                .map_or_else(String::default, |d| d.to_string()),
            volume: value.volume,
            is_mute: value.mute,
            ports: value
                .ports
                .iter()
                .filter_map(|port| {
                    if port.available != PortAvailable::No {
                        Some(Port {
                            name: port
                                .name
                                .as_ref()
                                .map_or_else(String::default, |n| n.to_string()),
                            description: port.description.as_ref().unwrap().to_string(),
                            device_type: match port.r#type {
                                DevicePortType::Headphones => DeviceType::Headphones,
                                DevicePortType::Speaker => DeviceType::Speaker,
                                DevicePortType::Headset => DeviceType::Headset,
                                DevicePortType::HDMI => DeviceType::Hdmi,
                                _ => DeviceType::Speaker,
                            },
                            active: value.active_port.as_ref().and_then(|p| p.name.as_ref())
                                == port.name.as_ref(),
                        })
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>(),
        }
    }
}

// --- guido service wrapper ---

fn start_audio_service(writers: AudioDataWriters) -> Service<AudioCmd> {
    create_service::<AudioCmd, _, _>(move |mut rx, ctx| async move {
        // Start PulseAudio threads
        let mut handle = match PulseAudioServer::start().await {
            Ok(h) => h,
            Err(e) => {
                error!("Failed to initialize audio service: {e}");
                return;
            }
        };

        // Maintain local state for command handling (need mutable access to volumes)
        let mut local_sinks: Vec<Device> = Vec::new();
        let mut local_sources: Vec<Device> = Vec::new();
        let mut local_server_info = ServerInfo::default();

        while ctx.is_running() {
            tokio::select! {
                cmd = rx.recv() => {
                    match cmd {
                        Some(cmd) => {
                            handle_audio_cmd(
                                &cmd, &handle.sender,
                                &mut local_sinks, &mut local_sources,
                                &local_server_info,
                            );
                        }
                        None => break,
                    }
                }
                event = handle.receiver.recv() => {
                    match event {
                        Some(PulseAudioServerEvent::Error) => {
                            error!("PulseAudio server error");
                            break;
                        }
                        Some(PulseAudioServerEvent::Sinks(sinks)) => {
                            let vol = compute_volume_pct(&sinks, &local_server_info.default_sink);
                            local_sinks = sinks.clone();
                            writers.sinks.set(sinks);
                            writers.cur_sink_volume.set(vol);
                        }
                        Some(PulseAudioServerEvent::Sources(sources)) => {
                            let vol = compute_volume_pct(&sources, &local_server_info.default_source);
                            local_sources = sources.clone();
                            writers.sources.set(sources);
                            writers.cur_source_volume.set(vol);
                        }
                        Some(PulseAudioServerEvent::ServerInfo(info)) => {
                            // Recompute volumes with new default
                            let sink_vol = compute_volume_pct(&local_sinks, &info.default_sink);
                            let source_vol = compute_volume_pct(&local_sources, &info.default_source);
                            local_server_info = info.clone();
                            writers.server_info.set(info);
                            writers.cur_sink_volume.set(sink_vol);
                            writers.cur_source_volume.set(source_vol);
                        }
                        None => {}
                    }
                }
            }
        }

        // Clean shutdown: stop PulseAudio threads before dropping resources
        handle.shutdown();
    })
}

fn handle_audio_cmd(
    cmd: &AudioCmd,
    sender: &UnboundedSender<PulseAudioCommand>,
    local_sinks: &mut Vec<Device>,
    local_sources: &mut Vec<Device>,
    server_info: &ServerInfo,
) {
    match cmd {
        AudioCmd::ToggleSinkMute => {
            if let Some(sink) = local_sinks
                .iter()
                .find(|s| s.name == server_info.default_sink)
            {
                let _ = sender.send(PulseAudioCommand::SinkMute(
                    sink.name.clone(),
                    !sink.is_mute,
                ));
            }
        }
        AudioCmd::ToggleSourceMute => {
            if let Some(source) = local_sources
                .iter()
                .find(|s| s.name == server_info.default_source)
            {
                let _ = sender.send(PulseAudioCommand::SourceMute(
                    source.name.clone(),
                    !source.is_mute,
                ));
            }
        }
        AudioCmd::SinkVolume(volume) => {
            if let Some(sink) = local_sinks
                .iter_mut()
                .find(|s| s.name == server_info.default_sink)
            {
                if let Some(vol) = sink.volume.scale_volume(*volume as f64 / 100.) {
                    let _ = sender.send(PulseAudioCommand::SinkVolume(sink.name.clone(), *vol));
                }
            }
        }
        AudioCmd::SourceVolume(volume) => {
            if let Some(source) = local_sources
                .iter_mut()
                .find(|s| s.name == server_info.default_source)
            {
                if let Some(vol) = source.volume.scale_volume(*volume as f64 / 100.) {
                    let _ = sender.send(PulseAudioCommand::SourceVolume(source.name.clone(), *vol));
                }
            }
        }
        AudioCmd::DefaultSink(name, port) => {
            let _ = sender.send(PulseAudioCommand::DefaultSink(name.clone(), port.clone()));
        }
        AudioCmd::DefaultSource(name, port) => {
            let _ = sender.send(PulseAudioCommand::DefaultSource(name.clone(), port.clone()));
        }
    }
}
