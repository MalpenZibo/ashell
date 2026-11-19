//! Whether the seat is in use, through the Wayland ext-idle-notify protocol.

use log::{info, warn};
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
/// channel closes when the compositor can't tell, or the connection is lost.
pub fn watch() -> watch::Receiver<bool> {
    let (sender, receiver) = watch::channel(true);
    // Its roundtrips block, so the connection is made on the thread too
    let spawned = thread::Builder::new()
        .name("keyboard-locks-idle".into())
        .spawn(move || {
            if let Err(err) = follow(State { sender }) {
                info!("Keyboard locks are polled even when idle: {err}");
            }
        });
    if let Err(err) = spawned {
        warn!("Failed to follow the seat's activity: {err}");
    }
    receiver
}

fn follow(mut state: State) -> anyhow::Result<()> {
    let connection = Connection::connect_to_env()?;
    let (globals, mut queue) = registry_queue_init::<State>(&connection)?;
    let handle = queue.handle();
    let seat: WlSeat = globals.bind(&handle, 1..=1, ())?;
    let notifier: ExtIdleNotifierV1 = globals.bind(&handle, 1..=2, ())?;
    // Since version 2 idle inhibitors don't count: a video playing presses no keys
    let _notification = if notifier.version() >= 2 {
        notifier.get_input_idle_notification(IDLE_AFTER_MS, &seat, &handle, ())
    } else {
        notifier.get_idle_notification(IDLE_AFTER_MS, &seat, &handle, ())
    };

    while !state.sender.is_closed() {
        queue.blocking_dispatch(&mut state)?;
    }
    Ok(())
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
