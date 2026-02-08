use guido::prelude::*;
use std::time::Duration;
use sysinfo::System;

use crate::theme;

// NerdFont icons
const CPU_ICON: &str = "\u{f0502}";
const MEM_ICON: &str = "\u{efc5}";
const TEMP_ICON: &str = "\u{f050f}";

#[derive(Clone, PartialEq)]
struct SystemInfoData {
    cpu_usage: f32,
    memory_usage: f32,
    temperature: Option<f32>,
}

impl Default for SystemInfoData {
    fn default() -> Self {
        Self {
            cpu_usage: 0.0,
            memory_usage: 0.0,
            temperature: None,
        }
    }
}

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
    let info = create_signal(SystemInfoData::default());
    let info_writer = info.writer();

    let _ = create_service::<(), _>(move |_rx, ctx| {
        let mut sys = System::new();

        while ctx.is_running() {
            sys.refresh_cpu_usage();
            sys.refresh_memory();

            let cpu_usage = sys.global_cpu_usage();
            let total_mem = sys.total_memory() as f64;
            let used_mem = sys.used_memory() as f64;
            let memory_usage = if total_mem > 0.0 {
                (used_mem / total_mem * 100.0) as f32
            } else {
                0.0
            };

            // Read temperature from thermal zone
            let temperature = std::fs::read_to_string("/sys/class/thermal/thermal_zone0/temp")
                .ok()
                .and_then(|s| s.trim().parse::<f32>().ok())
                .map(|t| t / 1000.0);

            info_writer.set(SystemInfoData {
                cpu_usage,
                memory_usage,
                temperature,
            });

            std::thread::sleep(Duration::from_secs(5));
        }
    });

    container()
        .layout(
            Flex::row()
                .spacing(12.0)
                .cross_axis_alignment(CrossAxisAlignment::Center),
        )
        .child(indicator(
            CPU_ICON,
            move || format!("{:.0}%", info.with(|i| i.cpu_usage)),
            move || status_color(info.with(|i| i.cpu_usage), 60.0, 80.0),
        ))
        .child(indicator(
            MEM_ICON,
            move || format!("{:.0}%", info.with(|i| i.memory_usage)),
            move || status_color(info.with(|i| i.memory_usage), 70.0, 85.0),
        ))
        .child(move || {
            let has_temp = info.with(|i| i.temperature.is_some());
            if has_temp {
                Some(indicator(
                    TEMP_ICON,
                    move || {
                        format!(
                            "{:.0}°",
                            info.with(|i| i.temperature.unwrap_or(0.0))
                        )
                    },
                    move || {
                        status_color(
                            info.with(|i| i.temperature.unwrap_or(0.0)),
                            60.0,
                            80.0,
                        )
                    },
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
                .spacing(4.0)
                .cross_axis_alignment(CrossAxisAlignment::Center),
        )
        .child(text(icon).color(move || color_fn()).font_size(14.0))
        .child(text(value_fn).color(move || color_fn2()).font_size(13.0))
}
