//! Whether the seat is in use, through the Wayland ext-idle-notify protocol.

use std::thread;
use tokio::sync::watch;
use wayland_client::{
    Connection, Dispatch, Proxy, QueueHandle, delegate_noop,
    globals::{GlobalListContents, registry_queue_init},
    protocol::{wl_registry::WlRegistry, wl_seat::WlSeat},
};
use wayland_protocols::ext::idle_notify::v1::client::{
    ext_idle_notification_v1::{self, ExtIdleNotificationV1},
    ext_idle_notifier_v1::ExtIdleNotifierV1,
};

/// How long the seat goes without input before it counts as idle.
const IDLE_AFTER_MS: u32 = 5_000;

/// `true` while the seat had input in the last [`IDLE_AFTER_MS`]. The
/// channel closes if the Wayland connection is lost.
pub fn watch() -> anyhow::Result<watch::Receiver<bool>> {
    let connection = Connection::connect_to_env()?;
    let (globals, mut queue) = registry_queue_init::<State>(&connection)?;
    let handle = queue.handle();
    let seat: WlSeat = globals.bind(&handle, 1..=1, ())?;
    let notifier: ExtIdleNotifierV1 = globals.bind(&handle, 1..=2, ())?;
    // Since version 2 idle inhibitors don't count: a video playing presses no keys
    let notification = if notifier.version() >= 2 {
        notifier.get_input_idle_notification(IDLE_AFTER_MS, &seat, &handle, ())
    } else {
        notifier.get_idle_notification(IDLE_AFTER_MS, &seat, &handle, ())
    };

    let (sender, receiver) = watch::channel(true);
    let mut state = State { sender };
    thread::Builder::new()
        .name("keyboard-locks-idle".into())
        .spawn(move || {
            let _alive = (connection, seat, notifier, notification);
            while !state.sender.is_closed() && queue.blocking_dispatch(&mut state).is_ok() {}
        })?;

    Ok(receiver)
}

struct State {
    sender: watch::Sender<bool>,
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
        let active = match event {
            ext_idle_notification_v1::Event::Idled => false,
            ext_idle_notification_v1::Event::Resumed => true,
            _ => return,
        };
        state.sender.send_replace(active);
    }
}

impl Dispatch<WlRegistry, GlobalListContents> for State {
    fn event(
        _: &mut Self,
        _: &WlRegistry,
        _: <WlRegistry as Proxy>::Event,
        _: &GlobalListContents,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

delegate_noop!(State: ignore WlSeat);
delegate_noop!(State: ExtIdleNotifierV1);
