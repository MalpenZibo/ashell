use guido::prelude::*;

use crate::components::{StaticIcon, icon};
use crate::config::Config;
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

/// Create the system info signals and start the service.
/// Returns the signals struct that can be shared between bar and menu views.
pub fn create() -> SystemInfoDataSignals {
    let info = SystemInfoDataSignals::new(SystemInfoData::default());
    start_system_info_service(info.writers());
    info
}

/// Bar view: compact indicators.
pub fn view(info: SystemInfoDataSignals) -> impl Widget {
    let theme = expect_context::<ThemeColors>();
    let config = expect_context::<Config>().system_info;
    let cpu = info.cpu_usage;
    let mem = info.memory_usage;
    let temp = info.temperature;
    let cpu_warn = config.cpu.warn_threshold as f32;
    let cpu_alert = config.cpu.alert_threshold as f32;
    let mem_warn = config.memory.warn_threshold as f32;
    let mem_alert = config.memory.alert_threshold as f32;
    let temp_warn = config.temperature.warn_threshold as f32;
    let temp_alert = config.temperature.alert_threshold as f32;

    container()
        .layout(
            Flex::row()
                .spacing(4)
                .cross_alignment(CrossAlignment::Center),
        )
        .child(indicator(
            StaticIcon::Cpu,
            move || format!("{:.0}%", cpu.get()),
            move || status_color(theme, cpu.get(), cpu_warn, cpu_alert),
        ))
        .child(indicator(
            StaticIcon::Mem,
            move || format!("{:.0}%", mem.get()),
            move || status_color(theme, mem.get(), mem_warn, mem_alert),
        ))
        .child(move || {
            let t = temp.get();
            if t.is_some() {
                Some(indicator(
                    StaticIcon::Temp,
                    move || format!("{:.0}°", temp.get().unwrap_or(0.0)),
                    move || status_color(theme, temp.get().unwrap_or(0.0), temp_warn, temp_alert),
                ))
            } else {
                None
            }
        })
}

/// Menu view: detailed system info rows.
pub fn menu_view(info: SystemInfoDataSignals) -> impl Widget {
    let theme = expect_context::<ThemeColors>();
    let config = expect_context::<Config>().system_info;
    let cpu = info.cpu_usage;
    let mem = info.memory_usage;
    let temp = info.temperature;
    let cpu_warn = config.cpu.warn_threshold as f32;
    let cpu_alert = config.cpu.alert_threshold as f32;
    let mem_warn = config.memory.warn_threshold as f32;
    let mem_alert = config.memory.alert_threshold as f32;
    let temp_warn = config.temperature.warn_threshold as f32;
    let temp_alert = config.temperature.alert_threshold as f32;

    container()
        .width(fill())
        .layout(Flex::column().spacing(12))
        .child(menu_row(
            theme,
            StaticIcon::Cpu,
            "CPU",
            move || format!("{:.0}%", cpu.get()),
            move || status_color(theme, cpu.get(), cpu_warn, cpu_alert),
        ))
        .child(menu_row(
            theme,
            StaticIcon::Mem,
            "Memory",
            move || format!("{:.0}%", mem.get()),
            move || status_color(theme, mem.get(), mem_warn, mem_alert),
        ))
        .child(move || {
            let t = temp.get();
            if t.is_some() {
                Some(menu_row(
                    theme,
                    StaticIcon::Temp,
                    "Temp",
                    move || format!("{:.0}°C", temp.get().unwrap_or(0.0)),
                    move || status_color(theme, temp.get().unwrap_or(0.0), temp_warn, temp_alert),
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
                .spacing(4)
                .cross_alignment(CrossAlignment::Center),
        )
        .child(icon().ic(ic).color(color_fn).font_size(14))
        .child(text(value_fn).color(color_fn2).font_size(13))
}

fn menu_row(
    theme: ThemeColors,
    ic: StaticIcon,
    label: &'static str,
    value_fn: impl Fn() -> String + 'static,
    color_fn: impl Fn() -> Color + 'static + Clone,
) -> impl Widget {
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
                .child(icon().ic(ic).color(color_fn).font_size(16))
                .child(text(label).color(theme.text).font_size(14)),
        )
        .child(text(value_fn).color(color_fn2).font_size(14))
}
