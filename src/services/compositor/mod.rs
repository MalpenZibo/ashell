pub mod backend;
pub mod generic;
pub mod hyprland;
mod listener;
pub mod niri;
pub mod patch;
pub mod types;

pub use self::types::{CompositorCommand, CompositorEvent, CompositorService, CompositorState};

use self::backend::Backend;
use crate::services::{ReadOnlyService, Service, ServiceEvent};
use iced::futures::SinkExt;
use iced::{Subscription, Task, stream::channel};
use std::{any::TypeId, ops::Deref, sync::OnceLock};
use tokio::sync::{OnceCell, broadcast};

const BROADCAST_CAPACITY: usize = 64;

static BROADCASTER: OnceCell<broadcast::Sender<ServiceEvent<CompositorService>>> =
    OnceCell::const_new();

static BACKEND: OnceLock<Option<Backend>> = OnceLock::new();

/// The detected compositor backend, initialized once and shared by reference.
fn backend() -> Option<&'static Backend> {
    BACKEND.get_or_init(backend::detect).as_ref()
}

/// Subscribe to compositor events.  Initializes the broadcaster on first call.
async fn broadcaster_subscribe() -> broadcast::Receiver<ServiceEvent<CompositorService>> {
    BROADCASTER
        .get_or_init(|| async {
            let (tx, _) = broadcast::channel(BROADCAST_CAPACITY);
            tokio::spawn(broadcaster_event_loop(tx.clone()));
            tx
        })
        .await
        .subscribe()
}

async fn broadcaster_event_loop(tx: broadcast::Sender<ServiceEvent<CompositorService>>) {
    let Some(compositor) = backend() else {
        log::error!("No supported compositor backend found");
        let _ = tx.send(ServiceEvent::Error(
            "No supported compositor backend found".into(),
        ));
        return;
    };

    log::info!(
        "Starting compositor event loop with {} backend",
        compositor.name()
    );

    listener::run(compositor, tx).await;
}

impl Deref for CompositorService {
    type Target = CompositorState;
    fn deref(&self) -> &Self::Target {
        &self.state
    }
}

impl ReadOnlyService for CompositorService {
    type UpdateEvent = CompositorEvent;
    type Error = String;

    fn update(&mut self, event: Self::UpdateEvent) {
        match event {
            CompositorEvent::StateChanged(new_state) => {
                self.state = *new_state;
            }
            CompositorEvent::ActionPerformed => {}
        }
    }

    fn subscribe() -> Subscription<ServiceEvent<Self>> {
        Subscription::run_with(TypeId::of::<Self>(), |_| {
            channel(10, async move |mut output| {
                let mut rx = broadcaster_subscribe().await;

                // Send an empty Init to new subscribers once a backend exists.
                if backend().is_some() {
                    let empty_init = CompositorService {
                        state: CompositorState::default(),
                    };
                    if output.send(ServiceEvent::Init(empty_init)).await.is_err() {
                        log::debug!("Compositor subscriber disconnected before receiving Init");
                        return;
                    }
                }

                loop {
                    match rx.recv().await {
                        Ok(event) => {
                            if output.send(event).await.is_err() {
                                log::debug!("Compositor subscriber disconnected");
                                break;
                            }
                        }
                        Err(broadcast::error::RecvError::Lagged(n)) => {
                            log::warn!("Compositor subscriber lagged by {} messages", n);
                        }
                        Err(broadcast::error::RecvError::Closed) => {
                            log::error!("Compositor broadcaster closed unexpectedly");
                            break;
                        }
                    }
                }
            })
        })
    }
}

impl Service for CompositorService {
    type Command = CompositorCommand;

    fn command(&mut self, command: Self::Command) -> Task<ServiceEvent<Self>> {
        Task::perform(execute_command(command), |res| match res {
            Ok(()) => ServiceEvent::Update(CompositorEvent::ActionPerformed),
            Err(e) => ServiceEvent::Error(e),
        })
    }
}

async fn execute_command(command: CompositorCommand) -> Result<(), String> {
    match backend() {
        Some(compositor) => compositor.execute(command).await.map_err(|e| e.to_string()),
        None => Err("No supported compositor backend found".into()),
    }
}
