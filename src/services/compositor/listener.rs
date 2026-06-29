use super::patch::StatePatch;
use super::types::{CompositorChoice, CompositorEvent, CompositorService, CompositorState};
use super::{hyprland, niri};
use crate::services::ServiceEvent;
use std::future::Future;
use tokio::sync::{broadcast, mpsc};

const PATCH_CAPACITY: usize = 64;

/// Drive the compositor backend: spawn its state sources, merge their patches
/// into a single [`CompositorState`] and re-broadcast a full snapshot on every
/// change so downstream subscribers keep the existing wire format.
pub async fn run(choice: CompositorChoice, tx: broadcast::Sender<ServiceEvent<CompositorService>>) {
    let (patch_tx, mut patch_rx) = mpsc::channel::<StatePatch>(PATCH_CAPACITY);

    match choice {
        CompositorChoice::Hyprland => {
            spawn_source("hyprland", hyprland::run_listener(patch_tx), &tx);
        }
        CompositorChoice::Niri => {
            spawn_source("niri", niri::run_listener(patch_tx), &tx);
        }
    }

    let mut state = CompositorState::default();
    while let Some(patch) = patch_rx.recv().await {
        patch.apply_to(&mut state);
        let _ = tx.send(ServiceEvent::Update(CompositorEvent::StateChanged(
            Box::new(state.clone()),
        )));
    }
}

/// Spawn a state source future, logging and surfacing its terminal error.
fn spawn_source<F>(
    name: &'static str,
    fut: F,
    tx: &broadcast::Sender<ServiceEvent<CompositorService>>,
) where
    F: Future<Output = anyhow::Result<()>> + Send + 'static,
{
    let tx = tx.clone();
    tokio::spawn(async move {
        if let Err(e) = fut.await {
            log::error!("compositor source {name} failed: {e}");
            let _ = tx.send(ServiceEvent::Error(e.to_string()));
        }
    });
}
