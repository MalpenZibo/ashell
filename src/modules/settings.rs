use gtk4::Widget;
use leptos::{create_memo, create_signal, ReadSignal, SignalGet, SignalSet};
use std::time::Duration;
use tokio::time::sleep;

use crate::{
    gtk4_wrapper::{container, label, spawn, Component},
    utils::battery::{get_battery_capacity, BatteryData},
};

pub fn settings() -> Widget {
    let (battery, set_battery) = create_signal(get_battery_capacity());

    spawn(async move {
        loop {
            sleep(Duration::from_secs(60)).await;
            set_battery.set(get_battery_capacity());
        }
    });

    container()
        .class(vec!["header-button", "settings"])
        .children(vec![battery_indicator(battery)])
        .into()
}

fn battery_indicator(battery: ReadSignal<Option<BatteryData>>) -> Widget {
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
