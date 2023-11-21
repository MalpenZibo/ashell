use crate::reactive_gtk::{label, Align, Dynamic, EllipsizeMode, Node, NodeBuilder};
use futures_signals::signal::Mutable;
use hyprland::{data::Client, event_listener::EventListener, shared::HyprDataActiveOptional};

pub fn title() -> impl Into<Node> {
    let title: Mutable<Option<String>> = Mutable::new(
        Client::get_active()
            .ok()
            .and_then(|w| w.map(|w| w.initial_title)),
    );

    tokio::spawn({
        let title = title.clone();
        async move {
            let mut event_listener = EventListener::new();

            event_listener.add_active_window_change_handler({
                let title = title.clone();
                move |e| {
                    title.replace(e.map(|w| w.window_title));
                }
            });

            event_listener.add_window_close_handler({
                let title = title.clone();
                move |_| {
                    title.set(None);
                }
            });

            event_listener
                .start_listener_async()
                .await
                .expect("failed to start active window listener");
        }
    });

    label()
        .class(vec!["bar-item", "title"])
        .ellipsize(EllipsizeMode::Middle)
        .text::<String>(Dynamic(title.signal_ref(|t| {
            t.as_ref().map_or_else(|| "".to_string(), |t| t.to_owned())
        })))
        .visible(Dynamic(title.signal_ref(|t| t.is_some())))
}
