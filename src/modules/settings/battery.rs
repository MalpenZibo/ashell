use futures_signals::signal::Mutable;
use crate::{
    reactive_gtk::{label, Dynamic, Node, NodeBuilder},
    utils::battery::BatteryData,
};

pub fn battery_indicator(battery: Mutable<Option<BatteryData>>) -> impl Into<Node> {
    label()
        .class(Dynamic(
            battery.signal_ref(|b| b.map_or(vec![], |b| b.get_class())),
        ))
        .text(Dynamic(battery.signal_ref(|b| {
            b.map_or("".to_string(), |b| {
                format!("{} {}%", b.get_icon(), b.capacity)
            })
        })))
        .visible(Dynamic(battery.signal_ref(|b| b.is_some())))
}

pub fn battery_settings_indicator(battery: Mutable<Option<BatteryData>>) -> impl Into<Node> {
    label()
        .class(Dynamic(battery.signal_ref(|b| {
            [b.map_or(vec![], |b| b.get_class()), vec!["settings-item"]].concat()
        })))
        .text(Dynamic(battery.signal_ref(|b| {
            b.map_or("".to_string(), |b| {
                format!("{} {}%", b.get_icon(), b.capacity)
            })
        })))
        .visible(Dynamic(battery.signal_ref(|b| b.is_some())))
}
