pub mod hyprland;
pub mod niri;
pub mod types;

// Re-export types publicly so modules can use crate::services::compositor::CompositorState
pub use self::types::{
    CompositorChoice, CompositorCommand, CompositorEvent, CompositorService, CompositorState,
};

use crate::services::{ReadOnlyService, Service, ServiceEvent};
use iced::futures::{SinkExt, StreamExt};
use iced::{Subscription, Task, stream::channel};
use std::{
    any::TypeId,
    ops::Deref,
    sync::{Arc, RwLock},
};
use tokio::sync::broadcast::{self, Sender};

impl Deref for CompositorService {
    type Target = CompositorState;
    fn deref(&self) -> &Self::Target {
        &self.state
    }
}

static STATE: parking_lot::Mutex<Option<Sender<ServiceEvent<CompositorService>>>> =
    parking_lot::const_mutex(None);

impl ReadOnlyService for CompositorService {
    type UpdateEvent = CompositorEvent;
    type Error = String;

    fn update(&mut self, event: Self::UpdateEvent) {
        match event {
            CompositorEvent::StateChanged(new_state) => {
                self.state = new_state;
            }
            CompositorEvent::ActionPerformed => {
                // No state change, just an action was performed
            }
        }
    }

    fn subscribe() -> Subscription<ServiceEvent<Self>> {
        let id = TypeId::of::<Self>();

        Subscription::run_with_id(
            id,
            channel(10, |mut output| async move {
                let mut rx = {
                    let mut state = STATE.lock();

                    if state.is_none() {
                        let (tx, _) = broadcast::channel(100);
                        *state = Some(tx.clone());

                        let tx_clone = tx.clone();

                        // Spawn compositor listener that broadcasts events
                        tokio::spawn(async move {
                            log::info!("Starting compositor listener (shared)");

                            // Create a channel to receive from hyprland/niri
                            let (internal_tx, mut internal_rx) =
                                iced::futures::channel::mpsc::channel(10);

                            // Spawn the actual listener
                            tokio::spawn(async move {
                                // Wrap internal_tx to send through it
                                let wrapped_output = Arc::new(RwLock::new(internal_tx));
                                let res = if hyprland::is_available() {
                                    log::info!("Using Hyprland compositor backend");
                                    hyprland::run_listener(wrapped_output.clone()).await
                                } else if niri::is_available() {
                                    log::info!("Using Niri compositor backend");
                                    niri::run_listener(wrapped_output.clone()).await
                                } else {
                                    log::error!("No supported compositor backend found");
                                    Err(anyhow::anyhow!("No supported compositor backend found"))
                                };

                                if let Err(e) = res {
                                    log::error!("Failed to start compositor listener: {}", e);
                                    if let Ok(mut o) = wrapped_output.clone().write() {
                                        let _ = o.try_send(ServiceEvent::Error(e.to_string()));
                                    }
                                }
                            });

                            // Forward from internal channel to broadcast
                            while let Some(event) = internal_rx.next().await {
                                let _ = tx_clone.send(event);
                            }

                            log::error!("Compositor listener ended");
                        });
                    }

                    state.as_ref().unwrap().subscribe()
                };

                loop {
                    match rx.recv().await {
                        Ok(event) => {
                            if output.send(event).await.is_err() {
                                log::error!("Compositor subscriber output closed");
                                break;
                            }
                        }
                        Err(broadcast::error::RecvError::Lagged(n)) => {
                            log::warn!("Subscriber lagged by {} messages", n);
                        }
                        Err(broadcast::error::RecvError::Closed) => {
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
        let choice = self.backend;
        Task::perform(
            async move {
                match choice {
                    CompositorChoice::Hyprland => hyprland::execute_command(command)
                        .await
                        .map_err(|e| e.to_string()),
                    CompositorChoice::Niri => niri::execute_command(command)
                        .await
                        .map_err(|e| e.to_string()),
                }
            },
            |res| match res {
                // Right now this informs the compositor something happend - this internally is a
                // noop. In the future this can be extended to trigger specialized updates.
                Ok(_) => ServiceEvent::Update(CompositorEvent::ActionPerformed),
                Err(e) => ServiceEvent::Error(e),
            },
        )
    }
}
