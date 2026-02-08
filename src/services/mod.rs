pub mod hyprland;
pub mod niri;
pub mod types;

pub use self::types::{CompositorChoice, CompositorCommand, CompositorState};

use guido::prelude::*;

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
    state: Signal<CompositorState>,
) -> Service<CompositorCommand> {
    let state_writer = state.writer();
    create_service(move |rx, ctx| {
        let Some(backend) = detect_backend() else {
            log::error!("No supported compositor backend found");
            return;
        };

        log::info!("Starting compositor service with {:?} backend", backend);

        let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");

        rt.block_on(async {
            // Spawn the event listener
            let listener_handle = tokio::spawn({
                let state_writer = state_writer;
                async move {
                    let result = match backend {
                        CompositorChoice::Hyprland => hyprland::run_listener(state_writer).await,
                        CompositorChoice::Niri => niri::run_listener(state_writer).await,
                    };
                    if let Err(e) = result {
                        log::error!("Compositor event loop failed: {}", e);
                    }
                }
            });

            // Poll for commands
            loop {
                if !ctx.is_running() {
                    listener_handle.abort();
                    break;
                }

                // Drain all pending commands
                while let Ok(cmd) = rx.try_recv() {
                    let backend = backend;
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

                // Check if listener died
                if listener_handle.is_finished() {
                    log::error!("Compositor listener exited unexpectedly");
                    break;
                }

                tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            }
        });
    })
}
