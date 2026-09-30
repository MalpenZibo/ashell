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
