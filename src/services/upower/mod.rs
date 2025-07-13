use super::{ReadOnlyService, Service, ServiceEvent};
use crate::{components::icons::Icons, utils::IndicatorState};
use dbus::{Battery, PowerProfilesProxy, UPowerDbus};
use iced::{
    Subscription,
    futures::{
        SinkExt, Stream, StreamExt,
        channel::mpsc::Sender,
        stream::{once, pending, select_all},
        stream_select,
    },
    stream::channel,
};
use log::{error, warn};
use std::{any::TypeId, time::Duration};
use zbus::zvariant::ObjectPath;

mod dbus;

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
pub enum UPowerEvent {
    UpdateBattery(BatteryData),
    NoBattery,
    UpdatePowerProfile(PowerProfile),
}

#[derive(Copy, Clone, Debug)]
pub enum BatteryStatus {
    Charging(Duration),
    Discharging(Duration),
    Full,
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum PowerProfile {
    Balanced,
    Performance,
    PowerSaver,
    #[default]
    Unknown,
}

impl From<String> for PowerProfile {
    fn from(power_profile: String) -> PowerProfile {
        match power_profile.as_str() {
            "balanced" => PowerProfile::Balanced,
            "performance" => PowerProfile::Performance,
            "power-saver" => PowerProfile::PowerSaver,
            _ => PowerProfile::Unknown,
        }
    }
}

impl From<PowerProfile> for Icons {
    fn from(profile: PowerProfile) -> Self {
        match profile {
            PowerProfile::Balanced => Icons::Balanced,
            PowerProfile::Performance => Icons::Performance,
            PowerProfile::PowerSaver => Icons::PowerSaver,
            PowerProfile::Unknown => Icons::None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct UPowerService {
    pub battery: Option<BatteryData>,
    pub power_profile: PowerProfile,
    conn: zbus::Connection,
}

enum State {
    Init,
    Active(zbus::Connection, Option<Vec<ObjectPath<'static>>>),
    Error,
}

impl ReadOnlyService for UPowerService {
    type UpdateEvent = UPowerEvent;
    type Error = ();

    fn update(&mut self, event: Self::UpdateEvent) {
        match event {
            UPowerEvent::UpdateBattery(data) => {
                self.battery.replace(data);
            }
            UPowerEvent::NoBattery => {
                self.battery = None;
            }
            UPowerEvent::UpdatePowerProfile(profile) => {
                self.power_profile = profile;
            }
        }
    }

    fn subscribe() -> Subscription<ServiceEvent<Self>> {
        let id = TypeId::of::<Self>();

        Subscription::run_with_id(
            id,
            channel(100, async |mut output| {
                let mut state = State::Init;

                loop {
                    state = UPowerService::start_listening(state, &mut output).await;
                }
            }),
        )
    }
}

impl UPowerService {
    async fn initialize_data(
        conn: &zbus::Connection,
    ) -> anyhow::Result<(
        Option<(BatteryData, Vec<ObjectPath<'static>>)>,
        PowerProfile,
    )> {
        let battery = UPowerService::initialize_battery_data(conn).await?;
        let power_profile = UPowerService::initialize_power_profile_data(conn).await;

        match (battery, power_profile) {
            (Some(battery), Ok(power_profile)) => Ok((
                Some((battery.0, battery.1.get_devices_path())),
                power_profile,
            )),
            (Some(battery), Err(err)) => {
                warn!("Failed to get power profile: {err}");

                Ok((
                    Some((battery.0, battery.1.get_devices_path())),
                    PowerProfile::Unknown,
                ))
            }
            (None, Ok(power_profile)) => Ok((None, power_profile)),
            (None, Err(err)) => {
                warn!("Failed to get power profile: {err}");

                Ok((None, PowerProfile::Unknown))
            }
        }
    }

    async fn initialize_power_profile_data(
        conn: &zbus::Connection,
    ) -> anyhow::Result<PowerProfile> {
        let powerprofiles = PowerProfilesProxy::new(conn).await?;

        let profile = powerprofiles
            .active_profile()
            .await
            .map(PowerProfile::from)?;

        Ok(profile)
    }

    async fn initialize_battery_data(
        conn: &zbus::Connection,
    ) -> anyhow::Result<Option<(BatteryData, Battery)>> {
        let upower = UPowerDbus::new(conn).await?;
        let battery = upower.get_battery_devices().await?;

        match battery {
            Some(battery) => {
                let state = battery.state().await;
                let state = match state {
                    1 => BatteryStatus::Charging(Duration::from_secs(
                        battery.time_to_full().await as u64,
                    )),
                    2 => BatteryStatus::Discharging(Duration::from_secs(
                        battery.time_to_empty().await as u64,
                    )),
                    4 => BatteryStatus::Full,
                    _ => BatteryStatus::Discharging(Duration::from_secs(0)),
                };
                let percentage = battery.percentage().await as i64;

                Ok(Some((
                    BatteryData {
                        capacity: percentage,
                        status: state,
                    },
                    battery,
                )))
            }
            _ => Ok(None),
        }
    }

    async fn events(
        conn: &zbus::Connection,
        battery_devices: &Option<Vec<ObjectPath<'static>>>,
    ) -> anyhow::Result<impl Stream<Item = UPowerEvent> + use<>> {
        let battery_event = if let Some(battery_devices) = battery_devices {
            let upower = UPowerDbus::new(conn).await?;

            let mut events = Vec::new();

            for device_path in battery_devices {
                let device = upower.get_device(device_path).await?;

                events.push(
                    stream_select!(
                        device.receive_state_changed().await.map(|_| ()),
                        device.receive_percentage_changed().await.map(|_| ()),
                        device.receive_time_to_full_changed().await.map(|_| ()),
                        device.receive_time_to_empty_changed().await.map(|_| ()),
                    )
                    .filter_map({
                        let conn = conn.clone();
                        move |_| {
                            let conn = conn.clone();
                            async move {
                                if let Some((data, _)) =
                                    Self::initialize_battery_data(&conn).await.ok().flatten()
                                {
                                    Some(UPowerEvent::UpdateBattery(data))
                                } else {
                                    None
                                }
                            }
                        }
                    })
                    .boxed(),
                );
            }

            select_all(events).boxed()
        } else {
            once(async {}).map(|_| UPowerEvent::NoBattery).boxed()
        };

        let powerprofiles = PowerProfilesProxy::new(conn).await?;
        let power_profile_event =
            powerprofiles
                .receive_active_profile_changed()
                .await
                .map(move |_| {
                    UPowerEvent::UpdatePowerProfile(
                        powerprofiles
                            .cached_active_profile()
                            .map(|d| d.map(PowerProfile::from).unwrap_or_default())
                            .unwrap_or_default(),
                    )
                });

        Ok(stream_select!(battery_event, power_profile_event))
    }

    async fn start_listening(state: State, output: &mut Sender<ServiceEvent<Self>>) -> State {
        match state {
            State::Init => match zbus::Connection::system().await {
                Ok(conn) => {
                    let (battery, battery_path, power_profile) =
                        match UPowerService::initialize_data(&conn).await {
                            Ok((Some((battery_data, battery_path)), power_profile)) => {
                                (Some(battery_data), Some(battery_path), power_profile)
                            }
                            Ok((None, power_profile)) => (None, None, power_profile),
                            Err(err) => {
                                error!("Failed to initialize upower service: {err}");

                                return State::Error;
                            }
                        };

                    let service = UPowerService {
                        battery,
                        power_profile,
                        conn: conn.clone(),
                    };
                    let _ = output.send(ServiceEvent::Init(service)).await;

                    State::Active(conn, battery_path)
                }
                Err(err) => {
                    error!("Failed to connect to system bus for upower: {err}");
                    State::Error
                }
            },
            State::Active(conn, battery_devices) => {
                match UPowerService::events(&conn, &battery_devices).await {
                    Ok(mut events) => {
                        while let Some(event) = events.next().await {
                            let _ = output.send(ServiceEvent::Update(event)).await;
                        }

                        State::Active(conn, battery_devices)
                    }
                    Err(err) => {
                        error!("Failed to listen for upower events: {err}");

                        State::Error
                    }
                }
            }
            State::Error => {
                let _ = pending::<u8>().next().await;

                State::Error
            }
        }
    }
}

pub enum PowerProfileCommand {
    Toggle,
}

impl Service for UPowerService {
    type Command = PowerProfileCommand;

    fn command(&mut self, command: Self::Command) -> iced::Task<ServiceEvent<Self>> {
        iced::Task::perform(
            {
                let conn = self.conn.clone();
                let power_profile = self.power_profile;
                async move {
                    let powerprofiles = PowerProfilesProxy::new(&conn)
                        .await
                        .expect("Failed to create PowerProfilesProxy");

                    match command {
                        PowerProfileCommand::Toggle => {
                            let current_profile = power_profile;
                            match current_profile {
                                PowerProfile::Balanced => {
                                    let _ = powerprofiles.set_active_profile("performance").await;

                                    PowerProfile::Performance
                                }
                                PowerProfile::Performance => {
                                    let _ = powerprofiles.set_active_profile("power-saver").await;

                                    PowerProfile::PowerSaver
                                }
                                PowerProfile::PowerSaver => {
                                    let _ = powerprofiles.set_active_profile("balanced").await;

                                    PowerProfile::Balanced
                                }
                                PowerProfile::Unknown => PowerProfile::Unknown,
                            }
                        }
                    }
                }
            },
            |power_profile| ServiceEvent::Update(UPowerEvent::UpdatePowerProfile(power_profile)),
        )
    }
}
