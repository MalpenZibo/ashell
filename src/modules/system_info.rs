use crate::{
    app,
    components::icons::{Icons, icon},
    config::{SystemIndicator, SystemModuleConfig},
    menu::MenuType,
};
use iced::{
    Alignment, Element, Length, Subscription, Task, Theme,
    time::every,
    widget::{Column, Row, column, container, horizontal_rule, row, text},
};
use itertools::Itertools;
use std::time::{Duration, Instant};
use sysinfo::{Components, Disks, Networks, System};

use super::{Module, OnModulePress};

struct NetworkData {
    ip: String,
    download_speed: u32,
    upload_speed: u32,
    last_check: Instant,
}

struct SystemInfoData {
    pub cpu_usage: u32,
    pub memory_usage: u32,
    pub memory_swap_usage: u32,
    pub temperature: Option<i32>,
    pub disks: Vec<(String, u32)>,
    pub network: Option<NetworkData>,
}

fn get_system_info(
    system: &mut System,
    components: &mut Components,
    disks: &mut Disks,
    (networks, last_check): (&mut Networks, Option<Instant>),
) -> SystemInfoData {
    system.refresh_memory();
    system.refresh_cpu_specifics(sysinfo::CpuRefreshKind::everything());

    components.refresh(true);
    disks.refresh(true);
    networks.refresh(true);

    let cpu_usage = system.global_cpu_usage().floor() as u32;
    let memory_usage = ((system.total_memory() - system.available_memory()) as f32
        / system.total_memory() as f32
        * 100.) as u32;

    let memory_swap_usage = ((system.total_swap() - system.free_swap()) as f32
        / system.total_swap() as f32
        * 100.) as u32;

    let temperature = components
        .iter()
        .find(|c| c.label() == "acpitz temp1")
        .and_then(|c| c.temperature().map(|t| t as i32));

    let disks = disks
        .into_iter()
        .filter(|d| !d.is_removable() && d.total_space() != 0)
        .map(|d| {
            (
                d.mount_point().to_string_lossy().to_string(),
                (((d.total_space() - d.available_space()) as f32) / d.total_space() as f32 * 100.)
                    as u32,
            )
        })
        .sorted_by(|a, b| a.0.cmp(&b.0))
        .collect::<Vec<_>>();

    let elapsed = last_check.map(|v| v.elapsed().as_secs());

    let network = networks.iter().fold(
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
        network: network.0.map(|ip| NetworkData {
            ip: ip.to_string(),
            download_speed: network_speed(network.1),
            upload_speed: network_speed(network.2),
            last_check: Instant::now(),
        }),
    }
}

pub struct SystemInfo {
    system: System,
    components: Components,
    disks: Disks,
    networks: Networks,
    data: SystemInfoData,
}

impl Default for SystemInfo {
    fn default() -> Self {
        let mut system = System::new();
        let mut components = Components::new_with_refreshed_list();
        let mut disks = Disks::new_with_refreshed_list();
        let mut networks = Networks::new_with_refreshed_list();
        let data = get_system_info(
            &mut system,
            &mut components,
            &mut disks,
            (&mut networks, None),
        );

        Self {
            system,
            components,
            disks,
            data,
            networks,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    Update,
}

impl SystemInfo {
    pub fn update(&mut self, message: Message) -> Task<crate::app::Message> {
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
                );

                Task::none()
            }
        }
    }

    fn info_element<'a>(info_icon: Icons, label: String, value: String) -> Element<'a, Message> {
        row!(
            container(icon(info_icon).size(22)).center_x(Length::Fixed(32.)),
            text(label).width(Length::Fill),
            text(value)
        )
        .align_y(Alignment::Center)
        .spacing(8)
        .into()
    }

    fn indicator_info_element<'a, V: std::fmt::Display + PartialOrd + 'a>(
        info_icon: Icons,
        value: V,
        unit: &str,
        threshold: Option<(V, V)>,
        prefix: Option<&str>,
    ) -> Element<'a, app::Message> {
        let element = container(
            row!(
                icon(info_icon),
                if let Some(prefix) = prefix {
                    text(format!("{prefix} {value}{unit}"))
                } else {
                    text(format!("{value}{unit}"))
                }
            )
            .spacing(4),
        );

        if let Some((warn_threshold, alert_threshold)) = threshold {
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

    pub fn menu_view(&self) -> Element<Message> {
        column!(
            text("System Info").size(20),
            horizontal_rule(1),
            Column::new()
                .push(Self::info_element(
                    Icons::Cpu,
                    "CPU Usage".to_string(),
                    format!("{}%", self.data.cpu_usage),
                ))
                .push(Self::info_element(
                    Icons::Mem,
                    "Memory Usage".to_string(),
                    format!("{}%", self.data.memory_usage),
                ))
                .push(Self::info_element(
                    Icons::Mem,
                    "Swap memory Usage".to_string(),
                    format!("{}%", self.data.memory_swap_usage),
                ))
                .push_maybe(self.data.temperature.map(|temp| {
                    Self::info_element(Icons::Temp, "Temperature".to_string(), format!("{temp}°C"))
                }))
                .push(
                    Column::with_children(
                        self.data
                            .disks
                            .iter()
                            .map(|(mount_point, usage)| {
                                Self::info_element(
                                    Icons::Drive,
                                    format!("Disk Usage {mount_point}"),
                                    format!("{usage}%"),
                                )
                            })
                            .collect::<Vec<Element<_>>>(),
                    )
                    .spacing(4),
                )
                .push_maybe(self.data.network.as_ref().map(|network| {
                    Column::with_children(vec![
                        Self::info_element(
                            Icons::IpAddress,
                            "IP Address".to_string(),
                            network.ip.clone(),
                        ),
                        Self::info_element(
                            Icons::DownloadSpeed,
                            "Download Speed".to_string(),
                            if network.download_speed > 1000 {
                                format!("{} MB/s", network.download_speed / 1000)
                            } else {
                                format!("{} KB/s", network.download_speed)
                            },
                        ),
                        Self::info_element(
                            Icons::UploadSpeed,
                            "Upload Speed".to_string(),
                            if network.upload_speed > 1000 {
                                format!("{} MB/s", network.upload_speed / 1000)
                            } else {
                                format!("{} KB/s", network.upload_speed)
                            },
                        ),
                    ])
                }))
                .spacing(4)
                .padding([0, 8])
        )
        .spacing(8)
        .into()
    }
}

impl Module for SystemInfo {
    type ViewData<'a> = &'a SystemModuleConfig;
    type SubscriptionData<'a> = ();

    fn view(
        &self,
        config: Self::ViewData<'_>,
    ) -> Option<(Element<app::Message>, Option<OnModulePress>)> {
        let indicators = config.indicators.iter().filter_map(|i| match i {
            SystemIndicator::Cpu => Some(Self::indicator_info_element(
                Icons::Cpu,
                self.data.cpu_usage,
                "%",
                Some((config.cpu.warn_threshold, config.cpu.alert_threshold)),
                None,
            )),
            SystemIndicator::Memory => Some(Self::indicator_info_element(
                Icons::Mem,
                self.data.memory_usage,
                "%",
                Some((config.memory.warn_threshold, config.memory.alert_threshold)),
                None,
            )),
            SystemIndicator::MemorySwap => Some(Self::indicator_info_element(
                Icons::Mem,
                self.data.memory_swap_usage,
                "%",
                Some((config.memory.warn_threshold, config.memory.alert_threshold)),
                Some("swap"),
            )),
            SystemIndicator::Temperature => self.data.temperature.map(|temperature| {
                Self::indicator_info_element(
                    Icons::Temp,
                    temperature,
                    "°C",
                    Some((
                        config.temperature.warn_threshold,
                        config.temperature.alert_threshold,
                    )),
                    None,
                )
            }),
            SystemIndicator::Disk(mount) => {
                self.data.disks.iter().find_map(|(disk_mount, disk)| {
                    if disk_mount == mount {
                        Some(Self::indicator_info_element(
                            Icons::Drive,
                            *disk,
                            "%",
                            Some((config.disk.warn_threshold, config.disk.alert_threshold)),
                            Some(disk_mount),
                        ))
                    } else {
                        None
                    }
                })
            }
            SystemIndicator::IpAddress => self.data.network.as_ref().map(|network| {
                Self::indicator_info_element(
                    Icons::IpAddress,
                    network.ip.to_string(),
                    "",
                    None,
                    None,
                )
            }),
            SystemIndicator::DownloadSpeed => self.data.network.as_ref().map(|network| {
                Self::indicator_info_element(
                    Icons::DownloadSpeed,
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
                    None,
                    None,
                )
            }),
            SystemIndicator::UploadSpeed => self.data.network.as_ref().map(|network| {
                Self::indicator_info_element(
                    Icons::UploadSpeed,
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
                    None,
                    None,
                )
            }),
        });

        Some((
            Row::with_children(indicators)
                .align_y(Alignment::Center)
                .spacing(4)
                .into(),
            Some(OnModulePress::ToggleMenu(MenuType::SystemInfo)),
        ))
    }

    fn subscription(&self, _: Self::SubscriptionData<'_>) -> Option<Subscription<app::Message>> {
        Some(every(Duration::from_secs(5)).map(|_| app::Message::SystemInfo(Message::Update)))
    }
}
