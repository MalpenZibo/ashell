use super::{ReadOnlyService, ServiceEvent};
use crate::components::icons::Icons;
use iced::{
    futures::{channel::mpsc::Sender, stream::pending, SinkExt, StreamExt},
    subscription::channel,
};
use log::{debug, error, info, warn};
use pipewire::{context::Context, main_loop::MainLoop};
use std::{
    any::TypeId,
    ops::{Deref, DerefMut},
    thread,
};
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver};

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Media {
    Video,
    Audio,
}

impl Media {
    pub fn to_icon(self) -> Icons {
        match self {
            Media::Video => Icons::ScreenShare,
            Media::Audio => Icons::Mic1,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ApplicationNode {
    pub id: u32,
    pub media: Media,
    pub application: String,
}

#[derive(Debug, Clone, Default)]
pub struct PrivacyData(Vec<ApplicationNode>);

impl Deref for PrivacyData {
    type Target = Vec<ApplicationNode>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for PrivacyData {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
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
    async fn create_listener() -> anyhow::Result<UnboundedReceiver<PrivacyEvent>> {
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
                                    application: props.get("node.name").map_or_else(
                                        || "generic".to_string(),
                                        |name| name.to_lowercase(),
                                    ),
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

    async fn start_listening(state: State, output: &mut Sender<ServiceEvent<Self>>) -> State {
        match state {
            State::Init => match Self::create_listener().await {
                Ok(rx) => {
                    let data = PrivacyData::default();

                    let _ = output
                        .send(ServiceEvent::Init(PrivacyService { data }))
                        .await;

                    State::Active(rx)
                }
                Err(err) => {
                    error!("Failed to connect to pipewire: {}", err);

                    State::Error
                }
            },
            State::Active(mut rx) => {
                info!("Listening for privacy events");

                while let Some(event) = rx.recv().await {
                    let _ = output.send(ServiceEvent::Update(event)).await;
                }

                State::Active(rx)
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
    Active(UnboundedReceiver<PrivacyEvent>),
    Error,
}

#[derive(Debug, Clone)]
pub enum PrivacyEvent {
    AddNode(ApplicationNode),
    RemoveNode(u32),
}

impl ReadOnlyService for PrivacyService {
    type UpdateEvent = PrivacyEvent;
    type Error = ();

    fn update(&mut self, event: Self::UpdateEvent) {
        match event {
            PrivacyEvent::AddNode(node) => {
                self.data.push(node);
            }
            PrivacyEvent::RemoveNode(id) => {
                self.data.retain(|n| n.id != id);
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
