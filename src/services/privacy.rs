use super::{ReadOnlyService, ServiceEvent};
use iced::{
    futures::{
        channel::mpsc::Sender, select, stream::pending, FutureExt, SinkExt, Stream, StreamExt,
    },
    subscription::channel,
};
use inotify::{EventMask, Inotify, WatchMask};
use log::{debug, error, info, warn};
use pipewire::{context::Context, main_loop::MainLoop};
use std::{any::TypeId, ops::Deref, thread};
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver};

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

#[derive(Debug, Clone, Default)]
pub struct PrivacyData {
    nodes: Vec<ApplicationNode>,
    webcam_access: bool,
}

impl PrivacyData {
    pub fn no_access(&self) -> bool {
        self.nodes.is_empty() && !self.webcam_access
    }

    pub fn microphone_access(&self) -> bool {
        self.nodes.iter().any(|n| n.media == Media::Audio)
    }

    pub fn webcam_access(&self) -> bool {
        self.webcam_access
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
                                debug!("New global: {:?}", global);
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
                        debug!("Remove global: {}", id);
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
            "/dev/video0",
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
                .filter_map({
                    move |event| async move {
                        if let Ok(event) = event {
                            match event.mask {
                                EventMask::OPEN => Some(PrivacyEvent::WebcamAccess(true)),
                                EventMask::CLOSE_WRITE => Some(PrivacyEvent::WebcamAccess(false)),
                                _ => None,
                            }
                        } else {
                            None
                        }
                    }
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
                        let data = PrivacyData::default();

                        let _ = output
                            .send(ServiceEvent::Init(PrivacyService { data }))
                            .await;

                        State::Active((pipewire, webcam))
                    }
                    _ => {
                        error!("Failed to connect to pipewire or webcam data");

                        State::Error
                    }
                }
            }
            State::Active((mut pipewire, mut webcam)) => {
                info!("Listening for privacy events");

                // let stream = select_all(vec![pipewire.recv(), webcam.next()]);
                //
                // while let Some(event) = stream.next().await {
                //     match event {
                //         Some(event) => {
                //             let _ = output.send(ServiceEvent::Update(event)).await;
                //         }
                //         None => {
                //             error!("Listener exited");
                //         }
                //     }
                // }

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
    WebcamAccess(bool),
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
            PrivacyEvent::WebcamAccess(access) => {
                self.data.webcam_access = access;
            }
        }
    }

    fn subscribe() -> iced::Subscription<ServiceEvent<Self>> {
        let id = TypeId::of::<Self>();

        channel(id, 100, |mut output| async move {
            let mut state = State::Init;

            loop {
                state = PrivacyService::start_listening(state, &mut output).await;
            }
        })
    }
}
