use crate::stream::channel;
use futures::{FutureExt, SinkExt, Stream, StreamExt, select, stream::pending};
use inotify::{EventMask, Inotify, WatchMask};
use log::{debug, error, info, warn};
use pipewire::{context::ContextBox, core::PW_ID_CORE, main_loop::MainLoopRc};
use std::{cell::RefCell, collections::HashSet, fs, path::Path, rc::Rc, thread, time::Duration};
use tokio::{
    sync::mpsc::{UnboundedReceiver, unbounded_channel},
    time::{Instant, sleep_until},
};

const WEBCAM_DEVICE_PATH: &str = "/dev/video0";
const RETRY_DELAY: Duration = Duration::from_secs(5);

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Media {
    Video,
    Audio,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplicationNode {
    pub id: u32,
    pub media: Media,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PrivacyData {
    nodes: Vec<ApplicationNode>,
    webcam_access: i32,
}

impl PrivacyData {
    fn new() -> Self {
        Self {
            nodes: Vec::new(),
            webcam_access: is_device_in_use(WEBCAM_DEVICE_PATH),
        }
    }

    pub fn no_access(&self) -> bool {
        self.nodes.is_empty() && self.webcam_access == 0
    }

    pub fn microphone_access(&self) -> bool {
        self.nodes.iter().any(|n| n.media == Media::Audio)
    }

    pub fn webcam_access(&self) -> bool {
        self.webcam_access > 0
    }

    pub fn screenshare_access(&self) -> bool {
        self.nodes.iter().any(|n| n.media == Media::Video)
    }

    /// Streams can't be followed without PipeWire. Returns whether any was dropped.
    fn forget_streams(&mut self) -> bool {
        let had_streams = !self.nodes.is_empty();
        self.nodes.clear();
        had_streams
    }

    fn update(&mut self, event: PrivacyEvent) {
        match event {
            PrivacyEvent::AddNode(node) => {
                self.nodes.push(node);
            }
            PrivacyEvent::RemoveNode(id) => {
                self.nodes.retain(|n| n.id != id);
            }
            PrivacyEvent::WebcamOpen => {
                self.webcam_access += 1;
                debug!("Webcam opened {}", self.webcam_access);
            }
            PrivacyEvent::WebcamClose => {
                self.webcam_access = i32::max(self.webcam_access - 1, 0);
                debug!("Webcam closed {}", self.webcam_access);
            }
        }
    }
}

#[derive(Debug, Clone)]
enum PrivacyEvent {
    AddNode(ApplicationNode),
    RemoveNode(u32),
    WebcamOpen,
    WebcamClose,
}

type WebcamEvents = Box<dyn Stream<Item = PrivacyEvent> + Unpin + Send>;

#[derive(Debug, Clone)]
pub struct Privacy {
    _private: (),
}

impl Privacy {
    pub async fn connect() -> anyhow::Result<Self> {
        Ok(Self { _private: () })
    }

    /// Yields the current state first, then a snapshot after every change.
    /// Without PipeWire, streams are reported as unused while the connection
    /// is retried; the webcam is followed either way.
    pub fn updates(&self) -> impl Stream<Item = PrivacyData> + Send + 'static {
        channel(100, |mut output| async move {
            let mut webcam = webcam_listener().unwrap_or_else(|err| {
                warn!("Failed to connect to webcam: {err}");
                Box::new(pending::<PrivacyEvent>().boxed())
            });

            let mut data = PrivacyData::new();
            if output.send(data.clone()).await.is_err() {
                return;
            }

            let mut pipewire = None;
            let mut next_attempt = Instant::now();
            let mut unavailable = false;

            loop {
                if pipewire.is_none() && Instant::now() >= next_attempt {
                    match create_pipewire_listener().await {
                        Ok(listener) => {
                            info!("Listening for privacy events");
                            unavailable = false;
                            pipewire = Some(listener);
                        }
                        Err(err) => {
                            if unavailable {
                                debug!("PipeWire is still not available: {err}");
                            } else {
                                warn!(
                                    "PipeWire is not available, retrying every {}s: {err}",
                                    RETRY_DELAY.as_secs()
                                );
                                unavailable = true;
                            }
                            next_attempt = Instant::now() + RETRY_DELAY;
                        }
                    }
                }

                let step = select! {
                    step = next_pipewire_step(&mut pipewire, next_attempt).fuse() => step,
                    value = webcam.next().fuse() => match value {
                        Some(event) => Step::Event(event),
                        None => Step::WebcamGone,
                    },
                };

                let changed = match step {
                    Step::Event(event) => {
                        data.update(event);
                        true
                    }
                    Step::PipewireGone => {
                        warn!("Lost the connection to PipeWire, reconnecting");
                        pipewire = None;
                        next_attempt = Instant::now();
                        data.forget_streams()
                    }
                    Step::Retry => false,
                    Step::WebcamGone => {
                        error!("Webcam listener exited");
                        webcam = Box::new(pending::<PrivacyEvent>().boxed());
                        false
                    }
                };

                if changed && output.send(data.clone()).await.is_err() {
                    return;
                }
            }
        })
    }
}

enum Step {
    Event(PrivacyEvent),
    PipewireGone,
    Retry,
    WebcamGone,
}

async fn next_pipewire_step(
    pipewire: &mut Option<UnboundedReceiver<PrivacyEvent>>,
    next_attempt: Instant,
) -> Step {
    match pipewire {
        Some(listener) => match listener.recv().await {
            Some(event) => Step::Event(event),
            None => Step::PipewireGone,
        },
        None => {
            sleep_until(next_attempt).await;
            Step::Retry
        }
    }
}

macro_rules! unwrap_or_send_err {
    ($constructor:expr, $tx:expr) => {
        match $constructor {
            Ok(val) => val,
            Err(e) => {
                let _ = $tx.send(Err(anyhow::Error::from(e)));
                return;
            }
        }
    };
}

async fn create_pipewire_listener() -> anyhow::Result<UnboundedReceiver<PrivacyEvent>> {
    let (tx, rx) = unbounded_channel::<PrivacyEvent>();
    let (boot_tx, boot_rx) = tokio::sync::oneshot::channel::<Result<(), anyhow::Error>>();

    thread::spawn(move || {
        let mainloop = unwrap_or_send_err!(MainLoopRc::new(None), boot_tx);
        let context = unwrap_or_send_err!(ContextBox::new(mainloop.loop_(), None), boot_tx);
        let core = unwrap_or_send_err!(context.connect(None), boot_tx);
        let registry = unwrap_or_send_err!(core.get_registry(), boot_tx);

        // A core error means the connection is gone: stop, so the service reconnects
        let _core_listener = core
            .add_listener_local()
            .error({
                let mainloop = mainloop.downgrade();
                move |id, _seq, res, message| {
                    if id == PW_ID_CORE {
                        warn!("PipeWire core error {res}: {message}");
                        if let Some(mainloop) = mainloop.upgrade() {
                            mainloop.quit();
                        }
                    }
                }
            })
            .register();

        let tracked_ids: Rc<RefCell<HashSet<u32>>> = Rc::new(RefCell::new(HashSet::new()));

        let _listener = registry
            .add_listener_local()
            .global({
                let tx = tx.clone();
                let tracked_ids = Rc::clone(&tracked_ids);
                move |global| {
                    if let Some(props) = global.props
                        && let Some(media) = props
                            .get("media.class")
                            .filter(|v| v == &"Stream/Input/Video" || v == &"Stream/Input/Audio")
                        // ashell's own visualizer captures the sink monitor, not a mic
                        && props.get("node.name") != Some("cava")
                    {
                        debug!("New global: {global:?}");
                        tracked_ids.borrow_mut().insert(global.id);
                        let _ = tx.send(PrivacyEvent::AddNode(ApplicationNode {
                            id: global.id,
                            media: if media == "Stream/Input/Video" {
                                Media::Video
                            } else {
                                Media::Audio
                            },
                        }));
                    }
                }
            })
            .global_remove({
                let tx = tx.clone();
                move |id| {
                    if tracked_ids.borrow_mut().remove(&id) {
                        debug!("Remove tracked global: {id}");
                        let _ = tx.send(PrivacyEvent::RemoveNode(id));
                    }
                }
            })
            .register();

        if boot_tx.send(Ok(())).is_err() {
            return;
        }
        mainloop.run();

        warn!("Pipewire mainloop exited");
    });

    match boot_rx.await {
        Ok(Ok(())) => Ok(rx),
        Ok(Err(e)) => Err(e),
        Err(recv_err) => Err(anyhow::anyhow!(
            "pipewire thread exited before boot could finish: {recv_err}"
        )),
    }
}

fn webcam_listener() -> anyhow::Result<WebcamEvents> {
    let inotify = Inotify::init()?;

    inotify.watches().add(
        WEBCAM_DEVICE_PATH,
        WatchMask::CLOSE_WRITE
            | WatchMask::CLOSE_NOWRITE
            | WatchMask::DELETE_SELF
            | WatchMask::OPEN
            | WatchMask::ATTRIB,
    )?;

    let buffer = [0; 512];
    Ok(Box::new(
        inotify
            .into_event_stream(buffer)?
            .filter_map(async move |event| match event {
                Ok(event) => {
                    debug!("Webcam event: {event:?}");
                    match event.mask {
                        EventMask::OPEN => Some(PrivacyEvent::WebcamOpen),
                        EventMask::CLOSE_WRITE | EventMask::CLOSE_NOWRITE => {
                            Some(PrivacyEvent::WebcamClose)
                        }
                        _ => None,
                    }
                }
                _ => None,
            })
            .boxed(),
    ))
}

fn is_device_in_use(target: &str) -> i32 {
    let mut used_by = 0;
    if let Ok(entries) = fs::read_dir("/proc") {
        for entry in entries.flatten() {
            let pid_path = entry.path();

            // Skip non-numeric directories (not process folders)
            if !pid_path.join("fd").exists() {
                continue;
            }

            // Check file descriptors in each process folder
            if let Ok(fd_entries) = fs::read_dir(pid_path.join("fd")) {
                for fd_entry in fd_entries.flatten() {
                    if let Ok(link_path) = fs::read_link(fd_entry.path())
                        && link_path == Path::new(target)
                    {
                        used_by += 1;
                    }
                }
            }
        }
    }

    used_by
}
