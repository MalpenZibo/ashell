use std::time::Duration;

use crate::{
    battery::get_battery_capacity, clock::clock, reactive_gtk::CenterBox, system_info::system_info,
    title::title, updates::update_button, utils::poll, workspaces::worspaces,
};

use futures_signals::signal::Mutable;

use crate::{
    launcher::launch_rofi,
    reactive_gtk::{Box, Component, Label, Node},
};

fn application_button() -> Node {
    Box::default()
        .class(&["rounded-m", "bg", "interactive"])
        .on_click(launch_rofi)
        .children(vec![Label::default().class(&["ph-2"]).text("󱗼").into()])
        .into()
}

fn right() -> Node {
    Box::default()
        .spacing(4)
        .children(vec![
            system_info(),
            Box::default().children(vec![clock(), settings()]).into(),
        ])
        .into()
}
fn settings() -> Node {
    let battery = Mutable::new(get_battery_capacity());

    let battery1 = battery.clone();
    poll(
        move || {
            battery1.replace(get_battery_capacity());
        },
        Duration::from_secs(60),
    );

    Box::default()
        .class(&["bg", "pl-2", "pr-4", "rounded-r-m"])
        .spacing(4)
        .children(vec![Box::default()
            .visible_signal(battery.signal_ref(|b| b.is_some()))
            .spacing(4)
            .children(vec![
                Label::default()
                    .class_signal(battery.signal_ref(|b| {
                        b.as_ref()
                            .map(|b| vec![b.to_class().to_owned()])
                            .unwrap_or_default()
                    }))
                    .text_signal(battery.signal_ref(|b| {
                        b.as_ref()
                            .map(|b| b.to_icon().to_string())
                            .unwrap_or_default()
                    }))
                    .into(),
                Label::default()
                    .text_signal(battery.signal_ref(|b| {
                        b.as_ref()
                            .map(|b| format!("{}%", b.capacity))
                            .unwrap_or_default()
                    }))
                    .into(),
            ])
            .into()])
        .into()
}

pub fn create_shell_bar() -> Node {
    CenterBox::default()
        .class(&["text-bold", "ph-1", "pv-1"])
        .children((
            Some(
                Box::default()
                    .spacing(4)
                    .children(vec![application_button(), update_button(), worspaces()])
                    .into(),
            ),
            Some(title()),
            Some(right()),
        ))
        .into()
}
