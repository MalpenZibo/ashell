use crate::{
    reactive_gtk::{label, Dynamic, Node, NodeBuilder},
    utils::poll,
};
use chrono::Local;
use futures_signals::signal::Mutable;
use std::time::Duration;

pub fn clock() -> impl Into<Node> {
    let get_date = || {
        let local = Local::now();
        local.format("%a %d %b %R").to_string()
    };

    let clock: Mutable<String> = Mutable::new(get_date());

    poll(
        {
            let clock = clock.clone();
            move || {
                clock.replace(get_date());
            }
        },
        Duration::from_secs(20),
    );

    label()
        .class(vec!["bar-item", "clock"])
        .text::<String>(Dynamic(clock.signal_cloned()))
}
