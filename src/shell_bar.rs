use std::time::Duration;

use crate::{
    audio::{audio_subscribe, Sink},
    battery::get_battery_capacity,
    clock::clock,
    net::{net_monitor, Vpn},
    reactive_gtk::CenterBox,
    system_info::system_info,
    title::title,
    updates::update_button,
    utils::poll,
    workspaces::worspaces,
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

    let active_connection = Mutable::new(None);
    let vpn_list: Mutable<Vec<Vpn>> = Mutable::new(Vec::with_capacity(0));
    net_monitor(active_connection.clone(), vpn_list.clone());

    let sinks: Mutable<Vec<Sink>> = Mutable::new(Vec::with_capacity(0));
    let sources: Mutable<u32> = Mutable::new(0);
    audio_subscribe(sinks.clone(), sources);

    Box::default()
        .class(&["bg", "pl-2", "pr-4", "rounded-r-m"])
        .spacing(4)
        .children(vec![
            Label::default()
                .text_signal(active_connection.signal_ref(|c| {
                    c.as_ref()
                        .map(|c| c.to_icon().to_string())
                        .unwrap_or_default()
                }))
                .into(),
            Label::default()
                .text("󰖂")
                .visible_signal(
                    vpn_list.signal_ref(|vpn_list| vpn_list.iter().any(|vpn| vpn.active)),
                )
                .into(),
            Label::default()
                .text_signal(sinks.signal_ref(|s| {
                    s.iter()
                        .find_map(|s| {
                            if s.active {
                                Some(s.to_icon().to_string())
                            } else {
                                None
                            }
                        })
                        .unwrap_or_default()
                }))
                .into(),
            Box::default()
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
                .into(),
        ])
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
