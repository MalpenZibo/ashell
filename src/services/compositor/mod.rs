pub mod hyprland;
pub mod niri;
pub mod types;

// Re-export types publicly so modules can use crate::services::compositor::CompositorState
pub use self::types::{
    CompositorChoice, CompositorCommand, CompositorEvent, CompositorService, CompositorState,
};

use crate::services::{ReadOnlyService, Service, ServiceEvent};
use iced::{Subscription, Task, stream::channel};
use std::{
    any::TypeId,
    ops::Deref,
    sync::{Arc, RwLock},
};

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
            channel(10, async |output| {
                let output = Arc::new(RwLock::new(output));

                let res = if hyprland::is_available() {
                    log::info!("Using Hyprland compositor backend");
                    hyprland::run_listener(output.clone()).await
                } else if niri::is_available() {
                    log::info!("Using Niri compositor backend");
                    niri::run_listener(output.clone()).await
                } else {
                    log::warn!("No supported compositor backend found (Hyprland or Niri)");
                    Err(anyhow::anyhow!(
                        "No supported compositor backend found (Hyprland or Niri)".to_string(),
                    ))
                };

                if let Err(e) = res {
                    log::error!("Failed to listen to compositor: {}", e);
                    if let Ok(mut o) = output.write() {
                        let _ = o.try_send(ServiceEvent::Error(e.to_string()));
                    }
                }

                std::future::pending().await
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
