//! A laptop on its battery, with headphones on Bluetooth and the microphone
//! muted: for pictures.

use std::time::Duration;

use ashell_services::audio::{AudioData, Device, NORMAL_VOLUME, Port, PortKind};
use ashell_services::bluetooth::{BluetoothData, BluetoothDevice, BluetoothState};
use ashell_services::brightness::BrightnessData;
use ashell_services::upower::{
    BatteryData, BatteryStatus, Peripheral, PeripheralDeviceKind, PowerProfile, UPowerData,
};
use guido::prelude::*;

use super::{Idle, Settings};
use crate::services::fixed;

pub(super) fn battery(capacity: i64, status: BatteryStatus) -> BatteryData {
    BatteryData {
        capacity,
        status,
        is_discharging: matches!(status, BatteryStatus::Discharging(_)),
        health: None,
    }
}

pub fn settings(config: Memo<super::SettingsConfig>) -> Settings {
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

    let device = |name: &str, description: &str, volume: u32, is_mute| Device {
        name: name.into(),
        description: description.into(),
        volume,
        is_mute,
        ports: Vec::new(),
        active_port: None,
    };
    let port = |name: &str, kind| Port {
        name: name.into(),
        description: name.into(),
        kind,
    };
    let audio = AudioData {
        default_sink: "speakers".into(),
        default_source: "microphone".into(),
        sinks: vec![
            Device {
                ports: vec![
                    port("Speakers", PortKind::Speaker),
                    port("Headphones", PortKind::Headphones),
                ],
                active_port: Some("Speakers".into()),
                ..device(
                    "speakers",
                    "Built-in audio",
                    NORMAL_VOLUME * 45 / 100,
                    false,
                )
            },
            device("monitor", "Monitor", NORMAL_VOLUME, false),
        ],
        sources: vec![device(
            "microphone",
            "Microphone",
            NORMAL_VOLUME * 80 / 100,
            true,
        )],
    };

    Settings {
        config,
        upower: fixed(upower),
        audio: fixed(audio),
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
