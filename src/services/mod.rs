pub mod hyprland;
pub mod niri;
pub mod types;

pub use self::types::{
    CompositorChoice, CompositorCommand, CompositorMonitor, CompositorState,
    CompositorStateSignals, CompositorStateWriters, CompositorWorkspace,
};

use guido::prelude::*;
use std::time::Duration;

fn detect_backend() -> Option<CompositorChoice> {
    if hyprland::is_available() {
        Some(CompositorChoice::Hyprland)
    } else if niri::is_available() {
        Some(CompositorChoice::Niri)
    } else {
        None
    }
}

pub fn start_compositor_service(
    state_writers: CompositorStateWriters,
) -> Service<CompositorCommand> {
    create_service(move |mut rx, ctx| async move {
        let Some(backend) = detect_backend() else {
            log::error!("No supported compositor backend found");
            return;
        };

        log::info!("Starting compositor service with {:?} backend", backend);

        // Spawn the event listener
        let listener_handle = tokio::spawn({
            let state_writers = state_writers;
            async move {
                let result = match backend {
                    CompositorChoice::Hyprland => hyprland::run_listener(state_writers).await,
                    CompositorChoice::Niri => niri::run_listener(state_writers).await,
                };
                if let Err(e) = result {
                    log::error!("Compositor event loop failed: {}", e);
                }
            }
        });

        // Main loop — recv commands or check liveness
        loop {
            tokio::select! {
                cmd = rx.recv() => {
                    match cmd {
                        Some(cmd) => {
                            tokio::spawn(async move {
                                let result = match backend {
                                    CompositorChoice::Hyprland => {
                                        hyprland::execute_command(cmd).await
                                    }
                                    CompositorChoice::Niri => niri::execute_command(cmd).await,
                                };
                                if let Err(e) = result {
                                    log::error!("Failed to execute compositor command: {}", e);
                                }
                            });
                        }
                        None => {
                            // Channel closed
                            listener_handle.abort();
                            break;
                        }
                    }
                }
                _ = tokio::time::sleep(Duration::from_millis(50)) => {
                    if !ctx.is_running() {
                        listener_handle.abort();
                        break;
                    }
                    if listener_handle.is_finished() {
                        log::error!("Compositor listener exited unexpectedly");
                        break;
                    }
                }
            }
        }
    })
}
