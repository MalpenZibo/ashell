use crate::{
    app,
    components::icons::{icon, Icons},
    config::SystemModuleConfig,
};
use iced::{
    time::every,
    widget::{container, row, text, Row},
    Alignment, Element, Subscription, Theme,
};
use std::time::Duration;
use sysinfo::{Components, System};

use super::{Module, OnModulePress};

struct SystemInfoData {
    pub cpu_usage: u32,
    pub memory_usage: u32,
    pub temperature: Option<i32>,
}

fn get_system_info(system: &mut System, components: &mut Components) -> SystemInfoData {
    system.refresh_memory();
    system.refresh_cpu_specifics(sysinfo::CpuRefreshKind::everything());

    components.refresh_list();
    components.refresh();

    let cpu_usage = system.global_cpu_usage().floor() as u32;
    let memory_usage = ((system.total_memory() - system.available_memory()) as f32
        / system.total_memory() as f32
        * 100.) as u32;

    let temperature = components
        .iter()
        .find(|c| c.label() == "acpitz temp1")
        .map(|c| c.temperature() as i32);

    SystemInfoData {
        cpu_usage,
        memory_usage,
        temperature,
    }
}

pub struct SystemInfo {
    system: System,
    components: Components,
    data: SystemInfoData,
}

impl Default for SystemInfo {
    fn default() -> Self {
        let mut system = System::new();
        let mut components = Components::new_with_refreshed_list();
        let data = get_system_info(&mut system, &mut components);

        Self {
            system,
            components,
            data,
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
                self.data = get_system_info(&mut self.system, &mut self.components);
            }
        }
    }

    pub fn subscription(&self) -> Subscription<Message> {
        every(Duration::from_secs(5)).map(|_| Message::Update)
    }
}

impl Module for SystemInfo {
    type Data<'a> = &'a SystemModuleConfig;

    fn view<'a>(
        &self,
        config: Self::Data<'a>,
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
                .align_y(Alignment::Center)
                .spacing(4)
                .into(),
            None,
        ))
    }
}
