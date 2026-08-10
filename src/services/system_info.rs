use crate::config::{SystemInfoIndicator, SystemInfoModuleConfig};
use guido::prelude::*;
use itertools::Itertools;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use sysinfo::{Components, Disks, Networks, System};

/// Which sysinfo domains a refresh tick must touch. Derived from the bar
/// indicators in config; everything is refreshed while the menu is open
/// (the menu shows all domains).
#[derive(Clone, Copy, PartialEq)]
struct RefreshScope {
    cpu: bool,
    memory: bool,
    temperature: bool,
    disks: bool,
    network: bool,
}

impl RefreshScope {
    const ALL: Self = Self {
        cpu: true,
        memory: true,
        temperature: true,
        disks: true,
        network: true,
    };

    fn from_indicators(indicators: &[SystemInfoIndicator]) -> Self {
        let mut scope = Self {
            cpu: false,
            memory: false,
            temperature: false,
            disks: false,
            network: false,
        };
        for indicator in indicators {
            match indicator {
                SystemInfoIndicator::Cpu => scope.cpu = true,
                SystemInfoIndicator::Memory | SystemInfoIndicator::MemorySwap => {
                    scope.memory = true
                }
                SystemInfoIndicator::Temperature => scope.temperature = true,
                SystemInfoIndicator::IpAddress
                | SystemInfoIndicator::DownloadSpeed
                | SystemInfoIndicator::UploadSpeed => scope.network = true,
                SystemInfoIndicator::Disk(_) => scope.disks = true,
            }
        }
        scope
    }
}

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

#[allow(clippy::too_many_arguments)]
fn collect_system_info(
    sys: &mut System,
    components: &mut Components,
    disks_sys: &mut Disks,
    networks: &mut Networks,
    last_check: Option<Instant>,
    temperature_sensor: &str,
    scope: RefreshScope,
    previous: &SystemInfoData,
) -> SystemInfoData {
    // Refresh only the domains in scope; out-of-scope fields keep their
    // previous values so the menu never shows zeros right after opening.
    if scope.cpu {
        sys.refresh_cpu_usage();
    }
    if scope.memory {
        sys.refresh_memory();
    }
    if scope.temperature {
        // Refresh only the configured sensor — a full Components::refresh
        // reads every hwmon node and was the single biggest CPU cost of the
        // whole bar at idle. A missing sensor is handled by the caller with
        // an occasional list rescan, never per tick.
        if let Some(component) = components
            .iter_mut()
            .find(|c| c.label() == temperature_sensor)
        {
            component.refresh();
        }
    }
    if scope.disks {
        disks_sys.refresh(true);
    }
    if scope.network {
        networks.refresh(true);
    }

    let cpu_usage = if scope.cpu {
        sys.global_cpu_usage()
    } else {
        previous.cpu_usage
    };

    let (memory_usage, memory_swap_usage) = if scope.memory {
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
        (memory_usage, memory_swap_usage)
    } else {
        (previous.memory_usage, previous.memory_swap_usage)
    };

    let temperature = if scope.temperature {
        components
            .iter()
            .find(|c| c.label() == temperature_sensor)
            .and_then(|c| c.temperature())
            .map(|t| t.floor())
    } else {
        previous.temperature
    };

    if !scope.disks && !scope.network {
        return SystemInfoData {
            cpu_usage,
            memory_usage,
            memory_swap_usage,
            temperature,
            disks: previous.disks.clone(),
            network: previous.network.clone(),
        };
    }

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
        // Upstream prefix list: prefix position doubles as priority
        .filter_map(|(name, data)| {
            ["en", "eth", "wl", "br", "bond"]
                .iter()
                .position(|p| name.starts_with(p))
                .map(|prio| (prio, data))
        })
        .sorted_by_key(|(prio, _)| *prio)
        .fold(
            (None::<std::net::IpAddr>, 0u64, 0u64),
            |(ip, rx, tx), (_prio, data)| {
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
        disks: if scope.disks {
            disks
        } else {
            previous.disks.clone()
        },
        network: if scope.network {
            network
        } else {
            previous.network.clone()
        },
    }
}

pub fn start_system_info_service(
    writers: SystemInfoDataWriters,
    config: SystemInfoModuleConfig,
    menu_open: Arc<AtomicBool>,
) {
    let bar_scope = RefreshScope::from_indicators(&config.indicators);

    let _ = create_service::<(), _, _>(move |_rx, ctx| async move {
        let mut sys = System::new();
        let mut components = Components::new_with_refreshed_list();
        let mut disks = Disks::new_with_refreshed_list();
        let mut networks = Networks::new_with_refreshed_list();
        let mut last_check: Option<Instant> = None;
        let mut data = SystemInfoData::default();
        let mut tick: u32 = 0;

        let sensor = &config.temperature.sensor;
        if bar_scope.temperature && !components.iter().any(|c| c.label() == sensor) {
            log::warn!(
                "Temperature sensor {sensor:?} not found; available: {:?}",
                components.iter().map(|c| c.label()).collect::<Vec<_>>()
            );
        }

        while ctx.is_running() {
            let open = menu_open.load(Ordering::Relaxed);
            let scope = if open { RefreshScope::ALL } else { bar_scope };

            // Missing sensor (typo or hotplug): rescan the component list
            // occasionally (~once a minute), never per tick.
            if scope.temperature
                && tick.is_multiple_of(12)
                && tick > 0
                && !components.iter().any(|c| c.label() == sensor)
            {
                components.refresh(true);
            }
            tick = tick.wrapping_add(1);
            data = collect_system_info(
                &mut sys,
                &mut components,
                &mut disks,
                &mut networks,
                last_check,
                &config.temperature.sensor,
                scope,
                &data,
            );
            writers.set(data.clone());
            last_check = Some(Instant::now());

            // Sleep in slices so an opening menu gets a full refresh right
            // away instead of showing stale disk/network data. Slice count
            // follows the configured interval (default 5s).
            for _ in 0..(config.interval.max(1) * 2) {
                tokio::time::sleep(Duration::from_millis(500)).await;
                if !ctx.is_running() {
                    return;
                }
                if !open && menu_open.load(Ordering::Relaxed) {
                    break;
                }
            }
        }
    });
}
