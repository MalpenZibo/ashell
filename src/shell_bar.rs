use std::{
    fs::read_to_string,
    thread::{self, sleep},
    time::Duration,
};

use crate::{reactive_gtk::CenterBox, updates::update_button, workspaces::worspaces};
use chrono::Local;
use futures_signals::{
    signal::Mutable,
    signal_vec::{MutableVec, SignalVecExt},
};
use hyprland::{
    data::{Client, Workspace},
    event_listener::EventListener,
    shared::{HyprData, HyprDataActive, HyprDataActiveOptional},
};
use sysinfo::{ComponentExt, CpuExt, System, SystemExt};

use crate::{
    launcher::{check_updates, launch_rofi, Update},
    reactive_gtk::{Align, Box, Component, Label, Node, Overlay},
};

fn application_button() -> Node {
    Box::default()
        .class(&["rounded-m", "bg", "interactive"])
        .on_click(launch_rofi)
        .children(vec![Label::default().class(&["ph-2"]).text("󱗼").into()])
        .into()
}

fn title() -> Node {
    let get_title = || Client::get_active().ok().flatten().map(|w| w.title);
    let title = Mutable::new(get_title());

    let title1 = title.clone();
    tokio::spawn(async move {
        let mut event_listener = EventListener::new();

        event_listener.add_active_window_change_handler(move |e| {
            println!("active window changed {:?}", e);
            title1.replace(e.map(|w| w.window_title));
        });

        event_listener
            .start_listener_async()
            .await
            .expect("failed to start active window listener");
    });

    Label::default()
        .class(&["bg", "ph-4", "rounded-m"])
        .text_signal(
            title.signal_ref(|t| t.as_ref().map_or_else(|| "".to_owned(), |t| t.to_owned())),
        )
        .visible_signal(title.signal_ref(|t| t.is_some()))
        .into()
}

struct SystemInfo {
    pub cpu_usage: u32,
    pub memory_usage: u32,
    pub temperature: Option<f32>,
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

fn system_info() -> Node {
    let system_info = Mutable::new(None);

    let system_info1 = system_info.clone();
    thread::spawn(move || {
        let mut system = System::new();
        loop {
            {
                system.refresh_memory();
                system.refresh_cpu_specifics(sysinfo::CpuRefreshKind::everything());
                system.refresh_components_list();
                system.refresh_components();

                let cpu_usage = system.global_cpu_info().cpu_usage().floor() as u32;
                let memory_usage = ((system.total_memory() - system.available_memory()) as f32
                    / system.total_memory() as f32
                    * 100.) as u32;

                println!("{:?}", system.components());

                let temperature = system
                    .components()
                    .iter()
                    .find(|c| c.label() == "acpitz temp1")
                    .map(|c| c.temperature());

                system_info1.replace(Some(SystemInfo {
                    cpu_usage,
                    memory_usage,
                    temperature,
                }));
            }

            sleep(Duration::from_secs(5));
        }
    });

    Box::default()
        .class(&["bg", "ph-4", "rounded-m"])
        .spacing(4)
        .children(vec![
            Label::default()
                .text_signal(system_info.signal_ref(|s| {
                    s.as_ref()
                        .map(|s| format!(" {}%", s.cpu_usage))
                        .unwrap_or_default()
                }))
                .into(),
            Label::default()
                .text_signal(system_info.signal_ref(|s| {
                    s.as_ref()
                        .map(|s| format!("󰘚 {}%", s.memory_usage))
                        .unwrap_or_default()
                }))
                .into(),
            Label::default()
                .text_signal(system_info.signal_ref(|s| {
                    s.as_ref()
                        .map(|s| format!(" {}%", s.temperature.unwrap_or_default()))
                        .unwrap_or_default()
                }))
                .visible_signal(system_info.signal_ref(|s| {
                    s.as_ref()
                        .map(|s| s.temperature.is_some())
                        .unwrap_or_default()
                }))
                .into(),
        ])
        .into()
}

fn clock() -> Node {
    let get_date = || {
        let local = Local::now();
        let formatted_date = local.format("%a %d %b %R").to_string();

        formatted_date
    };
    let clock = Mutable::new(get_date());

    let clock1 = clock.clone();
    thread::spawn(move || loop {
        {
            clock1.replace(get_date());
        }

        sleep(Duration::from_secs(20));
    });

    Label::default()
        .class(&["bg", "pl-4", "pr-2", "rounded-l-m"])
        .text_signal(clock.signal_cloned())
        .into()
}

struct BatteryData {
    capacity: i64,
    status: BatteryStatus,
}

enum BatteryStatus {
    Charging,
    Discharging,
}

impl BatteryData {
    pub fn to_class(&self) -> &str {
        match self {
            BatteryData {
                status: BatteryStatus::Charging,
                ..
            } => "fg-green",
            BatteryData {
                status: BatteryStatus::Discharging,
                capacity,
            } if *capacity < 20 => "fg-red",
            _ => "",
        }
    }

    pub fn to_icon(&self) -> &str {
        match self {
            BatteryData {
                status: BatteryStatus::Charging,
                ..
            } => "󰂄",
            BatteryData {
                status: BatteryStatus::Discharging,
                capacity,
            } if *capacity < 20 => "󰂃",
            BatteryData {
                status: BatteryStatus::Discharging,
                capacity,
            } if *capacity < 40 => "󰁼",
            BatteryData {
                status: BatteryStatus::Discharging,
                capacity,
            } if *capacity < 60 => "󰁾",
            BatteryData {
                status: BatteryStatus::Discharging,
                capacity,
            } if *capacity < 80 => "󰂀",
            _ => "󰁹",
        }
    }
}

fn get_battery_capacity() -> Option<BatteryData> {
    let power_supply_dir = std::path::Path::new("/sys/class/power_supply/BAT0");

    if let (Ok(capacity), Ok(status)) = (
        read_to_string(power_supply_dir.join("capacity")),
        read_to_string(power_supply_dir.join("status")),
    ) {
        capacity
            .trim_end_matches('\n')
            .parse::<f64>()
            .map(|c| BatteryData {
                status: match status.trim_end_matches('\n') {
                    "Charging" => BatteryStatus::Charging,
                    _ => BatteryStatus::Discharging,
                },
                capacity: c.round() as i64,
            })
            .ok()
    } else {
        None
    }
}

fn settings() -> Node {
    let battery = Mutable::new(get_battery_capacity());

    let battery1 = battery.clone();
    thread::spawn(move || loop {
        battery1.replace(get_battery_capacity());

        sleep(Duration::from_secs(60));
    });

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
