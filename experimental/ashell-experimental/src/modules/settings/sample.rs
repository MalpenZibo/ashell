//! A laptop on its battery, with headphones on Bluetooth: for pictures.

use std::time::Duration;

use ashell_services::bluetooth::{BluetoothData, BluetoothDevice, BluetoothState};
use ashell_services::brightness::BrightnessData;
use ashell_services::upower::{
    BatteryData, BatteryStatus, Peripheral, PeripheralDeviceKind, PowerProfile, UPowerData,
};
use guido::prelude::*;

use super::{Idle, Settings};
use crate::services::fixed;

fn battery(capacity: i64, status: BatteryStatus) -> BatteryData {
    BatteryData {
        capacity,
        status,
        is_discharging: matches!(status, BatteryStatus::Discharging(_)),
        health: None,
    }
}

pub fn settings() -> Settings {
    let upower = UPowerData {
        on_battery: Some(true),
        system_battery: Some(battery(
            68,
            BatteryStatus::Discharging(Duration::from_secs(4 * 3600)),
        )),
        peripherals: vec![Peripheral {
            name: "Headphones".into(),
            kind: PeripheralDeviceKind::Headphones,
            data: battery(15, BatteryStatus::Discharging(Duration::ZERO)),
            path: Default::default(),
        }],
        power_profile: PowerProfile::Balanced,
        ..UPowerData::default()
    };
    let bluetooth = BluetoothData {
        state: BluetoothState::Active,
        devices: vec![
            BluetoothDevice {
                name: "Headphones".into(),
                battery: Some(15),
                path: Default::default(),
                connected: true,
                paired: true,
            },
            BluetoothDevice {
                name: "Keyboard".into(),
                battery: None,
                path: Default::default(),
                connected: false,
                paired: true,
            },
            BluetoothDevice {
                name: "Speaker".into(),
                battery: None,
                path: Default::default(),
                connected: false,
                paired: false,
            },
        ],
        discovering: false,
    };

    Settings {
        upower: fixed(upower),
        bluetooth: fixed(bluetooth),
        brightness: fixed(Some(BrightnessData {
            current: 600,
            max: 1000,
        })),
        idle: Idle {
            inhibited: create_signal(Some(false)).read_only(),
            toggle: create_service::<(), _, _>(|_, _| async {}),
        },
    }
}
