use std::time::Duration;

use futures_signals::signal::Mutable;
use sysinfo::{ComponentExt, CpuExt, System, SystemExt};

use crate::{
    reactive_gtk::{Box, Component, Label, Node},
    utils::poll,
};

struct SystemInfo {
    pub cpu_usage: u32,
    pub memory_usage: u32,
    pub temperature: Option<f32>,
}

pub fn system_info() -> Node {
    let system_info = Mutable::new(None);

    let system_info1 = system_info.clone();

    let mut system = System::new();
    poll(
        move || {
            system.refresh_memory();
            system.refresh_cpu_specifics(sysinfo::CpuRefreshKind::everything());
            system.refresh_components_list();
            system.refresh_components();

            let cpu_usage = system.global_cpu_info().cpu_usage().floor() as u32;
            let memory_usage = ((system.total_memory() - system.available_memory()) as f32
                / system.total_memory() as f32
                * 100.) as u32;

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
        },
        Duration::from_secs(5),
    );

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
