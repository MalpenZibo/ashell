use gtk4::Widget;
use leptos::{create_memo, ReadSignal, SignalGet};

use crate::{
    gtk4_wrapper::{label, Component},
    utils::battery::BatteryData,
};

pub fn battery_indicator(battery: ReadSignal<Option<BatteryData>>) -> Widget {
    let format = create_memo(move |_| {
        let battery = battery.get();
        format!(
            "{} {}%",
            battery.map_or("".to_string(), |b| b.get_icon().into()),
            battery.map_or(0, |b| b.capacity)
        )
    });
    let class = create_memo(move |_| {
        let battery = battery.get();
        battery.map_or(vec![], |b| b.get_class())
    });
    let visible = create_memo(move |_| battery.get().is_some());

    label().class(class).text(format).visible(visible).into()
}

pub fn battery_settings_label(battery: ReadSignal<Option<BatteryData>>) -> Widget {
    let format = create_memo(move |_| {
        let battery = battery.get();
        format!(
            "{} {}%",
            battery.map_or("".to_string(), |b| b.get_icon().into()),
            battery.map_or(0, |b| b.capacity)
        )
    });
    let class = create_memo(move |_| {
        let battery = battery.get();
        [
            battery.map_or(vec![], |b| b.get_class()),
            vec!["settings-label"],
        ]
        .concat()
    });
    let visible = create_memo(move |_| battery.get().is_some());

    label().class(class).text(format).visible(visible).into()
}
