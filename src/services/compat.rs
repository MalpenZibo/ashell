//! Minimal mimic of the iced surface used by upstream ashell services, so
//! their files can be copied here nearly verbatim (only the `use iced::...`
//! lines change).
//!
//! Upstream services are state machines pushing `ServiceEvent`s into a
//! stream built with `Subscription::run_with` + `stream::channel`, plus a
//! `command()` returning a `Task`. This module provides those exact API
//! shapes, and [`run_service`] drives them from a guido `create_service`
//! task: events fold into the service value via `update()`, and every
//! change is published to a guido signal with `set_always` (services have
//! no meaningful equality — every publish notifies; memos cut off at the
//! consumer edge).
//!
//! Cancellation is inherited from guido: `create_service` aborts the task
//! on owner cleanup, and everything here is polled by that task — no
//! detached loops, no `is_running` polling.

use std::any::TypeId;
use std::future::Future;

use futures::channel::mpsc;
use futures::future::BoxFuture;
use futures::stream::{BoxStream, Stream, StreamExt};
use guido::prelude::*;

/// Event stream item produced by a service's `subscribe()` — identical to
/// upstream ashell's.
#[derive(Debug, Clone)]
pub enum ServiceEvent<S: ReadOnlyService> {
    Init(S),
    Update(S::UpdateEvent),
    Error(S::Error),
}

pub trait ReadOnlyService: Sized {
    type UpdateEvent;
    type Error: Clone;

    fn update(&mut self, event: Self::UpdateEvent);

    fn subscribe() -> Subscription<ServiceEvent<Self>>;
}

pub trait Service: ReadOnlyService {
    type Command;

    fn command(&mut self, command: Self::Command) -> Task<ServiceEvent<Self>>;
}

/// `iced::Subscription` mimic: just a boxed event stream.
pub struct Subscription<T>(BoxStream<'static, T>);

impl<T: Send + 'static> Subscription<T> {
    /// Matches iced's `Subscription::run_with(id, |_| stream)` call shape.
    /// The id gives iced subscription identity across view rebuilds; here
    /// each service is subscribed exactly once by its runner, so it is
    /// ignored.
    pub fn run_with<St>(_id: TypeId, make: impl FnOnce(&()) -> St) -> Self
    where
        St: Stream<Item = T> + Send + 'static,
    {
        Subscription(make(&()).boxed())
    }
}

/// `iced::stream::channel` mimic. The driver future and the receiver are
/// polled together by the consumer, so aborting the runner task drops the
/// whole pipeline — nothing detached is left behind.
pub fn channel<T, F, Fut>(capacity: usize, f: F) -> impl Stream<Item = T> + Send
where
    T: Send + 'static,
    F: FnOnce(mpsc::Sender<T>) -> Fut + Send + 'static,
    Fut: Future<Output = ()> + Send + 'static,
{
    let (tx, rx) = mpsc::channel(capacity);
    let driver = futures::stream::once(f(tx)).map(|_| None);
    futures::stream::select(rx.map(Some), driver).filter_map(|item| async move { item })
}

/// `iced::Task` mimic covering the constructors upstream services use.
pub struct Task<T>(TaskInner<T>);

enum TaskInner<T> {
    None,
    Perform(BoxFuture<'static, T>),
}

impl<T: Send + 'static> Task<T> {
    pub fn none() -> Self {
        Task(TaskInner::None)
    }

    pub fn perform<A: Send + 'static>(
        future: impl Future<Output = A> + Send + 'static,
        map: impl FnOnce(A) -> T + Send + 'static,
    ) -> Self {
        Task(TaskInner::Perform(Box::pin(
            async move { map(future.await) },
        )))
    }
}

/// `iced::widget::image` mimic: a data-only handle (the UI layer converts
/// it to a guido `ImageSource`).
pub mod image {
    use std::path::PathBuf;

    #[derive(Debug, Clone, PartialEq)]
    pub enum Handle {
        Path(PathBuf),
        Bytes(bytes::Bytes),
    }

    impl Handle {
        pub fn from_path(path: impl Into<PathBuf>) -> Self {
            Self::Path(path.into())
        }

        pub fn from_bytes(bytes: impl Into<bytes::Bytes>) -> Self {
            Self::Bytes(bytes.into())
        }
    }
}

/// `iced::widget::svg` mimic, same idea as [`image`].
pub mod svg {
    use std::path::PathBuf;

    #[derive(Debug, Clone, PartialEq)]
    pub struct Handle(pub PathBuf);

    impl Handle {
        pub fn from_path(path: impl Into<PathBuf>) -> Self {
            Self(path.into())
        }
    }
}

/// Reactive handle to a running upstream-style service: `None` until the
/// service emits `Init`.
///
/// Snapshots are published with `set_always` — services hold proxies and
/// channel handles, so equality is meaningless and every publish must
/// notify. Fine-grained cutoff belongs to `create_memo` at the consumer
/// edge (extract the value you render; the memo's `PartialEq` stops the
/// propagation when it didn't change).
pub type ServiceSignal<S> = RwSignal<Option<S>>;

/// Drive an upstream-style service from a guido service task.
///
/// Subscribes to the service's event stream, folds events into the service
/// value via `update()`, publishes every change as an always-notifying
/// snapshot, and executes `command()` tasks (their resulting events feed back into
/// the same loop, like iced's runtime does).
/// Bench-only gate: ASHELL_BENCH_ONLY=NameA,NameB runs only the services
/// whose type name contains one of the given fragments.
pub fn run_service<S>() -> (ServiceSignal<S>, guido::prelude::Service<S::Command>)
where
    S: Service + Clone + Send + 'static,
    S::UpdateEvent: Send + 'static,
    S::Command: Send + 'static,
    S::Error: Send + std::fmt::Debug + 'static,
{
    run_service_hooked::<S>(|_| {})
}

/// [`run_service`] for services without a command channel.
pub fn run_readonly_service<S>() -> ServiceSignal<S>
where
    S: ReadOnlyService + Clone + Send + 'static,
    S::UpdateEvent: Send + 'static,
    S::Error: Send + std::fmt::Debug + 'static,
{
    run_readonly_service_hooked::<S>(|_| {})
}

/// [`run_readonly_service`] plus the raw event hook.
pub fn run_readonly_service_hooked<S>(
    mut hook: impl FnMut(&ServiceEvent<S>) + Send + 'static,
) -> ServiceSignal<S>
where
    S: ReadOnlyService + Clone + Send + 'static,
    S::UpdateEvent: Send + 'static,
    S::Error: Send + std::fmt::Debug + 'static,
{
    let signal = create_signal(None::<S>);
    let writer = signal.writer();

    if let Ok(only) = std::env::var("ASHELL_BENCH_ONLY") {
        let name = std::any::type_name::<S>();
        if !only.split(',').any(|frag| name.contains(frag)) {
            return signal;
        }
    }

    let _svc = create_service::<(), _, _>(move |_rx, _ctx| async move {
        let mut events = S::subscribe().0;
        let mut service: Option<S> = None;

        while let Some(event) = events.next().await {
            hook(&event);
            match event {
                ServiceEvent::Init(s) => {
                    service = Some(s.clone());
                    log::debug!("publish {}", std::any::type_name::<S>());
                    writer.set_always(Some(s));
                }
                ServiceEvent::Update(update) => {
                    if let Some(s) = service.as_mut() {
                        s.update(update);
                        log::debug!("publish {}", std::any::type_name::<S>());
                        writer.set_always(Some(s.clone()));
                    }
                }
                ServiceEvent::Error(err) => {
                    log::error!("service {} error: {err:?}", std::any::type_name::<S>());
                }
            }
        }
    });

    signal
}

/// [`run_service`] plus a raw event hook, for consumers that react to the
/// events themselves rather than to folded snapshots (the notifications
/// list, the OSD). The hook runs on the service task before the event is
/// folded, mirroring how upstream's app.rs sees `ServiceEvent`s.
pub fn run_service_hooked<S>(
    mut hook: impl FnMut(&ServiceEvent<S>) + Send + 'static,
) -> (ServiceSignal<S>, guido::prelude::Service<S::Command>)
where
    S: Service + Clone + Send + 'static,
    S::UpdateEvent: Send + 'static,
    S::Command: Send + 'static,
    S::Error: Send + std::fmt::Debug + 'static,
{
    let signal = create_signal(None::<S>);
    let writer = signal.writer();

    if let Ok(only) = std::env::var("ASHELL_BENCH_ONLY") {
        let name = std::any::type_name::<S>();
        if !only.split(',').any(|frag| name.contains(frag)) {
            return (
                signal,
                create_service::<S::Command, _, _>(|_rx, _ctx| async {}),
            );
        }
    }

    let svc = create_service::<S::Command, _, _>(move |mut rx, _ctx| async move {
        let mut events = S::subscribe().0;
        // Results of command() tasks re-enter the event loop here
        let (task_tx, mut task_rx) = tokio::sync::mpsc::unbounded_channel::<ServiceEvent<S>>();
        let mut service: Option<S> = None;

        let publish = move |s: S| {
            log::debug!("publish {}", std::any::type_name::<S>());
            writer.set_always(Some(s));
        };

        loop {
            let event = tokio::select! {
                ev = events.next() => match ev {
                    Some(ev) => ev,
                    None => break,
                },
                Some(ev) = task_rx.recv() => ev,
                cmd = rx.recv() => match cmd {
                    Some(cmd) => {
                        if let Some(s) = service.as_mut() {
                            let task = s.command(cmd);
                            // command() may mutate the service synchronously
                            publish(s.clone());
                            if let Task(TaskInner::Perform(fut)) = task {
                                let task_tx = task_tx.clone();
                                tokio::spawn(async move {
                                    let _ = task_tx.send(fut.await);
                                });
                            }
                        }
                        continue;
                    }
                    None => break,
                },
            };

            hook(&event);

            match event {
                ServiceEvent::Init(s) => {
                    service = Some(s.clone());
                    publish(s);
                }
                ServiceEvent::Update(update) => {
                    if let Some(s) = service.as_mut() {
                        s.update(update);
                        publish(s.clone());
                    }
                }
                ServiceEvent::Error(err) => {
                    log::error!("service {} error: {err:?}", std::any::type_name::<S>());
                }
            }
        }
    });

    (signal, svc)
}
