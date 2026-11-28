use futures::{
    Stream, StreamExt,
    channel::mpsc,
    stream::{self},
};

pub(crate) fn channel<T, F>(
    size: usize,
    f: impl FnOnce(mpsc::Sender<T>) -> F,
) -> impl Stream<Item = T>
where
    F: Future<Output = ()>,
{
    let (sender, receiver) = mpsc::channel(size);
    let runner = stream::once(f(sender)).filter_map(|_| async { None });

    stream::select(receiver, runner)
}

/// Sends `data` unless it equals the last snapshot sent, which `last` keeps.
/// Returns false once nobody listens to `output`.
#[cfg(any(
    feature = "bluetooth",
    feature = "brightness",
    feature = "keyboard_locks",
    feature = "upower"
))]
pub(crate) async fn send<T: Clone + PartialEq>(
    output: &mut mpsc::Sender<T>,
    last: &mut Option<T>,
    data: T,
) -> bool {
    use futures::SinkExt;

    if last.as_ref() == Some(&data) {
        return true;
    }

    *last = Some(data.clone());
    output.send(data).await.is_ok()
}
