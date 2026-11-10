//! The libpulse side. libpulse is not async: the connection lives on its own
//! thread, which both listens for changes and runs commands.

use super::{Device, Port, ServerInfo};
use libpulse_binding::{
    callbacks::ListResult,
    context::{
        self, Context, FlagSet,
        introspect::{Introspector, SinkInfo, SourceInfo},
        subscribe::InterestMaskSet,
    },
    def::PortAvailable,
    mainloop::{
        api::Mainloop as MainloopTrait,
        events::io::FlagSet as IoFlagSet,
        standard::{IterateResult, Mainloop},
    },
    operation::{self, Operation},
    proplist::{Proplist, properties::APPLICATION_NAME},
    volume::ChannelVolumes,
};
use log::{debug, error, trace};
use std::{
    any::TypeId,
    cell::RefCell,
    fmt,
    os::fd::{AsRawFd, FromRawFd, OwnedFd},
    rc::Rc,
    sync::Arc,
    thread::{self, JoinHandle},
};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

/// Write end of a pipe used to wake the PulseAudio mainloop when commands arrive.
#[derive(Clone)]
pub(super) struct WakePipe(Arc<OwnedFd>);

impl fmt::Debug for WakePipe {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("WakePipe")
            .field(&self.0.as_raw_fd())
            .finish()
    }
}

impl WakePipe {
    pub(super) fn wake(&self) {
        // Write a single byte to unblock the mainloop's poll().
        // Safe to ignore errors - if the pipe is full, the mainloop is already awake.
        let fd = self.0.as_raw_fd();
        unsafe { libc::write(fd, [1u8].as_ptr() as *const _, 1) };
    }
}

pub(super) struct PulseAudioServerHandle {
    _listener: JoinHandle<()>,
    pub(super) receiver: UnboundedReceiver<PulseAudioServerEvent>,
    pub(super) sender: UnboundedSender<PulseAudioCommand>,
    pub(super) wake: WakePipe,
}

impl Drop for PulseAudioServerHandle {
    // Nobody listens any more: let the thread notice and drop its connection
    fn drop(&mut self) {
        self.receiver.close();
        self.wake.wake();
    }
}

pub(super) enum PulseAudioServerEvent {
    Error,
    Sinks(Vec<Device>),
    Sources(Vec<Device>),
    ServerInfo(ServerInfo),
}

pub(super) enum PulseAudioCommand {
    SinkMute(String, bool),
    SourceMute(String, bool),
    SinkVolume(String, ChannelVolumes),
    SourceVolume(String, ChannelVolumes),
    DefaultSink(String, Option<String>),
    DefaultSource(String, Option<String>),
}

pub(super) struct PulseAudioServer {
    mainloop: Mainloop,
    context: Context,
    introspector: Introspector,
}

impl PulseAudioServer {
    fn new() -> anyhow::Result<Self> {
        let name = format!("{:?}", TypeId::of::<Self>());
        let mut proplist = Proplist::new()
            .ok_or_else(|| anyhow::anyhow!("Failed to create PulseAudio property list"))?;
        proplist
            .set_str(APPLICATION_NAME, name.as_str())
            .map_err(|_| anyhow::anyhow!("Failed to set application name"))?;

        let mut mainloop = Mainloop::new()
            .ok_or_else(|| anyhow::anyhow!("Failed to create Pulse audio main loop"))?;

        let mut context = Context::new_with_proplist(&mainloop, name.as_str(), &proplist)
            .ok_or_else(|| anyhow::anyhow!("Failed to create Pulse audio context"))?;

        context.connect(None, FlagSet::NOFLAGS, None)?;

        // Wait for context to be ready
        loop {
            match mainloop.iterate(true) {
                IterateResult::Quit(_) | IterateResult::Err(_) => {
                    return Err(anyhow::anyhow!("PulseAudio: iterate state was not success"));
                }
                IterateResult::Success(_) => match context.get_state() {
                    context::State::Ready => break,
                    context::State::Failed | context::State::Terminated => {
                        return Err(anyhow::anyhow!("PulseAudio: connection failed"));
                    }
                    _ => {}
                },
            }
        }

        let introspector = context.introspect();

        Ok(Self {
            mainloop,
            context,
            introspector,
        })
    }

    pub(super) async fn start() -> anyhow::Result<PulseAudioServerHandle> {
        let (from_server_tx, from_server_rx) = tokio::sync::mpsc::unbounded_channel();
        let (to_server_tx, to_server_rx) = tokio::sync::mpsc::unbounded_channel();

        let (init_tx, mut init_rx) = tokio::sync::mpsc::unbounded_channel();

        // Create a pipe to wake the PulseAudio mainloop when commands arrive.
        let (wake_read, wake_write) = {
            let mut fds = [0i32; 2];
            if unsafe { libc::pipe2(fds.as_mut_ptr(), libc::O_CLOEXEC | libc::O_NONBLOCK) } != 0 {
                return Err(anyhow::anyhow!("failed to create wake pipe"));
            }
            unsafe { (OwnedFd::from_raw_fd(fds[0]), OwnedFd::from_raw_fd(fds[1])) }
        };
        let wake = WakePipe(Arc::new(wake_write));

        // Single thread for both listening and commanding — avoids PulseAudio
        // mainloop assertion failures from concurrent connections.
        let handle = thread::spawn({
            let from_server_tx = from_server_tx.clone();
            let mut to_server_rx = to_server_rx;
            move || match Self::new() {
                Ok(mut server) => {
                    let _ = init_tx.send(true);

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
                            return;
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
                            return;
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
                            return;
                        }
                    };

                    let introspector = server.context.introspect();
                    let error_tx = from_server_tx.clone();
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

                    // Register the wake pipe on the mainloop so that iterate(true)
                    // unblocks when a command is sent from another thread.
                    let wake_fd = wake_read.as_raw_fd();
                    let _wake_io = server.mainloop.new_io_event(
                        wake_fd,
                        IoFlagSet::INPUT,
                        Box::new(move |_event, _fd, _flags| {
                            // Drain the pipe so it doesn't keep firing
                            let mut buf = [0u8; 64];
                            unsafe { libc::read(wake_fd, buf.as_mut_ptr() as *mut _, buf.len()) };
                        }),
                    );

                    // Main loop: blocks until PulseAudio events OR wake pipe signal
                    let mut cmd_introspector = server.context.introspect();
                    loop {
                        let data = server.mainloop.iterate(true);
                        if let IterateResult::Quit(_) | IterateResult::Err(_) = data {
                            error!("PulseAudio mainloop error");
                            let _ = error_tx.send(PulseAudioServerEvent::Error);
                            break;
                        }

                        // A server restart (e.g. `systemctl --user restart pipewire`) drops the
                        // connection without making `iterate` fail: the context just moves out of
                        // `Ready`. Every context/introspector call below would then get a null
                        // operation back from libpulse and panic, so bail out and let the
                        // subscription reconnect instead.
                        let context_state = server.context.get_state();
                        if context_state != context::State::Ready {
                            error!("PulseAudio context is no longer ready: {context_state:?}");
                            let _ = error_tx.send(PulseAudioServerEvent::Error);
                            break;
                        }

                        // The listener is gone
                        if error_tx.is_closed() {
                            debug!("PulseAudio thread stopped");
                            break;
                        }

                        // Process any pending commands
                        while let Ok(cmd) = to_server_rx.try_recv() {
                            match cmd {
                                PulseAudioCommand::SinkMute(name, mute) => {
                                    cmd_introspector.set_sink_mute_by_name(&name, mute, None);
                                }
                                PulseAudioCommand::SourceMute(name, mute) => {
                                    cmd_introspector.set_source_mute_by_name(&name, mute, None);
                                }
                                PulseAudioCommand::SinkVolume(name, volume) => {
                                    cmd_introspector.set_sink_volume_by_name(&name, &volume, None);
                                }
                                PulseAudioCommand::SourceVolume(name, volume) => {
                                    cmd_introspector
                                        .set_source_volume_by_name(&name, &volume, None);
                                }
                                PulseAudioCommand::DefaultSink(name, port) => {
                                    server.context.set_default_sink(&name, |_| {});
                                    if let Some(port) = port {
                                        cmd_introspector.set_sink_port_by_name(&name, &port, None);
                                    }
                                }
                                PulseAudioCommand::DefaultSource(name, port) => {
                                    server.context.set_default_source(&name, |_| {});
                                    if let Some(port) = port {
                                        cmd_introspector
                                            .set_source_port_by_name(&name, &port, None);
                                    }
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to start PulseAudio thread: {e}");
                    let _ = init_tx.send(false);
                }
            }
        });

        match init_rx.recv().await {
            Some(true) => Ok(PulseAudioServerHandle {
                _listener: handle,
                receiver: from_server_rx,
                sender: to_server_tx,
                wake,
            }),
            _ => Err(anyhow::anyhow!("Failed to start PulseAudio thread")),
        }
    }

    fn wait_for_response<T: ?Sized>(&mut self, operation: Operation<T>) -> anyhow::Result<()> {
        loop {
            match self.mainloop.iterate(true) {
                IterateResult::Quit(_) | IterateResult::Err(_) => {
                    error!("PulseAudio: iterate state was not success");
                    return Err(anyhow::anyhow!("PulseAudio: iterate state was not success"));
                }
                IterateResult::Success(_) => match operation.get_state() {
                    operation::State::Done => break,
                    // Losing the connection cancels every pending operation. Without this the
                    // loop would wait for a `Done` that can never arrive.
                    operation::State::Cancelled => {
                        error!("PulseAudio: operation was cancelled");
                        return Err(anyhow::anyhow!("PulseAudio: operation was cancelled"));
                    }
                    operation::State::Running => {}
                },
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
                if data.ports.is_empty()
                    || data
                        .ports
                        .iter()
                        .any(|port| port.available != PortAvailable::No)
                    || data.proplist.get_str("node.link-group").is_some()
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
                .map_or_else(String::default, |n| n.to_string()),
            description: value
                .description
                .as_ref()
                .map(|d| d.to_string())
                .or_else(|| value.proplist.get_str("device.description"))
                .unwrap_or_default(),
            volume: value.volume,
            is_mute: value.mute,
            is_filter: value.proplist.get_str("node.link-group").is_some(),
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
                            description: port
                                .description
                                .as_ref()
                                .map_or_else(String::default, |d| d.to_string()),
                            device_type: port.r#type,
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
                .description
                .as_ref()
                .map(|d| d.to_string())
                .or_else(|| value.proplist.get_str("device.description"))
                .unwrap_or_default(),
            volume: value.volume,
            is_mute: value.mute,
            is_filter: value.proplist.get_str("node.link-group").is_some(),
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
                            description: port
                                .description
                                .as_ref()
                                .map_or_else(String::default, |d| d.to_string()),
                            device_type: port.r#type,
                        })
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>(),
        }
    }
}
