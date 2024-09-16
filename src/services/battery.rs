use std::time::Duration;

use iced::{
    futures::{
        stream::{self, select_all},
        SinkExt, StreamExt,
    },
    subscription::channel,
    Subscription,
};
use log::info;
use zbus::{proxy, zvariant::OwnedObjectPath, Connection, Result};

use crate::{components::icons::Icons, utils::IndicatorState};

#[derive(Copy, Clone, Debug)]
pub struct BatteryData {
    pub capacity: i64,
    pub status: BatteryStatus,
}

impl BatteryData {
    pub fn get_indicator_state(&self) -> IndicatorState {
        match self {
            BatteryData {
                status: BatteryStatus::Charging(_),
                ..
            } => IndicatorState::Success,
            BatteryData {
                status: BatteryStatus::Discharging(_),
                capacity,
            } if *capacity < 20 => IndicatorState::Danger,
            _ => IndicatorState::Normal,
        }
    }

    pub fn get_icon(&self) -> Icons {
        match self {
            BatteryData {
                status: BatteryStatus::Charging(_),
                ..
            } => Icons::BatteryCharging,
            BatteryData {
                status: BatteryStatus::Discharging(_),
                capacity,
            } if *capacity < 20 => Icons::Battery0,
            BatteryData {
                status: BatteryStatus::Discharging(_),
                capacity,
            } if *capacity < 40 => Icons::Battery1,
            BatteryData {
                status: BatteryStatus::Discharging(_),
                capacity,
            } if *capacity < 60 => Icons::Battery2,
            BatteryData {
                status: BatteryStatus::Discharging(_),
                capacity,
            } if *capacity < 80 => Icons::Battery3,
            _ => Icons::Battery4,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum BatteryMessage {
    PercentageChanged(i64),
    StatusChanged(BatteryStatus),
}

#[derive(Copy, Clone, Debug)]
pub enum BatteryStatus {
    Charging(Duration),
    Discharging(Duration),
    Full,
}

pub fn subscription() -> Subscription<BatteryMessage> {
    channel("battery-listener", 100, |mut output| async move {
        let conn = Connection::system().await.unwrap();
        let upower = UPowerProxy::new(&conn).await.unwrap();

        loop {
            let devices = upower.enumerate_devices().await.unwrap();

            let battery = stream::iter(devices.into_iter())
                .filter_map(|device| {
                    let conn = conn.clone();
                    async move {
                        let device = DeviceProxy::builder(&conn)
                            .path(device)
                            .unwrap()
                            .build()
                            .await
                            .unwrap();
                        let device_type = device.device_type().await.unwrap();
                        let power_supply = device.power_supply().await.unwrap();
                        if device_type == 2 && power_supply {
                            Some(device)
                        } else {
                            None
                        }
                    }
                })
                .collect::<Vec<_>>()
                .await;

            if let Some(battery) = battery.first() {
                let state = battery.state().await.unwrap();
                let state = match state {
                    1 => BatteryStatus::Charging(Duration::from_secs(
                        battery.time_to_full().await.unwrap_or_default() as u64,
                    )),
                    2 => BatteryStatus::Discharging(Duration::from_secs(
                        battery.time_to_empty().await.unwrap_or_default() as u64,
                    )),
                    4 => BatteryStatus::Full,
                    _ => BatteryStatus::Discharging(Duration::from_secs(0)),
                };
                let percentage = battery.percentage().await.unwrap_or_default() as i64;

                let _ = output.feed(BatteryMessage::StatusChanged(state)).await;
                let _ = output
                    .feed(BatteryMessage::PercentageChanged(percentage))
                    .await;
                let _ = output.flush().await;

                let state_signal = battery
                    .receive_state_changed()
                    .await
                    .then(|v| async move {
                        if let Ok(value) = v.get().await {
                            Some(BatteryMessage::StatusChanged(match value {
                                1 => BatteryStatus::Charging(Duration::from_secs(
                                    battery.time_to_full().await.unwrap_or_default() as u64,
                                )),
                                2 => BatteryStatus::Discharging(Duration::from_secs(
                                    battery.time_to_empty().await.unwrap_or_default() as u64,
                                )),
                                4 => BatteryStatus::Full,
                                _ => BatteryStatus::Discharging(Duration::from_secs(0)),
                            }))
                        } else {
                            None
                        }
                    })
                    .boxed();
                let percentage_signal = battery
                    .receive_percentage_changed()
                    .await
                    .then(|v| async move {
                        if let Ok(value) = v.get().await {
                            Some(BatteryMessage::PercentageChanged(value as i64))
                        } else {
                            None
                        }
                    })
                    .boxed();
                let time_to_full_signal = battery
                    .receive_time_to_full_changed()
                    .await
                    .then(|v| async move {
                        if let Ok(value) = v.get().await {
                            if value > 0 {
                                Some(BatteryMessage::StatusChanged(BatteryStatus::Charging(
                                    Duration::from_secs(value as u64),
                                )))
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    })
                    .boxed();
                let time_to_empty_signal = battery
                    .receive_time_to_empty_changed()
                    .await
                    .then(|v| async move {
                        if let Ok(value) = v.get().await {
                            if value > 0 {
                                Some(BatteryMessage::StatusChanged(BatteryStatus::Discharging(
                                    Duration::from_secs(value as u64),
                                )))
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    })
                    .boxed();

                let mut combined = select_all(vec![
                    state_signal,
                    percentage_signal,
                    time_to_full_signal,
                    time_to_empty_signal,
                ]);

                while let Some(event) = combined.next().await {
                    if let Some(battery_message) = event {
                        let _ = output.send(battery_message).await;
                    }
                }
            } else {
                let _ = upower.receive_device_added().await;
                info!("upower device added");
            }
        }
    })
}

#[proxy(
    interface = "org.freedesktop.UPower",
    default_service = "org.freedesktop.UPower",
    default_path = "/org/freedesktop/UPower"
)]
trait UPower {
    fn enumerate_devices(&self) -> Result<Vec<OwnedObjectPath>>;

    #[zbus(signal)]
    fn device_added(&self) -> Result<OwnedObjectPath>;
}

#[proxy(
    default_service = "org.freedesktop.UPower",
    default_path = "/org/freedesktop/UPower/Device",
    interface = "org.freedesktop.UPower.Device"
)]
trait Device {
    #[zbus(property, name = "Type")]
    fn device_type(&self) -> Result<u32>;

    #[zbus(property)]
    fn power_supply(&self) -> Result<bool>;

    #[zbus(property)]
    fn time_to_empty(&self) -> Result<i64>;

    #[zbus(property)]
    fn time_to_full(&self) -> Result<i64>;

    #[zbus(property)]
    fn percentage(&self) -> Result<f64>;

    #[zbus(property)]
    fn state(&self) -> Result<u32>;
}
