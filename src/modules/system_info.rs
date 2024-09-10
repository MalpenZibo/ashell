use crate::{
    components::icons::{icon, Icons},
    config::{Orientation, SystemModuleConfig},
    style::header_pills,
};
use iced::{
    alignment::Vertical,
    time,
    widget::{container, row, text, Column, Row},
    Alignment, Element, Subscription, Theme,
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

    pub fn view(
        &self,
        config: &SystemModuleConfig,
        orientation: Orientation,
    ) -> Option<Element<Message>> {
        if config.disabled {
            None
        } else {
            let cpu_usage = self.data.cpu_usage;
            let memory_usage = self.data.memory_usage;
            let temperature = self.data.temperature.unwrap_or_default() as i32;

            let cpu_warn_threshold = config.cpu_warn_threshold;
            let cpu_alert_threshold = config.cpu_alert_threshold;

            let mem_warn_threshold = config.mem_warn_threshold;
            let mem_alert_threshold = config.mem_alert_threshold;

            let temp_warn_threshold = config.temp_warn_threshold;
            let temp_alert_threshold = config.temp_alert_threshold;

            let cpu = vec![
                icon(Icons::Cpu).into(),
                text(format!("{}%", cpu_usage)).into(),
            ];
            let mem = vec![
                icon(Icons::Mem).into(),
                text(format!("{}%", memory_usage)).into(),
            ];
            let temp = vec![
                icon(Icons::Temp).into(),
                text(format!("{}°", temperature)).into(),
            ];

            let cpu_content: Element<Message> = match orientation {
                Orientation::Horizontal => Row::with_children(cpu).spacing(4).into(),
                Orientation::Vertical => Column::with_children(cpu)
                    .align_items(Alignment::Center)
                    .spacing(4)
                    .into(),
            };
            let mem_content: Element<Message> = match orientation {
                Orientation::Horizontal => Row::with_children(mem).spacing(4).into(),
                Orientation::Vertical => Column::with_children(mem)
                    .align_items(Alignment::Center)
                    .spacing(4)
                    .into(),
            };
            let temp_content: Element<Message> = match orientation {
                Orientation::Horizontal => Row::with_children(temp).spacing(4).into(),
                Orientation::Vertical => Column::with_children(temp)
                    .align_items(Alignment::Center)
                    .spacing(4)
                    .into(),
            };

            let content = vec![
                container(cpu_content)
                    .style(move |theme: &Theme| container::Appearance {
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
                    })
                    .into(),
                container(mem_content)
                    .style(move |theme: &Theme| container::Appearance {
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
                    })
                    .into(),
                container(temp_content)
                    .style(move |theme: &Theme| container::Appearance {
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
                    .into(),
            ];
            let main_content: Element<Message> = match orientation {
                Orientation::Horizontal => Row::with_children(content)
                    .align_items(Alignment::Center)
                    .into(),
                Orientation::Vertical => Column::with_children(content)
                    .align_items(Alignment::Center)
                    .spacing(4)
                    .into(),
            };
            Some(
                container(main_content)
                    .align_y(Vertical::Center)
                    .padding([2, 7])
                    .style(header_pills)
                    .into(),
            )
        }
    }

    pub fn subscription(&self) -> Subscription<Message> {
        time::every(Duration::from_secs(5)).map(|_| Message::Update)
    }
}
