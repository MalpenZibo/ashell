use super::{ReadOnlyService, Service, ServiceEvent};
use crate::{components::icons::StaticIcon, utils::IndicatorState};
use ashell_services::network::{Band, Network, NetworkData};
use iced::{
    Subscription, Task,
    futures::{SinkExt, StreamExt},
    stream::channel,
};
use log::{error, info, warn};
use std::{any::TypeId, ops::Deref, pin::pin};

pub use ashell_services::network::{
    AccessPointData, ActiveConnectionInfo, ConnectivityState, KnownConnection, NetworkCommand,
    PskOutcome, Redacted, Vpn,
};

static WIFI_SIGNAL_ICONS: [StaticIcon; 6] = [
    StaticIcon::Wifi0,
    StaticIcon::Wifi1,
    StaticIcon::Wifi2,
    StaticIcon::Wifi3,
    StaticIcon::Wifi4,
    StaticIcon::Wifi5,
];

static WIFI_LOCK_SIGNAL_ICONS: [StaticIcon; 5] = [
    StaticIcon::WifiLock1,
    StaticIcon::WifiLock2,
    StaticIcon::WifiLock3,
    StaticIcon::WifiLock4,
    StaticIcon::WifiLock5,
];

/// The label of the band `frequency` (in MHz) is in.
pub fn frequency_band(frequency: u32) -> Option<&'static str> {
    Band::from_frequency(frequency).map(|band| match band {
        Band::TwoPointFourGhz => "2.4 GHz",
        Band::FiveGhz => "5 GHz",
        Band::SixGhz => "6 GHz",
        Band::SixtyGhz => "60 GHz",
    })
}

pub fn wifi_icon(signal: u8) -> StaticIcon {
    let clamped_signal = signal.min(100);
    WIFI_SIGNAL_ICONS[1 + f32::round(clamped_signal as f32 / 100. * 4.) as usize]
}

pub fn wifi_lock_icon(signal: u8) -> StaticIcon {
    let clamped_signal = signal.min(100);
    WIFI_LOCK_SIGNAL_ICONS[f32::round(clamped_signal as f32 / 100. * 4.) as usize]
}

pub trait ActiveConnectionInfoExt {
    fn get_icon(&self) -> StaticIcon;

    fn get_indicator_state(&self) -> IndicatorState;
}

impl ActiveConnectionInfoExt for ActiveConnectionInfo {
    fn get_icon(&self) -> StaticIcon {
        match self {
            Self::WiFi { strength, .. } => wifi_icon(*strength),
            Self::Wired { .. } => StaticIcon::Ethernet,
            Self::Vpn { .. } => StaticIcon::Vpn,
        }
    }

    fn get_indicator_state(&self) -> IndicatorState {
        match self {
            Self::WiFi {
                strength: 0 | 1, ..
            } => IndicatorState::Warning,
            _ => IndicatorState::Normal,
        }
    }
}

#[derive(Debug, Clone)]
pub struct NetworkService {
    handle: Network,
    data: NetworkData,
}

impl Deref for NetworkService {
    type Target = NetworkData;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl NetworkService {
    /// The passphrase of `ssid`, the Wi-Fi network in use, to share it.
    /// `None` when it couldn't be read.
    pub fn read_psk(&self, ssid: String) -> Task<Option<PskOutcome>> {
        let handle = self.handle.clone();

        Task::future(async move {
            handle
                .read_psk(&ssid)
                .await
                .inspect_err(|err| warn!("read_psk failed for {ssid}: {err}"))
                .ok()
        })
    }
}

impl ReadOnlyService for NetworkService {
    /// `None` when neither NetworkManager nor iwd runs anymore.
    type UpdateEvent = Option<NetworkData>;
    type Error = ();

    fn update(&mut self, event: Self::UpdateEvent) {
        if let Some(data) = event {
            self.data = data;
        }
    }

    fn subscribe() -> Subscription<ServiceEvent<Self>> {
        Subscription::run_with(TypeId::of::<Self>(), |_| {
            channel(100, async |mut output| {
                let handle = match Network::connect().await {
                    Ok(handle) => handle,
                    Err(err) => {
                        error!("Failed to connect to system bus: {err}");
                        return;
                    }
                };

                let mut present = false;
                let mut updates = pin!(handle.updates());
                while let Some(data) = updates.next().await {
                    let available = data.is_some();
                    let event = match data {
                        Some(data) if !present => {
                            info!("Network service initialized");
                            ServiceEvent::Init(NetworkService {
                                handle: handle.clone(),
                                data,
                            })
                        }
                        None if !present => continue,
                        data => ServiceEvent::Update(data),
                    };
                    present = available;
                    let _ = output.send(event).await;
                }
            })
        })
    }
}

impl Service for NetworkService {
    type Command = NetworkCommand;

    fn command(&mut self, command: Self::Command) -> Task<ServiceEvent<Self>> {
        let handle = self.handle.clone();

        Task::future(async move {
            if let Err(err) = handle.execute(command).await {
                warn!("Network command failed: {err}");
            }
        })
        .discard()
    }
}
