use std::time::Duration;

use chrono::Local;
use gtk4::Widget;
use leptos::{create_signal, SignalSet};
use tokio::time::sleep;

use crate::gtk4_wrapper::{label, spawn, Component};

pub fn clock() -> Widget {
    let get_date = || {
        let local = Local::now();
        let formatted_date = local.format("%a %d %b %R").to_string();

        formatted_date
    };

    let (clock, set_clock) = create_signal(get_date());

    spawn(async move {
        loop {
            sleep(Duration::from_secs(20)).await;
            set_clock.set(get_date());
        }
    });

    label()
        .class(vec!["header-label", "clock"])
        .text(clock)
        .into()
}
