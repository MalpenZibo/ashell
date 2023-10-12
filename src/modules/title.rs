use crate::gtk4_wrapper::{label, spawn, Align, Component, EllipsizeMode};
use gtk4::Widget;
use hyprland::{
    async_closure, data::Client, event_listener::AsyncEventListener, shared::HyprDataActiveOptional,
};
use leptos::{create_memo, create_signal, SignalGet, SignalSet, SignalUpdate};

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

            event_listener.add_window_close_handler(async_closure!(move |_| {
                set_title.set(None);
            }));

            let _ = event_listener.start_listener_async().await;
        }
    });

    let formatted = create_memo(move |_| title.get().unwrap_or("".to_owned()));
    let visible = create_memo(move |_| title.get().is_some());

    label()
        .class(vec!["header-label"])
        .valign(Align::Center)
        .vexpand(false)
        .ellipsize(EllipsizeMode::Middle)
        .text(formatted)
        .visible(visible)
        .into()
}
