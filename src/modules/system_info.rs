use guido::prelude::*;

use crate::components::{StaticIcon, icon};
use crate::config::{Config, SystemInfoIndicator};
use crate::services::system_info::{
    SystemInfoData, SystemInfoDataSignals, start_system_info_service,
};
use crate::theme::ThemeColors;

fn status_color(theme: ThemeColors, value: f32, warn: f32, alert: f32) -> Color {
    if value > alert {
        theme.danger
    } else if value > warn {
        theme.warning
    } else {
        theme.text
    }
}

fn format_speed(kbps: u32) -> String {
    if kbps > 1000 {
        format!("{} MB/s", kbps / 1000)
    } else {
        format!("{kbps} KB/s")
    }
}

/// Create the system info signals and start the service.
/// Returns the signals struct that can be shared between bar and menu views.
pub fn create() -> SystemInfoDataSignals {
    let config = expect_context::<Config>().system_info;
    let info = SystemInfoDataSignals::new(SystemInfoData::default());
    start_system_info_service(info.writers(), config);
    info
}

/// Bar view: compact indicators driven by config.indicators.
pub fn view(info: SystemInfoDataSignals) -> impl Widget {
    let theme = expect_context::<ThemeColors>();
    let config = expect_context::<Config>().system_info;

    let cpu = info.cpu_usage;
    let mem = info.memory_usage;
    let swap = info.memory_swap_usage;
    let temp = info.temperature;
    let disks = info.disks;
    let network = info.network;

    let cpu_warn = config.cpu.warn_threshold as f32;
    let cpu_alert = config.cpu.alert_threshold as f32;
    let mem_warn = config.memory.warn_threshold as f32;
    let mem_alert = config.memory.alert_threshold as f32;
    let temp_warn = config.temperature.warn_threshold as f32;
    let temp_alert = config.temperature.alert_threshold as f32;
    let disk_warn = config.disk.warn_threshold as f32;
    let disk_alert = config.disk.alert_threshold as f32;

    let mut row = container().layout(
        Flex::row()
            .spacing(4)
            .cross_alignment(CrossAlignment::Center),
    );

    for ind in &config.indicators {
        match ind {
            SystemInfoIndicator::Cpu => {
                row = row.child(indicator(
                    StaticIcon::Cpu,
                    move || format!("{:.0}%", cpu.get()),
                    move || status_color(theme, cpu.get(), cpu_warn, cpu_alert),
                ));
            }
            SystemInfoIndicator::Memory => {
                row = row.child(indicator(
                    StaticIcon::Mem,
                    move || format!("{:.0}%", mem.get()),
                    move || status_color(theme, mem.get(), mem_warn, mem_alert),
                ));
            }
            SystemInfoIndicator::MemorySwap => {
                row = row.child(indicator(
                    StaticIcon::Mem,
                    move || format!("swap {:.0}%", swap.get()),
                    move || status_color(theme, swap.get(), mem_warn, mem_alert),
                ));
            }
            SystemInfoIndicator::Temperature => {
                row = row.child(move || {
                    temp.get().map(|_| {
                        indicator(
                            StaticIcon::Temp,
                            move || format!("{:.0}°", temp.get().unwrap_or(0.0)),
                            move || {
                                status_color(
                                    theme,
                                    temp.get().unwrap_or(0.0),
                                    temp_warn,
                                    temp_alert,
                                )
                            },
                        )
                    })
                });
            }
            SystemInfoIndicator::Disk(disk_cfg) => {
                let path = disk_cfg.path.clone();
                let name = disk_cfg.name.clone().unwrap_or_else(|| path.clone());
                row = row.child(move || {
                    let path = path.clone();
                    let name = name.clone();
                    disks
                        .with(|ds| {
                            ds.iter()
                                .find(|d| d.mount_point == path)
                                .map(|d| d.usage_pct)
                        })
                        .map(move |usage| {
                            indicator(
                                StaticIcon::Drive,
                                move || format!("{name} {usage:.0}%"),
                                move || status_color(theme, usage, disk_warn, disk_alert),
                            )
                        })
                });
            }
            SystemInfoIndicator::IpAddress => {
                row = row.child(move || {
                    network
                        .with(|n| n.as_ref().map(|n| n.ip.clone()))
                        .map(|ip| {
                            indicator(
                                StaticIcon::IpAddress,
                                move || ip.clone(),
                                move || theme.text,
                            )
                        })
                });
            }
            SystemInfoIndicator::DownloadSpeed => {
                row = row.child(move || {
                    network
                        .with(|n| n.as_ref().map(|n| n.download_speed_kbps))
                        .map(|speed| {
                            indicator(
                                StaticIcon::DownloadSpeed,
                                move || format_speed(speed),
                                move || theme.text,
                            )
                        })
                });
            }
            SystemInfoIndicator::UploadSpeed => {
                row = row.child(move || {
                    network
                        .with(|n| n.as_ref().map(|n| n.upload_speed_kbps))
                        .map(|speed| {
                            indicator(
                                StaticIcon::UploadSpeed,
                                move || format_speed(speed),
                                move || theme.text,
                            )
                        })
                });
            }
        }
    }

    row
}

/// Menu view: detailed system info rows.
pub fn menu_view(info: SystemInfoDataSignals) -> impl Widget {
    let theme = expect_context::<ThemeColors>();
    let config = expect_context::<Config>().system_info;
    let cpu = info.cpu_usage;
    let mem = info.memory_usage;
    let swap = info.memory_swap_usage;
    let temp = info.temperature;
    let disks = info.disks;
    let network = info.network;
    let cpu_warn = config.cpu.warn_threshold as f32;
    let cpu_alert = config.cpu.alert_threshold as f32;
    let mem_warn = config.memory.warn_threshold as f32;
    let mem_alert = config.memory.alert_threshold as f32;
    let temp_warn = config.temperature.warn_threshold as f32;
    let temp_alert = config.temperature.alert_threshold as f32;

    container()
        .width(fill())
        .layout(Flex::column().spacing(12))
        // CPU
        .child(menu_row(
            theme,
            StaticIcon::Cpu,
            "CPU",
            move || format!("{:.0}%", cpu.get()),
            move || status_color(theme, cpu.get(), cpu_warn, cpu_alert),
        ))
        // Memory
        .child(menu_row(
            theme,
            StaticIcon::Mem,
            "Memory",
            move || format!("{:.0}%", mem.get()),
            move || status_color(theme, mem.get(), mem_warn, mem_alert),
        ))
        // Swap
        .child(menu_row(
            theme,
            StaticIcon::Mem,
            "Swap Memory",
            move || format!("{:.0}%", swap.get()),
            move || status_color(theme, swap.get(), mem_warn, mem_alert),
        ))
        // Temperature
        .child(move || {
            temp.get().map(|_| {
                menu_row(
                    theme,
                    StaticIcon::Temp,
                    "Temperature",
                    move || format!("{:.0}°C", temp.get().unwrap_or(0.0)),
                    move || status_color(theme, temp.get().unwrap_or(0.0), temp_warn, temp_alert),
                )
            })
        })
        // Disks (dynamic, keyed)
        .children(move || {
            disks.with(|ds| {
                ds.iter()
                    .enumerate()
                    .map(|(i, d)| {
                        let mount = d.mount_point.clone();
                        let usage = d.usage_pct;
                        (i as u64, move || {
                            menu_row(
                                theme,
                                StaticIcon::Drive,
                                format!("Disk {mount}"),
                                move || format!("{usage:.0}%"),
                                move || theme.text,
                            )
                        })
                    })
                    .collect::<Vec<_>>()
            })
        })
        // Network
        .child(move || {
            network.with(|n| n.is_some()).then(|| {
                let ip = network.with(|n| n.as_ref().map(|n| n.ip.clone()).unwrap_or_default());
                let dl = network.with(|n| n.as_ref().map(|n| n.download_speed_kbps).unwrap_or(0));
                let ul = network.with(|n| n.as_ref().map(|n| n.upload_speed_kbps).unwrap_or(0));
                container()
                    .width(fill())
                    .layout(Flex::column().spacing(12))
                    .child(menu_row(
                        theme,
                        StaticIcon::IpAddress,
                        "IP Address",
                        move || ip.clone(),
                        move || theme.text,
                    ))
                    .child(menu_row(
                        theme,
                        StaticIcon::DownloadSpeed,
                        "Download",
                        move || format_speed(dl),
                        move || theme.text,
                    ))
                    .child(menu_row(
                        theme,
                        StaticIcon::UploadSpeed,
                        "Upload",
                        move || format_speed(ul),
                        move || theme.text,
                    ))
            })
        })
}

fn indicator(
    ic: StaticIcon,
    value_fn: impl Fn() -> String + 'static,
    color_fn: impl Fn() -> Color + 'static + Clone,
) -> impl Widget {
    let color_fn2 = color_fn.clone();
    container()
        .layout(
            Flex::row()
                .spacing(4)
                .cross_alignment(CrossAlignment::Center),
        )
        .child(icon().kind(ic).color(color_fn).font_size(14))
        .child(text(value_fn).color(color_fn2).font_size(13))
}

fn menu_row(
    theme: ThemeColors,
    ic: StaticIcon,
    label: impl Into<String>,
    value_fn: impl Fn() -> String + 'static,
    color_fn: impl Fn() -> Color + 'static + Clone,
) -> impl Widget {
    let label = label.into();
    let color_fn2 = color_fn.clone();
    container()
        .width(fill())
        .layout(
            Flex::row()
                .main_alignment(MainAlignment::SpaceBetween)
                .cross_alignment(CrossAlignment::Center),
        )
        .child(
            container()
                .layout(
                    Flex::row()
                        .spacing(8)
                        .cross_alignment(CrossAlignment::Center),
                )
                .child(icon().kind(ic).color(color_fn).font_size(16))
                .child(text(label).color(theme.text).font_size(14)),
        )
        .child(text(value_fn).color(color_fn2).font_size(14))
}
