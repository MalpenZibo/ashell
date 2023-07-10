use std::time::Duration;

use chrono::Local;
use futures_signals::signal::Mutable;

use crate::{
    reactive_gtk::{Component, Label, Node},
    utils::poll,
};

pub fn clock() -> Node {
    let get_date = || {
        let local = Local::now();
        let formatted_date = local.format("%a %d %b %R").to_string();

        formatted_date
    };
    let clock = Mutable::new(get_date());

    let clock1 = clock.clone();
    poll(
        move || {
            clock1.replace(get_date());
        },
        Duration::from_secs(20),
    );

    Label::default()
        .class(&["bg", "pl-4", "pr-2", "rounded-l-m"])
        .text_signal(clock.signal_cloned())
        .into()
}
