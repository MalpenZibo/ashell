use crate::{
    components::icons::{StaticIcon, icon},
    config::{
        CpuFormat, DiskFormat, MemoryFormat, SystemInfoIndicator, SystemInfoModuleConfig,
        TemperatureFormat,
    },
    menu::MenuSize,
    theme::AshellTheme,
    theme::AshellTheme,
    utils,
};
use iced_layershell::{
    Alignment, Element, Length, Subscription, Theme,
    time::every,
    widget::{Column, Row, column, container, row, rule, text},
};
use itertools::Itertools;
use std::time::{Duration, Instant};
use sysinfo::{Components, Disks, Networks, System};

const MAX_IP_LEN: usize = 45;

#[derive(Clone, Copy)]
struct FixedIp([u8; MAX_IP_LEN], usize);

impl FixedIp {
    fn from_str(s: &str) -> Option<Self> {
        if s.len() < MAX_IP_LEN {
            let mut arr = [0u8; MAX_IP_LEN];
            arr[..s.len()].copy_from_slice(s.as_bytes());
            Some(Self(arr, s.len()))
        } else {
            None
        }
    }
}

impl std::fmt::Display for FixedIp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            std::str::from_utf8(&self.0[..self.1]).unwrap_or("")
        )
    }
}

struct NetworkData {
    ip: FixedIp,
    download_speed: u32,
    upload_speed: u32,
    last_check: Instant,
}

struct MemoryUsage {
    percentage: u32,
    fraction: String,
}
struct CpuUsage {
    percentage: u32,
    frequency: f32,
}

struct Temperature {
    celsius: Option<i32>,
    fahrenheit: i32,
}

struct DiskView {
    percentage: u32,
    fraction: String,
}

struct SystemInfoData {
    pub cpu_usage: CpuUsage,
    pub memory_usage: MemoryUsage,
    pub memory_swap_usage: MemoryUsage,
    pub temperature: Temperature,
    pub disks: Vec<(String, DiskView)>,
    pub network: Option<NetworkData>,
}

fn get_system_info(
    system: &mut System,
    components: &mut Components,
    disks: &mut Disks,
    (networks, last_check): (&mut Networks, Option<Instant>),
    temperature_sensor: &str,
    sensor_index: Option<usize>,
) -> SystemInfoData {
    system.refresh_memory();
    system.refresh_cpu_all();

    components.refresh(true);
    disks.refresh(true);
    networks.refresh(true);

    let cpus = system.cpus();
    let avg_freq = cpus.iter().map(|cpu| cpu.frequency() as f32).sum::<f32>() / cpus.len() as f32;

    let cpu_usage = CpuUsage {
        percentage: system.global_cpu_usage() as u32,
        frequency: utils::floor_dp(avg_freq / 1000.0, 2),
    };

    let total_mem = system.total_memory();
    let avail_mem = system.available_memory();
    let used_mem = system.used_memory();

    let memory_usage = MemoryUsage {
        percentage: ((total_mem - avail_mem) as f32 / total_mem as f32 * 100.) as u32,

        fraction: format!(
            "{:.2}/{:.2}",
            utils::bytes_to_gib(used_mem),
            utils::bytes_to_gib(total_mem)
        ),
    };

    let total_swap = system.total_swap();
    let free_swap = system.free_swap();

    let memory_swap_usage = MemoryUsage {
        percentage: ((total_swap - free_swap) as f32 / total_swap as f32 * 100.) as u32,
        fraction: format!(
            "{:.2}/{:.2}",
            utils::bytes_to_gib(total_swap - free_swap),
            utils::bytes_to_gib(total_swap)
        ),
    };

    let temperature_cel = sensor_index
        .and_then(|i| components.get(i))
        .and_then(|c| c.temperature().map(|t| t as i32))
        .or_else(|| {
            components
                .iter()
                .find(|c| c.label() == temperature_sensor)
                .and_then(|c| c.temperature().map(|t| t as i32))
        });

    let temperature = Temperature {
        celsius: temperature_cel,
        fahrenheit: temperature_cel
            .map(utils::celsius_to_fahrenheit)
            .unwrap_or(0),
    };

    let disks: Vec<(String, DiskView)> = disks
        .iter()
        .filter(|d| !d.is_removable() && d.total_space() != 0)
        .map(|d| {
            let total_space = d.total_space();
            let avail_space = d.available_space();

            let space_per = (total_space - avail_space) as f32 / total_space as f32 * 100.;

            (
                d.mount_point().display().to_string(),
                DiskView {
                    percentage: space_per as u32,
                    fraction: format!(
                        "{:.2}/{:.2}",
                        utils::bytes_to_gb(total_space - avail_space),
                        utils::bytes_to_gb(total_space)
                    ),
                },
            )
        })
        .sorted_by(|a, b| a.0.cmp(&b.0))
        .collect();

    let elapsed = last_check.map(|v| v.elapsed().as_secs());

    let network = networks
        .iter()
        .filter(|(name, _)| {
            name.contains("en")
                || name.contains("eth")
                || name.contains("wl")
                || name.contains("wlan")
                || name.contains("br")
        })
        .sorted_by_key(|(name, _)| {
            if name.contains("en") {
                return 0;
            }

            if name.contains("eth") {
                return 1;
            }

            if name.contains("wl") {
                return 2;
            }

            if name.contains("wlan") {
                return 3;
            }
            if name.contains("br") {
                return 4;
            }

            99
        })
        .fold(
            (None, 0, 0),
            |(first_ip, total_received, total_transmitted), (_, data)| {
                let ip = first_ip.or_else(|| {
                    data.ip_networks()
                        .iter()
                        .sorted_by(|a, b| a.addr.cmp(&b.addr))
                        .next()
                        .map(|ip| ip.addr)
                });

                let received = data.received();
                let transmitted = data.transmitted();

                (
                    first_ip.or(ip),
                    total_received + received,
                    total_transmitted + transmitted,
                )
            },
        );

    let network_speed = |value: u64| {
        match elapsed {
            None | Some(0) => 0, // avoid division by zero
            Some(elapsed) => (value / 1000) as u32 / elapsed as u32,
        }
    };

    SystemInfoData {
        cpu_usage,
        memory_usage,
        memory_swap_usage,
        temperature,
        disks,
        network: network.0.and_then(|ip| {
            let ip_str = ip.to_string();
            FixedIp::from_str(&ip_str).map(|ip| NetworkData {
                ip,
                download_speed: network_speed(network.1),
                upload_speed: network_speed(network.2),
                last_check: Instant::now(),
            })
        }),
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    Update,
}

pub struct SystemInfo {
    config: SystemInfoModuleConfig,
    system: System,
    components: Components,
    disks: Disks,
    networks: Networks,
    data: SystemInfoData,
    cached_sensor_index: Option<usize>,
}

impl SystemInfo {
    pub fn new(config: SystemInfoModuleConfig) -> Self {
        let mut system = System::new();
        let mut components = Components::new_with_refreshed_list();
        let mut disks = Disks::new_with_refreshed_list();
        let mut networks = Networks::new_with_refreshed_list();

        let cached_sensor_index = components
            .iter()
            .position(|c| c.label() == config.temperature.sensor);

        let data = get_system_info(
            &mut system,
            &mut components,
            &mut disks,
            (&mut networks, None),
            config.temperature.sensor.as_str(),
            cached_sensor_index,
        );

        Self {
            config,
            system,
            components,
            disks,
            data,
            networks,
            cached_sensor_index,
        }
    }

    pub fn update(&mut self, message: Message) {
        match message {
            Message::Update => {
                self.data = get_system_info(
                    &mut self.system,
                    &mut self.components,
                    &mut self.disks,
                    (
                        &mut self.networks,
                        self.data.network.as_ref().map(|n| n.last_check),
                    ),
                    &self.config.temperature.sensor,
                    self.cached_sensor_index,
                );
            }
        }
    }

    fn info_element<'a>(
        theme: &AshellTheme,
        info_icon: StaticIcon,
        label: String,
        value: String,
    ) -> Element<'a, Message> {
        row!(
            container(icon(info_icon).size(theme.font_size.xl))
                .center_x(Length::Fixed(theme.space.xl as f32)),
            text(label).width(Length::Fill),
            text(value)
        )
        .align_y(Alignment::Center)
        .spacing(theme.space.xs)
        .into()
    }

    fn indicator_info_element<'a, V: PartialOrd + 'a>(
        theme: &AshellTheme,
        info_icon: StaticIcon,
        (display, unit): (impl std::fmt::Display + 'a, &str),
        // value: V,
        // unit: &str,
        threshold: Option<(V, V, V)>,
        prefix: Option<&str>,
    ) -> Element<'a, Message> {
        let element = container(
            row!(
                icon(info_icon),
                if let Some(prefix) = prefix {
                    text(format!("{prefix} {display}{unit}"))
                } else {
                    text(format!("{display}{unit}"))
                }
            )
            .spacing(theme.space.xxs),
        );

        if let Some((value, warn_threshold, alert_threshold)) = threshold {
            element
                .style(move |theme: &Theme| container::Style {
                    text_color: if value > warn_threshold && value < alert_threshold {
                        Some(theme.extended_palette().danger.weak.color)
                    } else if value >= alert_threshold {
                        Some(theme.palette().danger)
                    } else {
                        None
                    },
                    ..Default::default()
                })
                .into()
        } else {
            element.into()
        }
    }

    pub fn menu_view(&'_ self, theme: &AshellTheme) -> Element<'_, Message> {
        container(
            column!(
                text("System Info").size(theme.font_size.lg),
                rule::horizontal(1),
                Column::with_capacity(6)
                    .push(Self::info_element(
                        theme,
                        StaticIcon::Cpu,
                        "CPU Usage".to_string(),
                        match self.config.cpu.format {
                            CpuFormat::Percentage => format!("{}%", self.data.cpu_usage.percentage),
                            CpuFormat::Frequency =>
                                format!("{} GHz", self.data.cpu_usage.frequency),
                        }
                    ))
                    .push(Self::info_element(
                        theme,
                        StaticIcon::Mem,
                        "Memory Usage".to_string(),
                        match self.config.memory.format {
                            MemoryFormat::Percentage =>
                                format!("{}%", self.data.memory_usage.percentage),
                            MemoryFormat::Fraction =>
                                format!("{} GiB", self.data.memory_usage.fraction),
                        }
                    ))
                    .push(Self::info_element(
                        theme,
                        StaticIcon::Mem,
                        "Swap memory Usage".to_string(),
                        match self.config.memory.format {
                            MemoryFormat::Percentage =>
                                format!("{}%", self.data.memory_swap_usage.percentage),
                            MemoryFormat::Fraction =>
                                format!("{} GiB", self.data.memory_swap_usage.fraction),
                        }
                    ))
                    .push_maybe(self.data.temperature.celsius.map(|cel| {
                        Self::info_element(
                            theme,
                            StaticIcon::Temp,
                            "Temperature".to_string(),
                            match self.config.temperature.format {
                                TemperatureFormat::Celsius => format!("{cel}°C"),
                                TemperatureFormat::Fahrenheit => {
                                    format!("{}°F", self.data.temperature.fahrenheit)
                                }
                            },
                        )
                    }))
                    .push(
                        Column::with_children(
                            self.data
                                .disks
                                .iter()
                                .map(|(mount_point, usage)| {
                                    Self::info_element(
                                        theme,
                                        StaticIcon::Drive,
                                        format!("Disk Usage {mount_point}"),
                                        match self.config.disk.format {
                                            DiskFormat::Percentage => {
                                                format!("{}%", usage.percentage)
                                            }
                                            DiskFormat::Fraction => {
                                                format!("{} GB", usage.fraction)
                                            }
                                        },
                                    )
                                })
                                .collect::<Vec<Element<_>>>(),
                        )
                        .spacing(theme.space.xxs),
                    )
                    .push_maybe(self.data.network.as_ref().map(|network| {
                        Column::with_children(vec![
                            Self::info_element(
                                theme,
                                StaticIcon::IpAddress,
                                "IP Address".to_string(),
                                network.ip.to_string(),
                            ),
                            Self::info_element(
                                theme,
                                StaticIcon::DownloadSpeed,
                                "Download Speed".to_string(),
                                if network.download_speed > 1000 {
                                    format!("{} MB/s", network.download_speed / 1000)
                                } else {
                                    format!("{} KB/s", network.download_speed)
                                },
                            ),
                            Self::info_element(
                                theme,
                                StaticIcon::UploadSpeed,
                                "Upload Speed".to_string(),
                                if network.upload_speed > 1000 {
                                    format!("{} MB/s", network.upload_speed / 1000)
                                } else {
                                    format!("{} KB/s", network.upload_speed)
                                },
                            ),
                        ])
                    }))
                    .spacing(theme.space.xxs)
                    .padding([0.0, theme.space.xs])
            )
            .spacing(theme.space.xs),
        )
        .width(MenuSize::Medium)
        .into()
    }

    pub fn view(&'_ self, theme: &AshellTheme) -> Element<'_, Message> {
        let indicators = self.config.indicators.iter().filter_map(|i| match i {
            SystemInfoIndicator::Cpu => Some(Self::indicator_info_element(
                theme,
                StaticIcon::Cpu,
                match self.config.cpu.format {
                    CpuFormat::Percentage => (self.data.cpu_usage.percentage.to_string(), "%"),
                    CpuFormat::Frequency => (self.data.cpu_usage.frequency.to_string(), " GHz"),
                },
                match self.config.cpu.format {
                    // note quite sure on how to interpret thresholds with other types of display values yet.
                    CpuFormat::Percentage => Some((
                        self.data.cpu_usage.percentage,
                        self.config.cpu.warn_threshold,
                        self.config.cpu.alert_threshold,
                    )),
                    CpuFormat::Frequency => None,
                },
                None,
            )),

            SystemInfoIndicator::Memory => Some(Self::indicator_info_element(
                theme,
                StaticIcon::Mem,
                match self.config.memory.format {
                    MemoryFormat::Percentage => {
                        (self.data.memory_usage.percentage.to_string(), "%")
                    }
                    MemoryFormat::Fraction => (self.data.memory_usage.fraction.clone(), " GiB"),
                },
                match self.config.memory.format {
                    MemoryFormat::Percentage => Some((
                        self.data.memory_usage.percentage,
                        self.config.memory.warn_threshold,
                        self.config.memory.alert_threshold,
                    )),
                    MemoryFormat::Fraction => None,
                },
                None,
            )),

            SystemInfoIndicator::MemorySwap => Some(Self::indicator_info_element(
                theme,
                StaticIcon::Mem,
                match self.config.memory.format {
                    MemoryFormat::Percentage => {
                        (self.data.memory_swap_usage.percentage.to_string(), "%")
                    }
                    MemoryFormat::Fraction => {
                        (self.data.memory_swap_usage.fraction.clone(), " GiB")
                    }
                },
                match self.config.memory.format {
                    MemoryFormat::Percentage => Some((
                        self.data.memory_usage.percentage,
                        self.config.memory.warn_threshold,
                        self.config.memory.alert_threshold,
                    )),
                    MemoryFormat::Fraction => None,
                },
                Some("swap"),
            )),

            SystemInfoIndicator::Temperature => self.data.temperature.celsius.map(|cel| {
                Self::indicator_info_element(
                    theme,
                    StaticIcon::Temp,
                    match self.config.temperature.format {
                        TemperatureFormat::Celsius => (cel, "°C"),
                        TemperatureFormat::Fahrenheit => (self.data.temperature.fahrenheit, "°F"),
                    },
                    Some((
                        cel,
                        self.config.temperature.warn_threshold,
                        self.config.temperature.alert_threshold,
                    )),
                    None,
                )
            }),
            SystemInfoIndicator::Disk(config) => {
                self.data.disks.iter().find_map(|(disk_mount, disk)| {
                    if disk_mount == &config.path {
                        Some(Self::indicator_info_element(
                            theme,
                            StaticIcon::Drive,
                            match self.config.disk.format {
                                DiskFormat::Fraction => (disk.percentage.to_string(), "%"),
                                DiskFormat::Percentage => (disk.fraction.clone(), " GB"),
                            },
                            Some((
                                disk.percentage,
                                self.config.disk.warn_threshold,
                                self.config.disk.alert_threshold,
                            )),
                            Some(config.name.as_deref().unwrap_or(disk_mount)),
                        ))
                    } else {
                        None
                    }
                })
            }
            SystemInfoIndicator::IpAddress => self.data.network.as_ref().map(|network| {
                Self::indicator_info_element(
                    theme,
                    StaticIcon::IpAddress,
                    (network.ip.to_string(), ""),
                    None::<(u32, u32, u32)>,
                    None,
                )
            }),
            SystemInfoIndicator::DownloadSpeed => self.data.network.as_ref().map(|network| {
                Self::indicator_info_element(
                    theme,
                    StaticIcon::DownloadSpeed,
                    (
                        if network.download_speed > 1000 {
                            network.download_speed / 1000
                        } else {
                            network.download_speed
                        },
                        if network.download_speed > 1000 {
                            "MB/s"
                        } else {
                            "KB/s"
                        },
                    ),
                    None::<(u32, u32, u32)>,
                    None,
                )
            }),
            SystemInfoIndicator::UploadSpeed => self.data.network.as_ref().map(|network| {
                Self::indicator_info_element(
                    theme,
                    StaticIcon::UploadSpeed,
                    (
                        if network.upload_speed > 1000 {
                            network.upload_speed / 1000
                        } else {
                            network.upload_speed
                        },
                        if network.upload_speed > 1000 {
                            "MB/s"
                        } else {
                            "KB/s"
                        },
                    ),
                    None::<(u32, u32, u32)>,
                    None,
                )
            }),
        });

        Row::with_children(indicators)
            .align_y(Alignment::Center)
            .spacing(theme.space.xxs)
            .into()
    }

    pub fn subscription(&self) -> Subscription<Message> {
        every(Duration::from_secs(self.config.interval)).map(|_| Message::Update)
    }
}
