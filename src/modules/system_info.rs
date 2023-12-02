use crate::{
    components::icons::{icon, Icons},
    style::{header_pills, RED, TEXT, YELLOW},
};
use iced::{
    widget::{container, row, text},
    Element,
};
use std::time::Duration;
use sysinfo::{ComponentExt, CpuExt, System, SystemExt};

struct SystemInfoData {
    pub cpu_usage: u32,
    pub memory_usage: u32,
    pub temperature: Option<f32>,
}

fn get_system_info(system: &mut System) -> SystemInfoData {
    system.refresh_memory();
    system.refresh_cpu_specifics(sysinfo::CpuRefreshKind::everything());
    system.refresh_components_list();
    system.refresh_components();

    let cpu_usage = system.global_cpu_info().cpu_usage().floor() as u32;
    let memory_usage = ((system.total_memory() - system.available_memory()) as f32
        / system.total_memory() as f32
        * 100.) as u32;

    let temperature = system
        .components()
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
    data: SystemInfoData,
}

#[derive(Debug, Clone)]
pub enum Message {
    Update,
}

impl SystemInfo {
    pub fn new() -> Self {
        let mut system = System::new();
        let data = get_system_info(&mut system);

        Self { system, data }
    }

    pub fn update(&mut self, message: Message) {
        match message {
            Message::Update => {
                let data = get_system_info(&mut self.system);
                self.data = data;
            }
        }
    }

    pub fn view(&self) -> Element<Message> {
        let cpu_color = match self.data.cpu_usage {
            60..=80 => YELLOW,
            81..=100 => RED,
            _ => TEXT,
        };
        let ram_color = match self.data.memory_usage {
            70..=85 => YELLOW,
            86..=100 => RED,
            _ => TEXT,
        };
        let temp_color = match self.data.temperature.unwrap_or_default() as i32 {
            60..=80 => YELLOW,
            81.. => RED,
            _ => TEXT,
        };
        container(
            row!(
                icon(Icons::Cpu).style(cpu_color),
                text(format!("{}%", self.data.cpu_usage)).style(cpu_color),
                icon(Icons::Mem).style(ram_color),
                text(format!("{}%", self.data.memory_usage)).style(ram_color),
                icon(Icons::Temp).style(temp_color),
                text(format!("{}°", self.data.temperature.unwrap_or_default())).style(temp_color)
            )
            .align_items(iced::Alignment::Center)
            .spacing(4),
        )
        .align_y(iced::alignment::Vertical::Center)
        .padding([4, 8])
        .style(header_pills)
        .into()
    }

    pub fn subscription(&self) -> iced::Subscription<Message> {
        iced::time::every(Duration::from_secs(5)).map(|_| Message::Update)
    }
}
