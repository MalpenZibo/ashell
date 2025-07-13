use super::{ReadOnlyService, ServiceEvent};
use iced::{
    Subscription,
    futures::{
        FutureExt, SinkExt, Stream, StreamExt, channel::mpsc::Sender, select, stream::pending,
    },
    stream::channel,
};
use inotify::{EventMask, Inotify, WatchMask};
use log::{debug, error, info, warn};
use pipewire::{context::Context, main_loop::MainLoop};
use std::{any::TypeId, fs, ops::Deref, path::Path, thread};
use tokio::sync::mpsc::{UnboundedReceiver, unbounded_channel};

const WEBCAM_DEVICE_PATH: &str = "/dev/video0";

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Media {
    Video,
    Audio,
}

#[derive(Debug, Clone)]
pub struct ApplicationNode {
    pub id: u32,
    pub media: Media,
}

#[derive(Debug, Clone)]
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
}

#[derive(Debug, Clone)]
pub struct PrivacyService {
    data: PrivacyData,
}

impl Deref for PrivacyService {
    type Target = PrivacyData;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl PrivacyService {
    async fn create_pipewire_listener() -> anyhow::Result<UnboundedReceiver<PrivacyEvent>> {
        let (tx, rx) = unbounded_channel::<PrivacyEvent>();

        thread::spawn(move || {
            let mainloop = MainLoop::new(None).unwrap();
            let context = Context::new(&mainloop).unwrap();
            let core = context.connect(None).unwrap();
            let registry = core.get_registry().unwrap();

            let _listener = registry
                .add_listener_local()
                .global({
                    let tx = tx.clone();
                    move |global| {
                        if let Some(props) = global.props {
                            if let Some(media) = props.get("media.class").filter(|v| {
                                v == &"Stream/Input/Video" || v == &"Stream/Input/Audio"
                            }) {
                                debug!("New global: {global:?}");
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
                    }
                })
                .global_remove({
                    let tx = tx.clone();
                    move |id| {
                        debug!("Remove global: {id}");
                        let _ = tx.send(PrivacyEvent::RemoveNode(id));
                    }
                })
                .register();

            mainloop.run();

            warn!("Pipewire mainloop exited");
        });

        Ok(rx)
    }

    async fn webcam_listener() -> anyhow::Result<Box<dyn Stream<Item = PrivacyEvent> + Unpin + Send>>
    {
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

    async fn start_listening(state: State, output: &mut Sender<ServiceEvent<Self>>) -> State {
        match state {
            State::Init => {
                let pipewire = Self::create_pipewire_listener().await;
                let webcam = Self::webcam_listener().await;
                match (pipewire, webcam) {
                    (Ok(pipewire), Ok(webcam)) => {
                        let data = PrivacyData::new();

                        let _ = output
                            .send(ServiceEvent::Init(PrivacyService { data }))
                            .await;

                        State::Active((pipewire, webcam))
                    }
                    (Err(pipewire_error), Ok(_)) => {
                        error!("Failed to connect to pipewire: {pipewire_error}");

                        State::Error
                    }
                    (Ok(pipewire), Err(webcam_error)) => {
                        warn!("Failed to connect to webcam: {webcam_error}");

                        State::Active((pipewire, Box::new(pending::<PrivacyEvent>().boxed())))
                    }
                    (Err(pipewire_error), Err(webcam_error)) => {
                        error!("Failed to connect to pipewire: {pipewire_error}");
                        error!("Failed to connect to webcam: {webcam_error}");

                        State::Error
                    }
                }
            }
            State::Active((mut pipewire, mut webcam)) => {
                info!("Listening for privacy events");

                select! {
                    value = pipewire.recv().fuse() => {
                        match value {
                            Some(event) => {
                                let _ = output.send(ServiceEvent::Update(event)).await;
                            }
                            None => {
                                error!("Pipewire listener exited");
                            }
                        }
                    },
                    value = webcam.next().fuse() => {
                        match value {
                            Some(event) => {
                                let _ = output.send(ServiceEvent::Update(event)).await;
                            }
                            None => {
                                error!("Webcam listener exited");
                            }
                        }
                    }
                };

                State::Active((pipewire, webcam))
            }
            State::Error => {
                error!("Privacy service error");

                let _ = pending::<u8>().next().await;
                State::Error
            }
        }
    }
}

enum State {
    Init,
    Active(
        (
            UnboundedReceiver<PrivacyEvent>,
            Box<dyn Stream<Item = PrivacyEvent> + Unpin + Send>,
        ),
    ),
    Error,
}

#[derive(Debug, Clone)]
pub enum PrivacyEvent {
    AddNode(ApplicationNode),
    RemoveNode(u32),
    WebcamOpen,
    WebcamClose,
}

impl ReadOnlyService for PrivacyService {
    type UpdateEvent = PrivacyEvent;
    type Error = ();

    fn update(&mut self, event: Self::UpdateEvent) {
        match event {
            PrivacyEvent::AddNode(node) => {
                self.data.nodes.push(node);
            }
            PrivacyEvent::RemoveNode(id) => {
                self.data.nodes.retain(|n| n.id != id);
            }
            PrivacyEvent::WebcamOpen => {
                self.data.webcam_access += 1;
                debug!("Webcam opened {}", self.data.webcam_access);
            }
            PrivacyEvent::WebcamClose => {
                self.data.webcam_access = i32::max(self.data.webcam_access - 1, 0);
                debug!("Webcam closed {}", self.data.webcam_access);
            }
        }
    }

    fn subscribe() -> Subscription<ServiceEvent<Self>> {
        let id = TypeId::of::<Self>();

        Subscription::run_with_id(
            id,
            channel(100, async |mut output| {
                let mut state = State::Init;

                loop {
                    state = PrivacyService::start_listening(state, &mut output).await;
                }
            }),
        )
    }
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
                    if let Ok(link_path) = fs::read_link(fd_entry.path()) {
                        if link_path == Path::new(target) {
                            used_by += 1;
                        }
                    }
                }
            }
        }
    }

    used_by
}
