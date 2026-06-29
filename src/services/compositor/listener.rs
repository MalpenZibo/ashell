use super::backend::Compositor;
use super::patch::StatePatch;
use super::types::{CompositorEvent, CompositorService, CompositorState};
use crate::services::ServiceEvent;
use std::future::Future;
use tokio::sync::{broadcast, mpsc};

const PATCH_CAPACITY: usize = 64;

/// Merge the patches the compositor emits into a single [`CompositorState`] and
/// re-broadcast a full snapshot on every change.
pub async fn run(
    compositor: &'static dyn Compositor,
    tx: broadcast::Sender<ServiceEvent<CompositorService>>,
) {
    let (patch_tx, mut patch_rx) = mpsc::channel::<StatePatch>(PATCH_CAPACITY);

    spawn_source(compositor.name(), compositor.run(patch_tx), &tx);

    let mut state = CompositorState::default();
    while let Some(patch) = patch_rx.recv().await {
        patch.apply_to(&mut state);
        let _ = tx.send(ServiceEvent::Update(CompositorEvent::StateChanged(
            Box::new(state.clone()),
        )));
    }
}

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
