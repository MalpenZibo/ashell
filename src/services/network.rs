use super::{ReadOnlyService, Service, ServiceEvent};
use crate::{components::icons::StaticIcon, utils::IndicatorState};
use ashell_services::network::{Band, Network, NetworkData};
use iced::{
    Subscription, Task,
    futures::{SinkExt, StreamExt, stream},
    stream::channel,
};
use log::{error, info, warn};
use std::{any::TypeId, ops::Deref};

pub use ashell_services::network::{
    AccessPointData, ActiveConnectionInfo, ConnectivityState, KnownConnection, NetworkCommand,
    PasswordReply, PasswordRequest, PasswordRequestId, PskOutcome, Redacted, Vpn,
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
    password_requests: Vec<PasswordRequest>,
}

#[derive(Debug, Clone)]
pub enum NetworkEvent {
    /// `None` when neither NetworkManager nor iwd runs anymore.
    Data(Option<NetworkData>),
    PasswordRequests(Vec<PasswordRequest>),
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

    /// The password request to show: the oldest pending one.
    pub fn password_request(&self) -> Option<&PasswordRequest> {
        self.password_requests.first()
    }

    pub fn is_password_pending(&self, id: PasswordRequestId) -> bool {
        self.password_requests
            .iter()
            .any(|request| request.id == id)
    }

    pub fn reply_password(&mut self, id: PasswordRequestId, reply: PasswordReply) {
        self.password_requests.retain(|request| request.id != id);
        self.handle.reply_password(id, reply);
    }

    /// Leaves every pending request to the other agents.
    pub fn decline_passwords(&mut self) {
        for request in self.password_requests.drain(..) {
            self.handle
                .reply_password(request.id, PasswordReply::Decline);
        }
    }
}

impl ReadOnlyService for NetworkService {
    type UpdateEvent = NetworkEvent;
    type Error = ();

    fn update(&mut self, event: Self::UpdateEvent) {
        match event {
            NetworkEvent::Data(Some(data)) => self.data = data,
            NetworkEvent::Data(None) => {}
            NetworkEvent::PasswordRequests(requests) => self.password_requests = requests,
        }
    }

    fn subscribe() -> Subscription<ServiceEvent<Self>> {
        Self::subscription(false)
    }
}

impl NetworkService {
    /// The network state, and the password requests when `password_requests`:
    /// listening to them makes NetworkManager and iwd wait for ashell to
    /// answer, so only a UI that shows them may.
    pub fn subscription(password_requests: bool) -> Subscription<ServiceEvent<Self>> {
        Subscription::run_with(
            (TypeId::of::<Self>(), password_requests),
            |&(_, password_requests)| {
                channel(100, async move |mut output| {
                    let handle = match Network::connect().await {
                        Ok(handle) => handle,
                        Err(err) => {
                            error!("Failed to connect to system bus: {err}");
                            return;
                        }
                    };

                    let mut present = false;
                    let mut pending = Vec::new();
                    let updates = handle.updates().map(NetworkEvent::Data);
                    let mut events = if password_requests {
                        let requests = handle
                            .password_requests()
                            .map(NetworkEvent::PasswordRequests);
                        stream::select(updates, requests).boxed()
                    } else {
                        updates.boxed()
                    };
                    while let Some(event) = events.next().await {
                        let event = match event {
                            NetworkEvent::Data(Some(data)) if !present => {
                                info!("Network service initialized");
                                present = true;
                                ServiceEvent::Init(NetworkService {
                                    handle: handle.clone(),
                                    data,
                                    password_requests: pending.clone(),
                                })
                            }
                            NetworkEvent::Data(None) if !present => continue,
                            NetworkEvent::Data(data) => {
                                present = data.is_some();
                                ServiceEvent::Update(NetworkEvent::Data(data))
                            }
                            NetworkEvent::PasswordRequests(requests) => {
                                pending.clone_from(&requests);
                                if !present {
                                    continue;
                                }
                                ServiceEvent::Update(NetworkEvent::PasswordRequests(requests))
                            }
                        };
                        let _ = output.send(event).await;
                    }
                })
            },
        )
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
