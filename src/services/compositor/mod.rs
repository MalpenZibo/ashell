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

                if let Err(e) = hyprland::run_listener(output.clone()).await {
                    log::warn!("Failed to listen to hyprland: {}", e);
                    log::warn!("Listening for niri instead");

                    let l = niri::run_listener(output.clone()).await;
                    if let Err(e) = l {
                        log::error!("Failed to listen to niri: {}", e);

                        if let Ok(mut o) = output.write() {
                            let _ = o.try_send(ServiceEvent::Error(e.to_string()));
                        }
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
                // We don't necessarily need to trigger a refresh here as Hyprland will emit an event
                Ok(_) => ServiceEvent::Update(CompositorEvent::ActionPerformed),
                /*StateChanged(
                    // Ideally we wouldn't send empty state here, but the listener will trigger real updates.
                    // Using Default is safe if we just want to wake up, but better to let the listener handle it.
                    CompositorState::default(),
                )),*/
                Err(e) => ServiceEvent::Error(e),
            },
        )
    }
}
