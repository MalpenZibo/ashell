//! Whether the seat is in use, through the Wayland ext-idle-notify protocol.

use std::os::fd::{AsFd, AsRawFd, RawFd};
use tokio::io::{Interest, unix::AsyncFd};
use wayland_client::{
    Connection, Dispatch, EventQueue, Proxy, QueueHandle,
    backend::WaylandError,
    delegate_noop,
    protocol::{
        wl_registry::{self, WlRegistry},
        wl_seat::WlSeat,
    },
};
use wayland_protocols::ext::idle_notify::v1::client::{
    ext_idle_notification_v1::{self, ExtIdleNotificationV1},
    ext_idle_notifier_v1::ExtIdleNotifierV1,
};

/// How long the seat goes without input before it counts as idle.
const IDLE_AFTER_MS: u32 = 5_000;

/// A connection of its own that the compositor tells when the seat goes
/// idle and when it is used again. Read on the runtime, it goes away with
/// whoever polls it.
pub struct Activity {
    socket: AsyncFd<Socket>,
    queue: EventQueue<State>,
    state: State,
}

/// The connection, readable through its socket.
struct Socket(Connection);

impl AsRawFd for Socket {
    fn as_raw_fd(&self) -> RawFd {
        self.0.as_fd().as_raw_fd()
    }
}

impl Activity {
    /// Asks for the globals without waiting for them: a compositor without
    /// the protocol never tells a change.
    pub fn connect() -> anyhow::Result<Self> {
        let connection = Connection::connect_to_env()?;
        let queue = connection.new_event_queue();
        connection.display().get_registry(&queue.handle(), ());
        connection.flush()?;

        // SAFETY: the socket owns the connection, so its fd stays open
        // until the socket is dropped, and `as_raw_fd` always returns it.
        let socket =
            unsafe { AsyncFd::register_with_interest(Socket(connection), Interest::READABLE) }?;

        Ok(Self {
            socket,
            queue,
            state: State::default(),
        })
    }

    /// The next change: `false` when the seat goes idle, `true` when it is
    /// used again. An error when the connection is lost.
    pub async fn changed(&mut self) -> anyhow::Result<bool> {
        loop {
            self.queue.dispatch_pending(&mut self.state)?;

            if let Some(active) = self.state.change.take() {
                return Ok(active);
            }

            self.socket.get_ref().0.flush()?;

            // The only reader of its connection, it can wait first and take
            // the read guard after, which can't be held across an await.
            let mut ready = self.socket.readable().await?;
            let Some(read) = self.queue.prepare_read() else {
                continue;
            };

            match read.read() {
                Ok(_) => {}
                Err(WaylandError::Io(err)) if err.kind() == std::io::ErrorKind::WouldBlock => {
                    ready.clear_ready();
                }
                Err(err) => return Err(err.into()),
            }
        }
    }
}

#[derive(Default)]
struct State {
    seat: Option<WlSeat>,
    notifier: Option<ExtIdleNotifierV1>,
    notification: Option<ExtIdleNotificationV1>,
    /// The latest change not told yet.
    change: Option<bool>,
}

impl State {
    /// Asks to be told once both globals are bound.
    fn watch(&mut self, handle: &QueueHandle<Self>) {
        let (Some(seat), Some(notifier)) = (&self.seat, &self.notifier) else {
            return;
        };
        if self.notification.is_some() {
            return;
        }

        // Since version 2 idle inhibitors don't count: a video playing presses no keys
        self.notification = Some(if notifier.version() >= 2 {
            notifier.get_input_idle_notification(IDLE_AFTER_MS, seat, handle, ())
        } else {
            notifier.get_idle_notification(IDLE_AFTER_MS, seat, handle, ())
        });
    }
}

impl Dispatch<WlRegistry, ()> for State {
    fn event(
        state: &mut Self,
        registry: &WlRegistry,
        event: wl_registry::Event,
        _: &(),
        _: &Connection,
        handle: &QueueHandle<Self>,
    ) {
        let wl_registry::Event::Global {
            name,
            interface,
            version,
        } = event
        else {
            return;
        };

        match interface.as_str() {
            "wl_seat" if state.seat.is_none() => {
                state.seat = Some(registry.bind(name, 1, handle, ()));
            }
            "ext_idle_notifier_v1" => {
                state.notifier = Some(registry.bind(name, version.min(2), handle, ()));
            }
            _ => return,
        }

        state.watch(handle);
    }
}

impl Dispatch<ExtIdleNotificationV1, ()> for State {
    fn event(
        state: &mut Self,
        _: &ExtIdleNotificationV1,
        event: ext_idle_notification_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        state.change = match event {
            ext_idle_notification_v1::Event::Idled => Some(false),
            ext_idle_notification_v1::Event::Resumed => Some(true),
            _ => state.change,
        };
    }
}

delegate_noop!(State: ignore WlSeat);
delegate_noop!(State: ExtIdleNotifierV1);
