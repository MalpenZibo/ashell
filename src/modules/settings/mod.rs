use std::time::Duration;

use futures_signals::signal::Mutable;

use crate::{reactive_gtk::{Node, container, NodeBuilder}, nodes, utils::{battery::get_battery_capacity, poll}};

use self::battery::battery_indicator;

mod battery;

pub fn settings() -> impl Into<Node> {
    let battery = Mutable::new(get_battery_capacity());

    poll(
        {
            let battery = battery.clone();
            move || {
                battery.replace(get_battery_capacity());
            }
        },
        Duration::from_secs(60),
    );

    container()
        .class(vec!("bar-item", "settings"))
        .children(nodes!(battery_indicator(battery)))
}
