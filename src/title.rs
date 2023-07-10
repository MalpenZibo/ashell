use futures_signals::signal::Mutable;
use hyprland::{data::Client, event_listener::EventListener, shared::HyprDataActiveOptional};

use crate::reactive_gtk::{Component, Label, Node};

pub fn title() -> Node {
    let get_title = || Client::get_active().ok().flatten().map(|w| w.title);
    let title = Mutable::new(get_title());

    let title1 = title.clone();
    tokio::spawn(async move {
        let mut event_listener = EventListener::new();

        event_listener.add_active_window_change_handler(move |e| {
            title1.replace(e.map(|w| w.window_title));
        });

        event_listener
            .start_listener_async()
            .await
            .expect("failed to start active window listener");
    });

    Label::default()
        .class(&["bg", "ph-4", "rounded-m"])
        .text_signal(
            title.signal_ref(|t| t.as_ref().map_or_else(|| "".to_owned(), |t| t.to_owned())),
        )
        .visible_signal(title.signal_ref(|t| t.is_some()))
        .into()
}
