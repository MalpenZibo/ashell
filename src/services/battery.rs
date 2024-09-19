use super::{ReadOnlyService, ServiceEvent};
use crate::{components::icons::Icons, utils::IndicatorState};
use iced::{
    futures::{channel::mpsc::Sender, stream_select, SinkExt, Stream, StreamExt},
    subscription::channel,
    Subscription,
};
use std::{
    any::TypeId,
    ops::{Deref, DerefMut},
    time::Duration,
};
use zbus::{
    proxy,
    zvariant::{ObjectPath, OwnedObjectPath},
    Result,
};

#[derive(Clone, Copy, Debug)]
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

#[derive(Debug, Clone)]
pub enum BatteryEvent {
    Update(BatteryData),
}

#[derive(Copy, Clone, Debug)]
pub enum BatteryStatus {
    Charging(Duration),
    Discharging(Duration),
    Full,
}

#[derive(Debug, Clone)]
pub struct BatteryService {
    pub data: BatteryData,
    conn: zbus::Connection,
}

impl Deref for BatteryService {
    type Target = BatteryData;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl DerefMut for BatteryService {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}

enum State {
    Init,
    Active(zbus::Connection, ObjectPath<'static>),
    NoBattery,
    Error,
}

impl ReadOnlyService for BatteryService {
    type UpdateEvent = BatteryEvent;
    type Error = ();

    fn update(&mut self, event: Self::UpdateEvent) {
        match event {
            BatteryEvent::Update(data) => {
                self.data = data;
            }
        }
    }

    fn subscribe() -> Subscription<ServiceEvent<Self>> {
        let id = TypeId::of::<Self>();

        channel(id, 100, |mut output| async move {
            let mut state = State::Init;

            loop {
                state = BatteryService::start_listening(state, &mut output).await;
            }
        })
    }
}

impl BatteryService {
    async fn get_battery_device(conn: &zbus::Connection) -> anyhow::Result<Option<DeviceProxy>> {
        let upower = UPowerProxy::new(conn).await?;
        let devices = upower.enumerate_devices().await?;

        for device in devices {
            let device = DeviceProxy::builder(conn).path(device)?.build().await?;
            let device_type = device.device_type().await?;
            let power_supply = device.power_supply().await?;
            if device_type == 2 && power_supply {
                return Ok(Some(device));
            }
        }

        Ok(None)
    }

    async fn initialize_data(
        conn: &zbus::Connection,
    ) -> anyhow::Result<Option<(BatteryData, ObjectPath<'static>)>> {
        let battery = Self::get_battery_device(conn).await?;

        if let Some(battery) = battery {
            let state = battery.state().await?;
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

            Ok(Some((
                BatteryData {
                    capacity: percentage,
                    status: state,
                },
                battery.inner().path().to_owned(),
            )))
        } else {
            Ok(None)
        }
    }

    async fn events(
        conn: &zbus::Connection,
        device_path: &ObjectPath<'static>,
    ) -> anyhow::Result<impl Stream<Item = BatteryEvent>> {
        let device = DeviceProxy::builder(conn)
            .path(device_path)?
            .build()
            .await?;

        let combined = stream_select!(
            device.receive_state_changed().await.map(|_| ()),
            device.receive_percentage_changed().await.map(|_| ()),
            device.receive_time_to_full_changed().await.map(|_| ()),
            device.receive_time_to_empty_changed().await.map(|_| ()),
        )
        .map(move |_| {
            let state = device
                .cached_state()
                .unwrap_or_default()
                .unwrap_or_default();
            let state = match state {
                1 => BatteryStatus::Charging(Duration::from_secs(
                    device
                        .cached_time_to_full()
                        .unwrap_or_default()
                        .unwrap_or_default() as u64,
                )),
                2 => BatteryStatus::Discharging(Duration::from_secs(
                    device
                        .cached_time_to_empty()
                        .unwrap_or_default()
                        .unwrap_or_default() as u64,
                )),
                4 => BatteryStatus::Full,
                _ => BatteryStatus::Discharging(Duration::from_secs(0)),
            };

            BatteryEvent::Update(BatteryData {
                capacity: device
                    .cached_percentage()
                    .unwrap_or_default()
                    .unwrap_or_default() as i64,
                status: state,
            })
        });

        Ok(combined)
    }

    async fn start_listening(state: State, output: &mut Sender<ServiceEvent<Self>>) -> State {
        match state {
            State::Init => match zbus::Connection::system().await {
                Ok(conn) => {
                    let (data, path) = match BatteryService::initialize_data(&conn).await {
                        Ok(Some(data)) => data,
                        Ok(None) => return State::NoBattery,
                        Err(_) => return State::Error,
                    };

                    let service = BatteryService {
                        data,
                        conn: conn.clone(),
                    };
                    let _ = output.send(ServiceEvent::Init(service)).await;

                    State::Active(conn, path)
                }
                Err(_) => State::Error,
            },
            State::Active(conn, path) => match BatteryService::events(&conn, &path).await {
                Ok(mut events) => {
                    while let Some(event) = events.next().await {
                        let _ = output.send(ServiceEvent::Update(event)).await;
                    }

                    State::Active(conn, path)
                }
                Err(_) => State::Error,
            },
            State::NoBattery => State::NoBattery,
            State::Error => State::Error,
        }
    }
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
