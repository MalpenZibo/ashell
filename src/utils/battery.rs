use std::fs::read_to_string;

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
    pub fn get_class(&self) -> Vec<&'static str> {
        match self {
            BatteryData {
                status: BatteryStatus::Charging,
                ..
            } => vec!["battery", "charging"],
            BatteryData {
                status: BatteryStatus::Discharging,
                capacity,
            } if *capacity < 20 => vec!["battery", "critical"],
            _ => vec!["battery"],
        }
    }

    pub fn get_icon(&self) -> &str {
        match self {
            BatteryData {
                status: BatteryStatus::Charging,
                ..
            } => "󰂄",
            BatteryData {
                status: BatteryStatus::Discharging,
                capacity,
            } if *capacity < 20 => "󰂃",
            BatteryData {
                status: BatteryStatus::Discharging,
                capacity,
            } if *capacity < 40 => "󰁼",
            BatteryData {
                status: BatteryStatus::Discharging,
                capacity,
            } if *capacity < 60 => "󰁾",
            BatteryData {
                status: BatteryStatus::Discharging,
                capacity,
            } if *capacity < 80 => "󰂀",
            _ => "󰁹",
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

