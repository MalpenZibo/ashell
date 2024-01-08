extern crate libpulse_binding as pulse;

use iced::{futures::SinkExt, Subscription};
use libpulse_binding::{
    context::Context,
    mainloop::standard::{IterateResult, Mainloop},
    proplist::Proplist,
};
use pulse::{
    callbacks::ListResult,
    context::{
        introspect::{ServerInfo, SinkInfo, SourceInfo},
        subscribe::InterestMaskSet,
        FlagSet,
    },
    def::{DevicePortType, PortAvailable, SourceState},
    operation::{Operation, State},
};
use std::{
    cell::RefCell,
    ops::{Deref, DerefMut},
    rc::Rc,
    thread,
};

use crate::{components::icons::Icons, modules::settings::AudioMessage};

fn init(name: &str) -> (Rc<RefCell<Mainloop>>, Rc<RefCell<Context>>) {
    let mut proplist = Proplist::new().unwrap();
    proplist
        .set_str(pulse::proplist::properties::APPLICATION_NAME, name)
        .unwrap();

    let mainloop = Rc::new(RefCell::new(
        Mainloop::new().expect("Failed to create mainloop"),
    ));

    let context = Rc::new(RefCell::new(
        Context::new_with_proplist(mainloop.borrow().deref(), "FooAppContext", &proplist)
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
                    println!("Operation done");
                    break;
                }
            }
        }
    }
}

fn set_default_sink(info: &ServerInfo, default_sink: &mut Option<String>) {
    if let Some(name) = info.default_sink_name.as_ref() {
        default_sink.replace(name.to_string());
    } else {
        default_sink.take();
    }
}

fn set_default_source(info: &ServerInfo, default_source: &mut Option<String>) {
    if let Some(name) = info.default_source_name.as_ref() {
        default_source.replace(name.to_string());
    } else {
        default_source.take();
    }
}

fn create_sink(data: &SinkInfo, default_sink: &Option<String>) -> Option<Sink> {
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
            volume: data.volume.avg().0 as f64 / libpulse_binding::volume::Volume::NORMAL.0 as f64,
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
                            r#type: match port.r#type {
                                DevicePortType::Headphones => DeviceType::Headphones,
                                DevicePortType::Speaker => DeviceType::Speakers,
                                DevicePortType::Headset => DeviceType::Headset,
                                _ => DeviceType::Speakers,
                            },
                            active: data.active_port.as_ref().and_then(|p| p.name.as_ref())
                                == port.name.as_ref()
                                && &data.name.as_ref().map(|n| n.to_string()) == default_sink,
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

fn create_source(data: &SourceInfo, default_source: &Option<String>) -> Option<Source> {
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
            volume: data.volume.avg().0 as f64 / libpulse_binding::volume::Volume::NORMAL.0 as f64,
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
                            r#type: match port.r#type {
                                DevicePortType::Headphones => DeviceType::Headphones,
                                DevicePortType::Speaker => DeviceType::Speakers,
                                DevicePortType::Headset => DeviceType::Headset,
                                _ => DeviceType::Speakers,
                            },
                            active: data.active_port.as_ref().and_then(|p| p.name.as_ref())
                                == port.name.as_ref()
                                && &data.name.as_ref().map(|n| n.to_string()) == default_source,
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
    default_sink: &Option<String>,
) {
    match info {
        ListResult::Item(data) => {
            if let Some(sink) = create_sink(data, default_sink) {
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
    default_source: &Option<String>,
) {
    match info {
        ListResult::Item(data) => {
            if let Some(source) = create_source(data, default_source) {
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

pub trait Sinks {
    fn get_icon(&self) -> Icons;
}

impl Sinks for Vec<Sink> {
    fn get_icon(&self) -> Icons {
        match self.iter().find_map(|s| {
            if s.ports.iter().any(|p| p.active) {
                Some((s.is_mute, s.volume))
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
    pub volume: f64,
    pub is_mute: bool,
    pub ports: Vec<Port>,
}

#[derive(Debug, Clone)]
pub struct Source {
    pub name: String,
    pub description: String,
    pub volume: f64,
    pub is_mute: bool,
    pub ports: Vec<Port>,
}

#[derive(Debug, Clone)]
pub struct Port {
    pub name: String,
    pub description: String,
    pub r#type: DeviceType,
    pub active: bool,
}

#[derive(Debug, Clone)]
pub enum DeviceType {
    Headphones,
    Speakers,
    Headset,
}

pub fn subscription() -> Subscription<AudioMessage> {
    iced::subscription::channel("audio-listener", 100, |mut output| async move {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<AudioMessage>();

        thread::spawn(move || {
            let (mainloop, context) = init("Ashell-audio-listener");

            context.borrow_mut().subscribe(
                InterestMaskSet::SINK.union(InterestMaskSet::SOURCE),
                |res| {
                    if !res {
                        println!("Subscription failed!");
                    }
                },
            );

            let default_sink = Rc::new(RefCell::new(None));
            let default_source = Rc::new(RefCell::new(None));
            let sinks = Rc::new(RefCell::new(Vec::new()));
            let sources = Rc::new(RefCell::new(Vec::new()));

            let introspector = context.borrow().introspect();
            let get_server = introspector.get_server_info({
                let default_sink = default_sink.clone();
                let default_source = default_source.clone();
                move |info| {
                    set_default_sink(info, &mut default_sink.borrow_mut());
                    set_default_source(info, &mut default_source.borrow_mut())
                }
            });
            wait_for_response(mainloop.borrow_mut().deref_mut(), get_server);
            let get_and_send_sinks = introspector.get_sink_info_list({
                let tx = tx.clone();
                let default_sink = default_sink.clone();
                let sinks = sinks.clone();
                move |info| {
                    populate_and_send_sinks(
                        info,
                        &tx,
                        &mut sinks.borrow_mut(),
                        &default_sink.borrow(),
                    )
                }
            });
            wait_for_response(mainloop.borrow_mut().deref_mut(), get_and_send_sinks);
            let get_and_send_source = introspector.get_source_info_list({
                let tx = tx.clone();
                let default_source = default_source.clone();
                let sources = sources.clone();
                move |info| {
                    populate_and_send_sources(
                        info,
                        &tx,
                        &mut sources.borrow_mut(),
                        &default_source.borrow(),
                    )
                }
            });
            wait_for_response(mainloop.borrow_mut().deref_mut(), get_and_send_source);

            context.borrow_mut().set_subscribe_callback({
                let context = context.clone();
                Some(Box::new(move |_facility, _operation, _idx| {
                    let introspector = context.borrow().introspect();
                    introspector.get_server_info({
                        let default_sink = default_sink.clone();
                        move |info| set_default_sink(info, &mut default_sink.borrow_mut())
                    });
                    introspector.get_sink_info_list({
                        let tx = tx.clone();
                        let default_sink = default_sink.clone();
                        let sinks = sinks.clone();

                        move |info| {
                            populate_and_send_sinks(
                                info,
                                &tx,
                                &mut sinks.borrow_mut(),
                                &default_sink.borrow(),
                            )
                        }
                    });
                    introspector.get_source_info_list({
                        let tx = tx.clone();
                        let default_source = default_source.clone();
                        let sources = sources.clone();

                        move |info| {
                            populate_and_send_sources(
                                info,
                                &tx,
                                &mut sources.borrow_mut(),
                                &default_source.borrow(),
                            )
                        }
                    });
                }))
            });

            println!("Starting PulseAudio mainloop");

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
    })
}
