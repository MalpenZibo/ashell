use crate::gtk4_wrapper::{label, spawn, Align, Component};
use gtk4::Widget;
use hyprland::{
    async_closure, data::Client, event_listener::AsyncEventListener, shared::HyprDataActiveOptional,
};
use leptos::{create_memo, create_signal, SignalGet, SignalUpdate};

pub fn title() -> Widget {
    let (title, set_title) = create_signal::<Option<String>>(
        Client::get_active()
            .ok()
            .and_then(|w| w.map(|w| w.initial_title)),
    );

    spawn({
        async move {
            let mut event_listener = AsyncEventListener::new();

            event_listener.add_active_window_change_handler(async_closure! { move |e| {
                set_title.update(|title| *title = e.map(|w| w.window_title));
            }});

            let _ = event_listener.start_listener_async().await;
        }
    });

    let formatted = create_memo(move |_| title.get().unwrap_or("".to_owned()));
    let visible = create_memo(move |_| title.get().is_some());

    label()
        .class(vec!["header-label"])
        .valign(Align::Center)
        .vexpand(false)
        .text(formatted)
        .visible(visible)
        .into()
}
