use crate::config::SystemInfoModuleConfig;
use guido::prelude::*;
use itertools::Itertools;
use std::time::{Duration, Instant};
use sysinfo::{Components, Disks, Networks, System};

#[derive(Clone, Debug, PartialEq)]
pub struct DiskInfo {
    pub mount_point: String,
    pub usage_pct: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct NetworkInfo {
    pub ip: String,
    pub download_speed_kbps: u32,
    pub upload_speed_kbps: u32,
}

#[derive(Clone, PartialEq, guido::SignalFields)]
pub struct SystemInfoData {
    pub cpu_usage: f32,
    pub memory_usage: f32,
    pub memory_swap_usage: f32,
    pub temperature: Option<f32>,
    pub disks: Vec<DiskInfo>,
    pub network: Option<NetworkInfo>,
}

impl Default for SystemInfoData {
    fn default() -> Self {
        Self {
            cpu_usage: 0.0,
            memory_usage: 0.0,
            memory_swap_usage: 0.0,
            temperature: None,
            disks: Vec::new(),
            network: None,
        }
    }
}

fn collect_system_info(
    sys: &mut System,
    components: &mut Components,
    disks_sys: &mut Disks,
    networks: &mut Networks,
    last_check: Option<Instant>,
    temperature_sensor: &str,
) -> SystemInfoData {
    sys.refresh_cpu_usage();
    sys.refresh_memory();
    components.refresh(true);
    disks_sys.refresh(true);
    networks.refresh(true);

    let cpu_usage = sys.global_cpu_usage();

    let total_mem = sys.total_memory() as f64;
    let used_mem = sys.used_memory() as f64;
    let memory_usage = if total_mem > 0.0 {
        (used_mem / total_mem * 100.0) as f32
    } else {
        0.0
    };

    let total_swap = sys.total_swap() as f64;
    let free_swap = sys.free_swap() as f64;
    let memory_swap_usage = if total_swap > 0.0 {
        ((total_swap - free_swap) / total_swap * 100.0) as f32
    } else {
        0.0
    };

    let temperature = components
        .iter()
        .find(|c| c.label() == temperature_sensor)
        .and_then(|c| c.temperature())
        .map(|t| t.floor());

    let disks = disks_sys
        .iter()
        .filter(|d| !d.is_removable() && d.total_space() != 0)
        .map(|d| {
            let total = d.total_space() as f64;
            let used = (total - d.available_space() as f64) / total * 100.0;
            DiskInfo {
                mount_point: d.mount_point().to_string_lossy().to_string(),
                usage_pct: used as f32,
            }
        })
        .sorted_by(|a, b| a.mount_point.cmp(&b.mount_point))
        .collect();

    let elapsed_secs = last_check.map(|lc| lc.elapsed().as_secs());

    let (first_ip, total_received, total_transmitted) = networks
        .iter()
        .filter(|(name, _)| {
            name.contains("en")
                || name.contains("eth")
                || name.contains("wl")
                || name.contains("wlan")
        })
        .sorted_by_key(|(name, _)| {
            if name.contains("en") {
                0
            } else if name.contains("eth") {
                1
            } else if name.contains("wl") {
                2
            } else {
                3
            }
        })
        .fold(
            (None::<std::net::IpAddr>, 0u64, 0u64),
            |(ip, rx, tx), (_, data)| {
                let found_ip = ip.or_else(|| {
                    data.ip_networks()
                        .iter()
                        .sorted_by(|a, b| a.addr.cmp(&b.addr))
                        .next()
                        .map(|n| n.addr)
                });
                (found_ip, rx + data.received(), tx + data.transmitted())
            },
        );

    let speed = |bytes: u64| -> u32 {
        match elapsed_secs {
            None | Some(0) => 0,
            Some(s) => (bytes / 1000) as u32 / s as u32,
        }
    };

    let network = first_ip.map(|ip| NetworkInfo {
        ip: ip.to_string(),
        download_speed_kbps: speed(total_received),
        upload_speed_kbps: speed(total_transmitted),
    });

    SystemInfoData {
        cpu_usage,
        memory_usage,
        memory_swap_usage,
        temperature,
        disks,
        network,
    }
}

pub fn start_system_info_service(writers: SystemInfoDataWriters, config: SystemInfoModuleConfig) {
    let _ = create_service::<(), _, _>(move |_rx, ctx| async move {
        let mut sys = System::new();
        let mut components = Components::new_with_refreshed_list();
        let mut disks = Disks::new_with_refreshed_list();
        let mut networks = Networks::new_with_refreshed_list();
        let mut last_check: Option<Instant> = None;

        while ctx.is_running() {
            let data = collect_system_info(
                &mut sys,
                &mut components,
                &mut disks,
                &mut networks,
                last_check,
                &config.temperature.sensor,
            );
            writers.set(data);
            last_check = Some(Instant::now());

            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    });
}
