extern crate libpulse_binding as pulse;

use crate::{components::icons::Icons, modules::settings::AudioMessage};
use iced::{futures::SinkExt, Subscription};
use libpulse_binding::{
    context::Context,
    mainloop::standard::{IterateResult, Mainloop},
    proplist::Proplist,
};
use pulse::{
    callbacks::ListResult,
    context::{
        introspect::{SinkInfo, SourceInfo},
        subscribe::InterestMaskSet,
        FlagSet,
    },
    def::{DevicePortType, PortAvailable, SourceState},
    operation::{Operation, State},
    volume::ChannelVolumes,
};
use std::{
    cell::RefCell,
    ops::{Deref, DerefMut},
    rc::Rc,
    thread,
};

fn init(name: &str) -> (Rc<RefCell<Mainloop>>, Rc<RefCell<Context>>) {
    let mut proplist = Proplist::new().unwrap();
    proplist
        .set_str(pulse::proplist::properties::APPLICATION_NAME, name)
        .unwrap();

    let mainloop = Rc::new(RefCell::new(
        Mainloop::new().expect("Failed to create mainloop"),
    ));

    let context = Rc::new(RefCell::new(
        Context::new_with_proplist(mainloop.borrow().deref(), name, &proplist)
            .expect("Failed to create new context"),
    ));

    context
        .borrow_mut()
        .connect(None, FlagSet::NOFLAGS, None)
        .expect("Failed to connect context");

    // Wait for context to be ready
    loop {
        match mainloop.borrow_mut().iterate(true) {
            IterateResult::Quit(_) | IterateResult::Err(_) => {
                panic!("PulseAudio: iterate state was not success")
            }
            IterateResult::Success(_) => {
                if context.borrow().get_state() == pulse::context::State::Ready {
                    break;
                }
            }
        }
    }

    (mainloop, context)
}

fn wait_for_response<T: ?Sized>(mainloop: &mut Mainloop, operation: Operation<T>) {
    loop {
        match mainloop.iterate(true) {
            IterateResult::Quit(_) | IterateResult::Err(_) => {
                panic!("PulseAudio: iterate state was not success")
            }
            IterateResult::Success(_) => {
                if operation.get_state() == State::Done {
                    break;
                }
            }
        }
    }
}

fn create_sink(data: &SinkInfo) -> Option<Sink> {
    if data
        .ports
        .iter()
        .any(|port| port.available != PortAvailable::No)
    {
        Some(Sink {
            name: data
                .name
                .as_ref()
                .map_or(String::default(), |n| n.to_string()),
            description: data
                .proplist
                .get_str("device.description")
                .map_or(String::default(), |d| d.to_string()),
            volume: data.volume,
            is_mute: data.mute,
            ports: data
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
                            active: data.active_port.as_ref().and_then(|p| p.name.as_ref())
                                == port.name.as_ref(),
                        })
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>(),
        })
    } else {
        None
    }
}

fn create_source(data: &SourceInfo) -> Option<Source> {
    if data.state == SourceState::Running
        && data.ports.iter().any(|p| p.available != PortAvailable::No)
    {
        Some(Source {
            name: data
                .name
                .as_ref()
                .map_or(String::default(), |n| n.to_string()),
            description: data
                .proplist
                .get_str("device.description")
                .map_or(String::default(), |d| d.to_string()),
            volume: data.volume,
            is_mute: data.mute,
            ports: data
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
                            active: data.active_port.as_ref().and_then(|p| p.name.as_ref())
                                == port.name.as_ref(),
                        })
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>(),
        })
    } else {
        None
    }
}

fn populate_and_send_sinks(
    info: ListResult<&SinkInfo>,
    tx: &tokio::sync::mpsc::UnboundedSender<AudioMessage>,
    sinks: &mut Vec<Sink>,
) {
    match info {
        ListResult::Item(data) => {
            if let Some(sink) = create_sink(data) {
                sinks.push(sink);
            }
        }
        ListResult::End => {
            let _ = tx.send(AudioMessage::SinkChanges(sinks.clone()));
            sinks.clear();
        }
        ListResult::Error => println!("Error"),
    }
}

fn populate_and_send_sources(
    info: ListResult<&SourceInfo>,
    tx: &tokio::sync::mpsc::UnboundedSender<AudioMessage>,
    sources: &mut Vec<Source>,
) {
    match info {
        ListResult::Item(data) => {
            if let Some(source) = create_source(data) {
                sources.push(source);
            }
        }
        ListResult::End => {
            let _ = tx.send(AudioMessage::SourceChanges(sources.clone()));
            sources.clear();
        }
        ListResult::Error => println!("Error"),
    }
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
    fn get_icon(&self) -> Icons;
}

impl Sinks for Vec<Sink> {
    fn get_icon(&self) -> Icons {
        match self.iter().find_map(|s| {
            if s.ports.iter().any(|p| p.active) {
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

pub trait Sources {
    fn get_icon(&self) -> Icons;
}

impl Sources for Vec<Source> {
    fn get_icon(&self) -> Icons {
        match self.iter().find_map(|s| {
            if s.ports.iter().any(|p| p.active) {
                Some(s.is_mute)
            } else {
                None
            }
        }) {
            Some(true) => Icons::Mic0,
            Some(false) => Icons::Mic1,
            None => Icons::Mic0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Sink {
    pub name: String,
    pub description: String,
    pub volume: ChannelVolumes,
    pub is_mute: bool,
    pub ports: Vec<Port>,
}

#[derive(Debug, Clone)]
pub struct Source {
    pub name: String,
    pub description: String,
    pub volume: ChannelVolumes,
    pub is_mute: bool,
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

pub enum AudioCommand {
    SinkMute(String, bool),
    SourceMute(String, bool),
    SinkVolume(String, ChannelVolumes),
    SourceVolume(String, ChannelVolumes),
    DefaultSink(String, String),
    DefaultSource(String, String),
}

pub struct AudioCommander {
    mainloop: Rc<RefCell<Mainloop>>,
    context: Rc<RefCell<Context>>,
    introspector: pulse::context::introspect::Introspector,
}

impl AudioCommander {
    pub fn new() -> Self {
        let (mainloop, context) = init("Ashell-audio-listener");
        let introspector = context.borrow().introspect();
        AudioCommander {
            mainloop,
            context,
            introspector,
        }
    }

    pub fn set_sink_mute(&mut self, name: &str, mute: bool) {
        let op = self.introspector.set_sink_mute_by_name(name, mute, None);

        self.wait_for_response(op);
    }

    pub fn set_source_mute(&mut self, name: &str, mute: bool) {
        let op = self.introspector.set_source_mute_by_name(name, mute, None);

        self.wait_for_response(op);
    }

    pub fn set_sink_volume(&mut self, name: &str, volume: &ChannelVolumes) {
        let op = self
            .introspector
            .set_sink_volume_by_name(name, volume, None);

        self.wait_for_response(op);
    }

    pub fn set_source_volume(&mut self, name: &str, volume: &ChannelVolumes) {
        let op = self
            .introspector
            .set_source_volume_by_name(name, volume, None);

        self.wait_for_response(op);
    }

    pub fn set_default_sink(&mut self, name: &str, port: &str) {
        let op = self.context.borrow_mut().set_default_sink(name, |_| {});
        self.wait_for_response(op);

        let op = self.introspector.set_sink_port_by_name(name, port, None);
        self.wait_for_response(op);
    }

    pub fn set_default_source(&mut self, name: &str, port: &str) {
        let op = self.context.borrow_mut().set_default_source(name, |_| {});
        self.wait_for_response(op);

        let op = self.introspector.set_source_port_by_name(name, port, None);
        self.wait_for_response(op);
    }

    fn wait_for_response<T: ?Sized>(&self, operation: Operation<T>) {
        loop {
            match self.mainloop.borrow_mut().iterate(true) {
                IterateResult::Quit(_) | IterateResult::Err(_) => {
                    panic!("PulseAudio: iterate state was not success")
                }
                IterateResult::Success(_) => {
                    if operation.get_state() == State::Done {
                        break;
                    }
                }
            }
        }
    }
}

pub fn subscription(
    rx: Option<tokio::sync::mpsc::UnboundedReceiver<AudioCommand>>,
) -> Subscription<AudioMessage> {
    iced::Subscription::batch(vec![
        iced::subscription::channel("audio-commander", 100, |_| async move {
            let (internal_tx, mut internal_rx) = tokio::sync::mpsc::unbounded_channel::<()>();

            thread::spawn(move || {
                let mut rx = rx.unwrap();
                let mut audio_commander = AudioCommander::new();

                internal_tx.send(()).unwrap();
                loop {
                    if let Some(command) = rx.blocking_recv() {
                        match command {
                            AudioCommand::SinkMute(name, mute) => {
                                audio_commander.set_sink_mute(&name, mute);
                            }
                            AudioCommand::SourceMute(name, mute) => {
                                audio_commander.set_source_mute(&name, mute);
                            }
                            AudioCommand::SinkVolume(name, volume) => {
                                audio_commander.set_sink_volume(&name, &volume);
                            }
                            AudioCommand::SourceVolume(name, volume) => {
                                audio_commander.set_source_volume(&name, &volume);
                            }
                            AudioCommand::DefaultSink(name, port) => {
                                audio_commander.set_default_sink(&name, &port);
                            }
                            AudioCommand::DefaultSource(name, port) => {
                                audio_commander.set_default_source(&name, &port);
                            }
                        }
                    }
                }
            });

            loop {
                let _ = internal_rx.recv().await;
            }
        }),
        iced::subscription::channel("audio-listener", 100, |mut output| async move {
            let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<AudioMessage>();

            thread::spawn(move || {
                let (mainloop, context) = init("ashell-audio-listener");

                context.borrow_mut().subscribe(
                    InterestMaskSet::SERVER
                        .union(InterestMaskSet::SINK)
                        .union(InterestMaskSet::SOURCE),
                    |res| {
                        if !res {
                            println!("Subscription failed!");
                        }
                    },
                );

                let sinks = Rc::new(RefCell::new(Vec::new()));
                let sources = Rc::new(RefCell::new(Vec::new()));

                let introspector = context.borrow().introspect();
                let get_server = introspector.get_server_info({
                    let tx = tx.clone();
                    move |info| {
                        let _ = tx.send(AudioMessage::DefaultSinkSourceChanged(
                            info.default_sink_name
                                .as_ref()
                                .map_or_else(String::default, |s| s.to_string()),
                            info.default_source_name
                                .as_ref()
                                .map_or_else(String::default, |s| s.to_string()),
                        ));
                    }
                });
                wait_for_response(mainloop.borrow_mut().deref_mut(), get_server);
                let get_and_send_sinks = introspector.get_sink_info_list({
                    let tx = tx.clone();
                    let sinks = sinks.clone();
                    move |info| populate_and_send_sinks(info, &tx, &mut sinks.borrow_mut())
                });
                wait_for_response(mainloop.borrow_mut().deref_mut(), get_and_send_sinks);
                let get_and_send_source = introspector.get_source_info_list({
                    let tx = tx.clone();
                    let sources = sources.clone();
                    move |info| populate_and_send_sources(info, &tx, &mut sources.borrow_mut())
                });
                wait_for_response(mainloop.borrow_mut().deref_mut(), get_and_send_source);

                context.borrow_mut().set_subscribe_callback({
                    let context = context.clone();
                    Some(Box::new(move |_facility, _operation, _idx| {
                        let introspector = context.borrow().introspect();
                        introspector.get_server_info({
                            let tx = tx.clone();
                            move |info| {
                                let _ = tx.send(AudioMessage::DefaultSinkSourceChanged(
                                    info.default_sink_name
                                        .as_ref()
                                        .map_or_else(String::default, |s| s.to_string()),
                                    info.default_source_name
                                        .as_ref()
                                        .map_or_else(String::default, |s| s.to_string()),
                                ));
                            }
                        });
                        introspector.get_sink_info_list({
                            let tx = tx.clone();
                            let sinks = sinks.clone();

                            move |info| {
                                populate_and_send_sinks(info, &tx, &mut sinks.borrow_mut());
                            }
                        });
                        introspector.get_source_info_list({
                            let tx = tx.clone();
                            let sources = sources.clone();

                            move |info| {
                                populate_and_send_sources(info, &tx, &mut sources.borrow_mut())
                            }
                        });
                    }))
                });

                loop {
                    let data = mainloop.borrow_mut().iterate(true);
                    if let IterateResult::Quit(_) | IterateResult::Err(_) = data {
                        println!("PulseAudio mainloop error");
                    }
                }
            });

            loop {
                let data = rx.recv().await.unwrap();
                let _ = output.send(data).await;
            }
        }),
    ])
}
