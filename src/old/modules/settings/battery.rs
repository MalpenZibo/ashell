use crate::{
    components::icons::icon_with_text,
    reactive_gtk::{Dynamic, Node, NodeBuilder},
    utils::battery::BatteryData,
};
use futures_signals::signal::Mutable;

pub fn battery_indicator(battery: Mutable<Option<BatteryData>>) -> impl Into<Node> {
    icon_with_text::<String, &str>(
        Dynamic(battery.signal_ref(|b| b.map(|b| b.get_icon()).unwrap_or_default())),
        Dynamic(battery.signal_ref(|b| b.map(|b| format!("{}%", b.capacity)).unwrap_or_default())),
        Dynamic(battery.signal_ref(|b| b.map(|b| b.get_class()).unwrap_or_default())),
    )
    .into()
    .visible(Dynamic(battery.signal_ref(|b| b.is_some())))
}

pub fn battery_settings_indicator(battery: Mutable<Option<BatteryData>>) -> impl Into<Node> {
    icon_with_text::<String, &str>(
        Dynamic(battery.signal_ref(|b| b.map(|b| b.get_icon()).unwrap_or_default())),
        Dynamic(battery.signal_ref(|b| b.map(|b| format!("{}%", b.capacity)).unwrap_or_default())),
        Dynamic(battery.signal_ref(|b| {
            [
                vec!["settings-item"],
                b.map(|b| b.get_class()).unwrap_or_default(),
            ]
            .concat()
        })),
    )
    .into()
    .visible(Dynamic(battery.signal_ref(|b| b.is_some())))
}
