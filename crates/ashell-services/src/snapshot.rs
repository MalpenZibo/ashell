use futures::{Stream, stream};
use tokio::sync::watch;

/// The current value, then one per change. A consumer that falls behind
/// gets the latest, never a stale one.
pub(crate) fn snapshots<T: Clone + Send + Sync + 'static>(
    mut data: watch::Receiver<T>,
) -> impl Stream<Item = T> + Send + 'static {
    data.mark_changed();
    stream::unfold(data, |mut data| async move {
        data.changed().await.ok()?;
        let snapshot = data.borrow_and_update().clone();
        Some((snapshot, data))
    })
}
