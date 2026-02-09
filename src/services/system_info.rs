use guido::prelude::*;
use std::time::Duration;
use sysinfo::System;

#[derive(Clone, PartialEq, guido::SignalFields)]
pub struct SystemInfoData {
    pub cpu_usage: f32,
    pub memory_usage: f32,
    pub temperature: Option<f32>,
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

pub fn start_system_info_service(writers: SystemInfoDataWriters) {
    let _ = create_service::<(), _, _>(move |_rx, ctx| async move {
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

            writers.set(SystemInfoData {
                cpu_usage,
                memory_usage,
                temperature,
            });

            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    });
}
