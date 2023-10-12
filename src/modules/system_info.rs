use gtk4::Widget;
use leptos::{create_memo, create_signal, SignalGet, SignalSet};
use std::time::Duration;
use sysinfo::{ComponentExt, CpuExt, System, SystemExt};
use tokio::time::sleep;

use crate::gtk4_wrapper::{container, label, spawn, Component};

#[derive(Copy, Clone)]
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

pub fn system_info() -> Widget {
    let mut system = System::new();
    let (system_info, set_system_info) = create_signal(get_system_info(&mut system));

    spawn(async move {
        loop {
            sleep(Duration::from_secs(10)).await;
            set_system_info.set(get_system_info(&mut system));
        }
    });

    let cpu = create_memo(move |_| format!("󰔂  {}%", system_info.get().cpu_usage));
    let ram = create_memo(move |_| format!("󰘚  {}%", system_info.get().memory_usage));
    let temp =
        create_memo(move |_| format!("󰔏 {}°", system_info.get().temperature.unwrap_or_default()));

    container()
        .class(vec!["header-label"])
        .spacing(4)
        .children(vec![
            label().text(cpu).into(),
            label().text(ram).into(),
            label().text(temp).into(),
        ])
        .into()
}
