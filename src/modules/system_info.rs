use guido::prelude::*;

use crate::services::system_info::{
    SystemInfoData, SystemInfoDataSignals, start_system_info_service,
};
use crate::theme;

// NerdFont icons
const CPU_ICON: &str = "\u{f0502}";
const MEM_ICON: &str = "\u{efc5}";
const TEMP_ICON: &str = "\u{f050f}";

fn status_color(value: f32, warn: f32, alert: f32) -> Color {
    if value > alert {
        theme::RED
    } else if value > warn {
        theme::YELLOW
    } else {
        theme::TEXT
    }
}

pub fn view() -> impl Widget {
    let info = SystemInfoDataSignals::new(SystemInfoData::default());
    start_system_info_service(info.writers());

    let cpu = info.cpu_usage;
    let mem = info.memory_usage;
    let temp = info.temperature;

    container()
        .layout(
            Flex::row()
                .spacing(12.0)
                .cross_axis_alignment(CrossAxisAlignment::Center),
        )
        .child(indicator(
            CPU_ICON,
            move || format!("{:.0}%", cpu.get()),
            move || status_color(cpu.get(), 60.0, 80.0),
        ))
        .child(indicator(
            MEM_ICON,
            move || format!("{:.0}%", mem.get()),
            move || status_color(mem.get(), 70.0, 85.0),
        ))
        .child(move || {
            let t = temp.get();
            if t.is_some() {
                Some(indicator(
                    TEMP_ICON,
                    move || format!("{:.0}°", temp.get().unwrap_or(0.0)),
                    move || status_color(temp.get().unwrap_or(0.0), 60.0, 80.0),
                ))
            } else {
                None
            }
        })
}

fn indicator(
    icon: &'static str,
    value_fn: impl Fn() -> String + 'static,
    color_fn: impl Fn() -> Color + 'static + Clone,
) -> impl Widget {
    let color_fn2 = color_fn.clone();
    container()
        .layout(
            Flex::row()
                .spacing(12.0)
                .cross_axis_alignment(CrossAxisAlignment::Center),
        )
        .child(text(icon).color(color_fn).font_size(14.0))
        .child(text(value_fn).color(color_fn2).font_size(13.0))
}
