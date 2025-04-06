use crate::{
    app,
    components::icons::{Icons, icon},
    config::SystemModuleConfig,
};
use iced::{
    Alignment, Element, Subscription, Theme,
    time::every,
    widget::{Row, container, row, text},
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

    let network = networks
        .iter()
        .filter(|(name, _)| name.starts_with("wlan") || name.starts_with("eth"))
        .fold(
            (None, 0, 0),
            |(first_ip, total_received, total_transmitted), (_, data)| {
                let ip = first_ip.or_else(|| data.ip_networks().first().map(|ip| ip.addr));

                let received = data.received();
                let transmitted = data.transmitted();

                (
                    first_ip.or(ip),
                    total_received + received,
                    total_transmitted + transmitted,
                )
            },
        );

    SystemInfoData {
        cpu_usage,
        memory_usage,
        temperature,
        disks,
        network: network.0.map(|ip| NetworkData {
            ip: ip.to_string(),
            download_speed: if let Some(elapsed) = elapsed {
                (network.1 / 1000) as u32 / elapsed as u32
            } else {
                0
            },
            upload_speed: if let Some(elapsed) = elapsed {
                (network.2 / 1000) as u32 / elapsed as u32
            } else {
                0
            },
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
                );
            }
        }
    }
}

impl Module for SystemInfo {
    type ViewData<'a> = &'a SystemModuleConfig;
    type SubscriptionData<'a> = ();

    fn view(
        &self,
        config: Self::ViewData<'_>,
    ) -> Option<(Element<app::Message>, Option<OnModulePress>)> {
        let cpu_usage = self.data.cpu_usage;
        let memory_usage = self.data.memory_usage;
        let temperature = self.data.temperature;

        let cpu_warn_threshold = config.cpu_warn_threshold;
        let cpu_alert_threshold = config.cpu_alert_threshold;

        let mem_warn_threshold = config.mem_warn_threshold;
        let mem_alert_threshold = config.mem_alert_threshold;

        let temp_warn_threshold = config.temp_warn_threshold;
        let temp_alert_threshold = config.temp_alert_threshold;

        Some((
            Row::new()
                .push(
                    container(row!(icon(Icons::Cpu), text(format!("{}%", cpu_usage))).spacing(4))
                        .style(move |theme: &Theme| container::Style {
                            text_color: if cpu_usage > cpu_warn_threshold
                                && cpu_usage < cpu_alert_threshold
                            {
                                Some(theme.extended_palette().danger.weak.color)
                            } else if cpu_usage >= cpu_alert_threshold {
                                Some(theme.palette().danger)
                            } else {
                                None
                            },
                            ..Default::default()
                        }),
                )
                .push(
                    container(
                        row!(icon(Icons::Mem), text(format!("{}%", memory_usage))).spacing(4),
                    )
                    .style(move |theme: &Theme| container::Style {
                        text_color: if memory_usage > mem_warn_threshold
                            && memory_usage < mem_alert_threshold
                        {
                            Some(theme.extended_palette().danger.weak.color)
                        } else if memory_usage >= mem_alert_threshold {
                            Some(theme.palette().danger)
                        } else {
                            None
                        },
                        ..Default::default()
                    }),
                )
                .push_maybe(temperature.map(|temperature| {
                    container(row!(icon(Icons::Temp), text(format!("{}°", temperature))).spacing(4))
                        .style(move |theme: &Theme| container::Style {
                            text_color: if temperature > temp_warn_threshold
                                && temperature < temp_alert_threshold
                            {
                                Some(theme.extended_palette().danger.weak.color)
                            } else if temperature >= temp_alert_threshold {
                                Some(theme.palette().danger)
                            } else {
                                None
                            },
                            ..Default::default()
                        })
                }))
                .push(
                    Row::with_children(
                        self.data
                            .disks
                            .iter()
                            .map(|(mount_point, usage)| {
                                row!(
                                    icon(Icons::Drive),
                                    text(format!("{} {}%", mount_point, usage))
                                )
                                .spacing(4)
                                .into()
                            })
                            .collect::<Vec<Element<_>>>(),
                    )
                    .spacing(4),
                )
                .push_maybe(self.data.network.as_ref().map(|network| {
                    container(
                        row!(
                            row!(icon(Icons::IpAddress), text(&network.ip)).spacing(4),
                            row!(
                                icon(Icons::DownloadSpeed),
                                text(format!("{} Kbit/s", network.download_speed))
                            )
                            .spacing(4),
                            row!(
                                icon(Icons::UploadSpeed),
                                text(format!("{} Kbit/s", network.upload_speed))
                            )
                            .spacing(4),
                        )
                        .spacing(4),
                    )
                }))
                .align_y(Alignment::Center)
                .spacing(4)
                .into(),
            None,
        ))
    }

    fn subscription(&self, _: Self::SubscriptionData<'_>) -> Option<Subscription<app::Message>> {
        Some(every(Duration::from_secs(5)).map(|_| app::Message::SystemInfo(Message::Update)))
    }
}
