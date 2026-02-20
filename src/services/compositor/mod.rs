pub mod hyprland;
pub mod mangowc;
pub mod niri;
pub mod types;

pub use self::types::{
    CompositorChoice, CompositorCommand, CompositorEvent, CompositorService, CompositorState,
};

use crate::services::{ReadOnlyService, Service, ServiceEvent};
use iced::futures::SinkExt;
use iced::{Subscription, Task, stream::channel};
use std::{any::TypeId, ops::Deref};
use tokio::sync::{OnceCell, broadcast};

const BROADCAST_CAPACITY: usize = 64;

static BROADCASTER: OnceCell<broadcast::Sender<ServiceEvent<CompositorService>>> =
    OnceCell::const_new();

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
    let Some(backend) = detect_backend() else {
        log::error!("No supported compositor backend found");
        let _ = tx.send(ServiceEvent::Error(
            "No supported compositor backend found".into(),
        ));
        return;
    };

    log::info!("Starting compositor event loop with {:?} backend", backend);

    let result = match backend {
        CompositorChoice::Hyprland => hyprland::run_listener(&tx).await,
        CompositorChoice::Niri => niri::run_listener(&tx).await,
        CompositorChoice::Mango => mangowc::run_listener(&tx).await,
    };

    if let Err(e) = result {
        log::error!("Compositor event loop failed: {}", e);
        let _ = tx.send(ServiceEvent::Error(e.to_string()));
    }
}

fn detect_backend() -> Option<CompositorChoice> {
    if hyprland::is_available() {
        Some(CompositorChoice::Hyprland)
    } else if niri::is_available() {
        Some(CompositorChoice::Niri)
    } else if mangowc::is_available() {
        Some(CompositorChoice::Mango)
    } else {
        None
    }
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
        Subscription::run_with_id(
            TypeId::of::<Self>(),
            channel(10, async move |mut output| {
                let mut rx = broadcaster_subscribe().await;

                // Send an empty Init with the correct backend to new subscribers
                // - assumes detect_backend is cheap
                if let Some(backend) = detect_backend() {
                    let empty_init = CompositorService {
                        state: CompositorState::default(),
                        backend,
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
            }),
        )
    }
}

impl Service for CompositorService {
    type Command = CompositorCommand;

    fn command(&mut self, command: Self::Command) -> Task<ServiceEvent<Self>> {
        let backend = self.backend;
        Task::perform(
            async move { execute_command(backend, command).await },
            |res| match res {
                Ok(()) => ServiceEvent::Update(CompositorEvent::ActionPerformed),
                Err(e) => ServiceEvent::Error(e),
            },
        )
    }
}

async fn execute_command(
    backend: CompositorChoice,
    command: CompositorCommand,
) -> Result<(), String> {
    match backend {
        CompositorChoice::Hyprland => hyprland::execute_command(command).await,
        CompositorChoice::Niri => niri::execute_command(command).await,
        CompositorChoice::Mango => mangowc::execute_command(command).await,
    }
    .map_err(|e| e.to_string())
}
