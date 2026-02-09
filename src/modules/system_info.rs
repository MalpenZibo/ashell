use guido::prelude::*;

use crate::components::{StaticIcon, icon};
use crate::services::system_info::{
    SystemInfoData, SystemInfoDataSignals, start_system_info_service,
};
use crate::theme;

fn status_color(value: f32, warn: f32, alert: f32) -> Color {
    if value > alert {
        theme::RED
    } else if value > warn {
        theme::YELLOW
    } else {
        theme::TEXT
    }
}

/// Create the system info signals and start the service.
/// Returns the signals struct that can be shared between bar and menu views.
pub fn create() -> SystemInfoDataSignals {
    let info = SystemInfoDataSignals::new(SystemInfoData::default());
    start_system_info_service(info.writers());
    info
}

/// Bar view: compact indicators.
pub fn view(info: SystemInfoDataSignals) -> impl Widget {
    let cpu = info.cpu_usage;
    let mem = info.memory_usage;
    let temp = info.temperature;

    container()
        .layout(
            Flex::row()
                .spacing(4.0)
                .cross_axis_alignment(CrossAxisAlignment::Center),
        )
        .child(indicator(
            StaticIcon::Cpu,
            move || format!("{:.0}%", cpu.get()),
            move || status_color(cpu.get(), 60.0, 80.0),
        ))
        .child(indicator(
            StaticIcon::Mem,
            move || format!("{:.0}%", mem.get()),
            move || status_color(mem.get(), 70.0, 85.0),
        ))
        .child(move || {
            let t = temp.get();
            if t.is_some() {
                Some(indicator(
                    StaticIcon::Temp,
                    move || format!("{:.0}°", temp.get().unwrap_or(0.0)),
                    move || status_color(temp.get().unwrap_or(0.0), 60.0, 80.0),
                ))
            } else {
                None
            }
        })
}

/// Menu view: detailed system info rows.
pub fn menu_view(info: SystemInfoDataSignals) -> impl Widget {
    let cpu = info.cpu_usage;
    let mem = info.memory_usage;
    let temp = info.temperature;

    container()
        .layout(Flex::column().spacing(12.0))
        .child(menu_row(
            StaticIcon::Cpu,
            "CPU",
            move || format!("{:.0}%", cpu.get()),
            move || status_color(cpu.get(), 60.0, 80.0),
        ))
        .child(menu_row(
            StaticIcon::Mem,
            "Memory",
            move || format!("{:.0}%", mem.get()),
            move || status_color(mem.get(), 70.0, 85.0),
        ))
        .child(move || {
            let t = temp.get();
            if t.is_some() {
                Some(menu_row(
                    StaticIcon::Temp,
                    "Temp",
                    move || format!("{:.0}°C", temp.get().unwrap_or(0.0)),
                    move || status_color(temp.get().unwrap_or(0.0), 60.0, 80.0),
                ))
            } else {
                None
            }
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
                .spacing(4.0)
                .cross_axis_alignment(CrossAxisAlignment::Center),
        )
        .child(icon(ic).color(color_fn).font_size(14.0))
        .child(text(value_fn).color(color_fn2).font_size(13.0))
}

fn menu_row(
    ic: StaticIcon,
    label: &'static str,
    value_fn: impl Fn() -> String + 'static,
    color_fn: impl Fn() -> Color + 'static + Clone,
) -> impl Widget {
    let color_fn2 = color_fn.clone();
    container()
        .layout(
            Flex::row()
                .spacing(8.0)
                .cross_axis_alignment(CrossAxisAlignment::Center),
        )
        .child(icon(ic).color(color_fn).font_size(16.0))
        .child(
            text(label)
                .color(theme::TEXT)
                .font_size(14.0),
        )
        .child(container().width(fill()))
        .child(text(value_fn).color(color_fn2).font_size(14.0))
}
