use std::fs::read_to_string;
use iced::Color;

use crate::{components::icons::Icons, style::{GREEN, TEXT, RED}};

#[derive(Copy, Clone)]
pub struct BatteryData {
    pub capacity: i64,
    pub status: BatteryStatus,
}

#[derive(Copy, Clone)]
pub enum BatteryStatus {
    Charging,
    Discharging,
}

impl BatteryData {
    pub fn get_color(&self) -> Color {
        match self {
            BatteryData {
                status: BatteryStatus::Charging,
                ..
            } => GREEN,
            BatteryData {
                status: BatteryStatus::Discharging,
                capacity,
            } if *capacity < 20 => RED,
            _ => TEXT,
        }
    }

    pub fn get_icon(&self) -> Icons {
        match self {
            BatteryData {
                status: BatteryStatus::Charging,
                ..
            } => Icons::BatteryCharging,
            BatteryData {
                status: BatteryStatus::Discharging,
                capacity,
            } if *capacity < 20 => Icons::Battery0,
            BatteryData {
                status: BatteryStatus::Discharging,
                capacity,
            } if *capacity < 40 => Icons::Battery1,
            BatteryData {
                status: BatteryStatus::Discharging,
                capacity,
            } if *capacity < 60 => Icons::Battery2,
            BatteryData {
                status: BatteryStatus::Discharging,
                capacity,
            } if *capacity < 80 => Icons::Battery3,
            _ => Icons::Battery4,
        }
    }
}

pub fn get_battery_capacity() -> Option<BatteryData> {
    let power_supply_dir = std::path::Path::new("/sys/class/power_supply/BAT0");

    if let (Ok(capacity), Ok(status)) = (
        read_to_string(power_supply_dir.join("capacity")),
        read_to_string(power_supply_dir.join("status")),
    ) {
        capacity
            .trim_end_matches('\n')
            .parse::<f64>()
            .map(|c| BatteryData {
                status: match status.trim_end_matches('\n') {
                    "Charging" => BatteryStatus::Charging,
                    _ => BatteryStatus::Discharging,
                },
                capacity: c.round() as i64,
            })
            .ok()
    } else {
        None
    }
}

