use super::{ReadOnlyService, Service, ServiceEvent};
use crate::components::icons::Icons;
use iced::{
    Subscription, Task,
    futures::{SinkExt, StreamExt, channel::mpsc::Sender, executor::block_on, stream::pending},
    stream::channel,
};
use libpulse_binding::{
    callbacks::ListResult,
    context::{
        self, Context, FlagSet,
        introspect::{Introspector, SinkInfo, SourceInfo},
        subscribe::InterestMaskSet,
    },
    def::{DevicePortType, PortAvailable, SinkState, SourceState},
    mainloop::standard::{IterateResult, Mainloop},
    operation::{self, Operation},
    proplist::{Proplist, properties::APPLICATION_NAME},
    volume::ChannelVolumes,
};
use log::{debug, error, trace};
use std::{
    any::TypeId,
    cell::RefCell,
    ops::{Deref, DerefMut},
    rc::Rc,
    thread::{self, JoinHandle},
};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

#[derive(Debug, Clone)]
pub struct Device {
    pub name: String,
    pub description: String,
    pub volume: ChannelVolumes,
    pub is_mute: bool,
    pub in_use: bool,
    pub ports: Vec<Port>,
}

#[derive(Debug, Clone)]
pub struct Port {
    pub name: String,
    pub description: String,
    pub device_type: DeviceType,
    pub active: bool,
}

#[derive(Debug, Copy, Clone)]
pub enum DeviceType {
    Headphones,
    Speaker,
    Headset,
    Hdmi,
}

impl DeviceType {
    pub fn get_icon(&self) -> Icons {
        match self {
            DeviceType::Speaker => Icons::Speaker3,
            DeviceType::Headphones => Icons::Headphones1,
            DeviceType::Headset => Icons::Headset,
            DeviceType::Hdmi => Icons::MonitorSpeaker,
        }
    }
}

#[derive(Debug, Default, Clone)]
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
    fn get_icon(&self, default_sink: &str) -> Icons;
}

impl Sinks for Vec<Device> {
    fn get_icon(&self, default_sink: &str) -> Icons {
        match self.iter().find_map(|s| {
            if s.ports.iter().any(|p| p.active) && s.name == default_sink {
                Some((s.is_mute, s.volume.get_volume()))
            } else {
                None
            }
        }) {
            Some((true, _)) => Icons::Speaker0,
            Some((false, volume)) => {
                if volume > 0.66 {
                    Icons::Speaker3
                } else if volume > 0.33 {
                    Icons::Speaker2
                } else if volume > 0.000001 {
                    Icons::Speaker1
                } else {
                    Icons::Speaker0
                }
            }
            None => Icons::Speaker0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct AudioData {
    pub server_info: ServerInfo,
    pub sinks: Vec<Device>,
    pub sources: Vec<Device>,
    pub cur_sink_volume: i32,
    pub cur_source_volume: i32,
}

#[derive(Debug, Clone)]
pub struct AudioService {
    data: AudioData,
    commander: UnboundedSender<PulseAudioCommand>,
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

struct PulseAudioServerHandle {
    _listener: JoinHandle<()>,
    _commander: JoinHandle<()>,
    receiver: UnboundedReceiver<PulseAudioServerEvent>,
    sender: UnboundedSender<PulseAudioCommand>,
}

impl AudioService {
    async fn init_service() -> anyhow::Result<PulseAudioServerHandle> {
        PulseAudioServer::start().await
    }

    async fn start_listening(state: State, output: &mut Sender<ServiceEvent<Self>>) -> State {
        match state {
            State::Init => match Self::init_service().await {
                Ok(handle) => {
                    let _ = output
                        .send(ServiceEvent::Init(AudioService {
                            data: AudioData {
                                server_info: ServerInfo::default(),
                                sinks: Vec::new(),
                                sources: Vec::new(),
                                cur_sink_volume: 0,
                                cur_source_volume: 0,
                            },
                            commander: handle.sender.clone(),
                        }))
                        .await;
                    State::Active(handle)
                }
                Err(err) => {
                    error!("Failed to initialize audio service: {err}");
                    State::Error
                }
            },
            State::Active(mut handle) => match handle.receiver.recv().await {
                Some(PulseAudioServerEvent::Error) => {
                    error!("PulseAudio server error");
                    State::Error
                }
                Some(PulseAudioServerEvent::Sinks(sinks)) => {
                    let _ = output
                        .send(ServiceEvent::Update(AudioEvent::Sinks(sinks)))
                        .await;

                    State::Active(handle)
                }
                Some(PulseAudioServerEvent::Sources(sources)) => {
                    let _ = output
                        .send(ServiceEvent::Update(AudioEvent::Sources(sources)))
                        .await;

                    State::Active(handle)
                }
                Some(PulseAudioServerEvent::ServerInfo(info)) => {
                    let _ = output
                        .send(ServiceEvent::Update(AudioEvent::ServerInfo(info)))
                        .await;

                    State::Active(handle)
                }
                None => State::Active(handle),
            },
            State::Error => {
                error!("Audio service error");

                let _ = pending::<u8>().next().await;
                State::Error
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum AudioEvent {
    Sinks(Vec<Device>),
    Sources(Vec<Device>),
    ServerInfo(ServerInfo),
}

enum State {
    Init,
    Active(PulseAudioServerHandle),
    Error,
}

impl ReadOnlyService for AudioService {
    type UpdateEvent = AudioEvent;
    type Error = ();

    fn update(&mut self, event: Self::UpdateEvent) {
        match event {
            AudioEvent::Sinks(sinks) => {
                self.data.sinks = sinks;
                self.data.cur_sink_volume = (self
                    .sinks
                    .iter()
                    .find_map(|sink| {
                        if sink
                            .ports
                            .iter()
                            .any(|p| p.active && sink.name == self.server_info.default_sink)
                        {
                            Some(if sink.is_mute {
                                0.
                            } else {
                                sink.volume.get_volume()
                            })
                        } else {
                            None
                        }
                    })
                    .unwrap_or_default()
                    * 100.) as i32;
            }
            AudioEvent::Sources(sources) => {
                self.data.sources = sources;
                self.data.cur_source_volume = (self
                    .sources
                    .iter()
                    .find_map(|source| {
                        if source
                            .ports
                            .iter()
                            .any(|p| p.active && source.name == self.server_info.default_source)
                        {
                            Some(if source.is_mute {
                                0.
                            } else {
                                source.volume.get_volume()
                            })
                        } else {
                            None
                        }
                    })
                    .unwrap_or_default()
                    * 100.) as i32;
            }
            AudioEvent::ServerInfo(info) => {
                self.data.server_info = info;
                self.data.cur_sink_volume = (self
                    .sinks
                    .iter()
                    .find_map(|sink| {
                        if sink
                            .ports
                            .iter()
                            .any(|p| p.active && sink.name == self.server_info.default_sink)
                        {
                            Some(if sink.is_mute {
                                0.
                            } else {
                                sink.volume.get_volume()
                            })
                        } else {
                            None
                        }
                    })
                    .unwrap_or_default()
                    * 100.) as i32;
                self.data.cur_source_volume = (self
                    .sources
                    .iter()
                    .find_map(|source| {
                        if source
                            .ports
                            .iter()
                            .any(|p| p.active && source.name == self.server_info.default_source)
                        {
                            Some(if source.is_mute {
                                0.
                            } else {
                                source.volume.get_volume()
                            })
                        } else {
                            None
                        }
                    })
                    .unwrap_or_default()
                    * 100.) as i32;
            }
        }
    }

    fn subscribe() -> iced::Subscription<super::ServiceEvent<Self>> {
        let id = TypeId::of::<Self>();

        Subscription::run_with_id(
            id,
            channel(100, async |mut output| {
                let mut state = State::Init;

                loop {
                    state = AudioService::start_listening(state, &mut output).await;
                }
            }),
        )
    }
}

pub enum AudioCommand {
    ToggleSinkMute,
    ToggleSourceMute,
    SinkVolume(i32),
    SourceVolume(i32),
    DefaultSink(String, String),
    DefaultSource(String, String),
}

impl Service for AudioService {
    type Command = AudioCommand;

    fn command(&mut self, command: Self::Command) -> Task<ServiceEvent<Self>> {
        match command {
            AudioCommand::ToggleSinkMute => {
                if let Some(sink) = self
                    .data
                    .sinks
                    .iter()
                    .find(|sink| sink.name == self.data.server_info.default_sink)
                {
                    let _ = self.commander.send(PulseAudioCommand::SinkMute(
                        sink.name.clone(),
                        !sink.is_mute,
                    ));
                }
            }
            AudioCommand::ToggleSourceMute => {
                if let Some(source) = self
                    .data
                    .sources
                    .iter()
                    .find(|source| source.name == self.data.server_info.default_source)
                {
                    let _ = self.commander.send(PulseAudioCommand::SourceMute(
                        source.name.clone(),
                        !source.is_mute,
                    ));
                }
            }
            AudioCommand::SinkVolume(volume) => {
                if let Some(sink) = self
                    .data
                    .sinks
                    .iter_mut()
                    .find(|sink| sink.name == self.data.server_info.default_sink)
                {
                    if let Some(volume) = sink.volume.scale_volume(volume as f64 / 100.) {
                        let _ = self
                            .commander
                            .send(PulseAudioCommand::SinkVolume(sink.name.clone(), *volume));
                    }
                }
            }
            AudioCommand::SourceVolume(volume) => {
                if let Some(source) = self
                    .data
                    .sources
                    .iter_mut()
                    .find(|source| source.name == self.data.server_info.default_source)
                {
                    if let Some(volume) = source.volume.scale_volume(volume as f64 / 100.) {
                        let _ = self.commander.send(PulseAudioCommand::SourceVolume(
                            source.name.clone(),
                            *volume,
                        ));
                    }
                }
            }
            AudioCommand::DefaultSink(name, port) => {
                let _ = self
                    .commander
                    .send(PulseAudioCommand::DefaultSink(name, port));
            }
            AudioCommand::DefaultSource(name, port) => {
                let _ = self
                    .commander
                    .send(PulseAudioCommand::DefaultSource(name, port));
            }
        }

        iced::Task::none()
    }
}

enum PulseAudioServerEvent {
    Error,
    Sinks(Vec<Device>),
    Sources(Vec<Device>),
    ServerInfo(ServerInfo),
}

enum PulseAudioCommand {
    SinkMute(String, bool),
    SourceMute(String, bool),
    SinkVolume(String, ChannelVolumes),
    SourceVolume(String, ChannelVolumes),
    DefaultSink(String, String),
    DefaultSource(String, String),
}

struct PulseAudioServer {
    mainloop: Mainloop,
    context: Context,
    introspector: Introspector,
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

        // Wait for context to be ready
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

        let listener = Self::start_listener(from_server_tx.clone()).await?;
        let commander = Self::start_commander(from_server_tx.clone(), to_server_rx).await?;

        Ok(PulseAudioServerHandle {
            _listener: listener,
            _commander: commander,
            receiver: from_server_rx,
            sender: to_server_tx,
        })
    }

    async fn start_listener(
        from_server_tx: UnboundedSender<PulseAudioServerEvent>,
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

                    loop {
                        let data = server.mainloop.iterate(true);
                        if let IterateResult::Quit(_) | IterateResult::Err(_) = data {
                            error!("PulseAudio mainloop error");
                        }
                    }
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
        mut to_sever_tx: UnboundedReceiver<PulseAudioCommand>,
    ) -> anyhow::Result<JoinHandle<()>> {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

        let handle = thread::spawn(move || {
            block_on(async move {
                match Self::new() {
                    Ok(mut server) => {
                        let _ = tx.send(true);
                        loop {
                            match to_sever_tx.recv().await {
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
                                None => {}
                            }
                        }
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
                trace!("Receved source data: {data:?}");

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
                .map_or(String::default(), |n| n.to_string()),
            description: value
                .proplist
                .get_str("device.description")
                .map_or(String::default(), |d| d.to_string()),
            volume: value.volume,
            is_mute: value.mute,
            in_use: value.state == SinkState::Running,
            ports: value
                .ports
                .iter()
                .filter_map(|port| {
                    if port.available != PortAvailable::No {
                        Some(Port {
                            name: port
                                .name
                                .as_ref()
                                .map_or(String::default(), |n| n.to_string()),
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
                .map_or(String::default(), |n| n.to_string()),
            description: value
                .proplist
                .get_str("device.description")
                .map_or(String::default(), |d| d.to_string()),
            volume: value.volume,
            is_mute: value.mute,
            in_use: value.state == SourceState::Running,
            ports: value
                .ports
                .iter()
                .filter_map(|port| {
                    if port.available != PortAvailable::No {
                        Some(Port {
                            name: port
                                .name
                                .as_ref()
                                .map_or(String::default(), |n| n.to_string()),
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
