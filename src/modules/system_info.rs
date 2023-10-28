use crate::{
    reactive_gtk::{container, label, Node, NodeBuilder, Dynamic},
    utils::poll,
};
use futures_signals::signal::Mutable;
use std::time::Duration;
use sysinfo::{ComponentExt, CpuExt, System, SystemExt};

struct SystemInfo {
    pub cpu_usage: u32,
    pub memory_usage: u32,
    pub temperature: Option<f32>,
}

fn get_system_info(system: &mut System) -> SystemInfo {
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

    SystemInfo {
        cpu_usage,
        memory_usage,
        temperature,
    }
}

pub fn system_info() -> impl Into<Node> {
    let system_info = Mutable::new(None);

    poll(
        {
            let system_info = system_info.clone();
            let mut system = System::new();
            move || {
                system_info.replace(Some(get_system_info(&mut system)));
            }
        },
        Duration::from_secs(5),
    );

    let cpu = system_info.signal_ref(|s| {
        s.as_ref()
            .map(|s| format!("󰔂  {}%", s.cpu_usage))
            .unwrap_or_default()
    });
    let ram = system_info.signal_ref(|s| {
        s.as_ref()
            .map(|s| format!("󰘚  {}%", s.memory_usage))
            .unwrap_or_default()
    });
    let temp = system_info.signal_ref(|s| {
        s.as_ref()
            .map(|s| format!("󰔏 {}°", s.temperature.unwrap_or_default()))
            .unwrap_or_default()
    });

    container()
        .class(vec!["bar-item", "system-info"])
        .spacing(4)
        .children(vec![
            label()
                .class(vec!["system-info-cpu"])
                .text(Dynamic(cpu))
                .into(),
            label()
                .class(vec!["system-info-ram"])
                .text(Dynamic(ram))
                .into(),
            label()
                .class(vec!["system-info-temp"])
                .text(Dynamic(temp))
                .into(),
        ])
}
