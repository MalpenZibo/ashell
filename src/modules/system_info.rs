use crate::{
    components::icons::{icon, Icons},
    config::SystemModuleConfig,
    style::{header_pills, RED, TEXT, YELLOW},
};
use iced::{
    widget::{container, row, text},
    Element,
};
use std::time::Duration;
use sysinfo::{Components, System};

struct SystemInfoData {
    pub cpu_usage: u32,
    pub memory_usage: u32,
    pub temperature: Option<f32>,
}

fn get_system_info(system: &mut System, components: &mut Components) -> SystemInfoData {
    system.refresh_memory();
    system.refresh_cpu_specifics(sysinfo::CpuRefreshKind::everything());

    components.refresh_list();
    components.refresh();

    let cpu_usage = system.global_cpu_info().cpu_usage().floor() as u32;
    let memory_usage = ((system.total_memory() - system.available_memory()) as f32
        / system.total_memory() as f32
        * 100.) as u32;

    let temperature = components
        .iter()
        .find(|c| c.label() == "acpitz temp1")
        .map(|c| c.temperature());

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

#[derive(Debug, Clone)]
pub enum Message {
    Update,
}

impl SystemInfo {
    pub fn new() -> Self {
        let mut system = System::new();
        let mut components = Components::new_with_refreshed_list();
        let data = get_system_info(&mut system, &mut components);

        Self {
            system,
            components,
            data,
        }
    }

    pub fn update(&mut self, message: Message) {
        match message {
            Message::Update => {
                let data = get_system_info(&mut self.system, &mut self.components);
                self.data = data;
            }
        }
    }

    pub fn view(&self, config: &SystemModuleConfig) -> Option<Element<Message>> {
        if config.disabled {
            None
        } else {
            let cpu_color = if self.data.cpu_usage > config.cpu_warn_threshold
                && self.data.cpu_usage < config.cpu_alert_threshold
            {
                YELLOW
            } else if self.data.cpu_usage >= config.cpu_alert_threshold {
                RED
            } else {
                TEXT
            };

            let ram_color = if self.data.memory_usage > config.mem_warn_threshold
                && self.data.memory_usage < config.mem_alert_threshold
            {
                YELLOW
            } else if self.data.memory_usage >= config.mem_alert_threshold {
                RED
            } else {
                TEXT
            };

            let temp = self.data.temperature.unwrap_or_default() as i32;
            let temp_color =
                if temp > config.temp_warn_threshold && temp < config.temp_alert_threshold {
                    YELLOW
                } else if temp >= config.temp_alert_threshold {
                    RED
                } else {
                    TEXT
                };

            Some(
                container(
                    row!(
                        icon(Icons::Cpu).style(cpu_color),
                        text(format!("{}%", self.data.cpu_usage)).style(cpu_color),
                        icon(Icons::Mem).style(ram_color),
                        text(format!("{}%", self.data.memory_usage)).style(ram_color),
                        icon(Icons::Temp).style(temp_color),
                        text(format!("{}°", self.data.temperature.unwrap_or_default()))
                            .style(temp_color)
                    )
                    .align_items(iced::Alignment::Center)
                    .spacing(4),
                )
                .align_y(iced::alignment::Vertical::Center)
                .padding([2, 7])
                .style(header_pills)
                .into(),
            )
        }
    }

    pub fn subscription(&self) -> iced::Subscription<Message> {
        iced::time::every(Duration::from_secs(5)).map(|_| Message::Update)
    }
}
