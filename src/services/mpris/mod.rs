use super::{ReadOnlyService, Service, ServiceEvent};
use dbus::MprisPlayerProxy;
use iced::{
    Subscription,
    futures::{
        SinkExt, Stream, StreamExt,
        channel::mpsc::Sender,
        future::join_all,
        stream::{SelectAll, pending},
    },
    stream::channel,
};
use log::{debug, error, info};
use std::{any::TypeId, collections::HashMap, fmt::Display, ops::Deref, sync::Arc};
use zbus::{fdo::DBusProxy, zvariant::OwnedValue};

mod dbus;

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaybackStatus {
    #[default]
    Playing,
    Paused,
    Stopped,
}
impl From<String> for PlaybackStatus {
    fn from(playback_status: String) -> PlaybackStatus {
        match playback_status.as_str() {
            "Playing" => PlaybackStatus::Playing,
            "Paused" => PlaybackStatus::Paused,
            "Stopped" => PlaybackStatus::Stopped,
            _ => PlaybackStatus::Playing,
        }
    }
}

#[derive(Debug, Clone)]
pub struct MprisPlayerData {
    pub service: String,
    pub metadata: Option<MprisPlayerMetadata>,
    pub volume: Option<f64>,
    pub state: PlaybackStatus,
    proxy: MprisPlayerProxy<'static>,
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct MprisPlayerMetadata {
    pub artists: Option<Vec<String>>,
    pub title: Option<String>,
}

impl Display for MprisPlayerMetadata {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let t = match (self.artists.clone(), self.title.clone()) {
            (None, None) => String::new(),
            (None, Some(t)) => t,
            (Some(a), None) => a.join(", "),
            (Some(a), Some(t)) => format!("{} - {}", a.join(", "), t),
        };
        write!(f, "{t}")
    }
}

impl From<HashMap<String, OwnedValue>> for MprisPlayerMetadata {
    fn from(value: HashMap<String, OwnedValue>) -> Self {
        let artists = match value.get("xesam:artist") {
            Some(v) => v.clone().try_into().ok(),
            None => None,
        };
        let title = match value.get("xesam:title") {
            Some(v) => v.clone().try_into().ok(),
            None => None,
        };

        Self { artists, title }
    }
}

#[derive(Debug, Clone)]
pub struct MprisPlayerService {
    data: Vec<MprisPlayerData>,
    conn: zbus::Connection,
}

impl Deref for MprisPlayerService {
    type Target = Vec<MprisPlayerData>;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

enum State {
    Init,
    Active(zbus::Connection),
    Error,
}

#[derive(Debug, Clone)]
pub enum MprisPlayerEvent {
    Refresh(Vec<MprisPlayerData>),
    Metadata(String, Option<MprisPlayerMetadata>),
    Volume(String, Option<f64>),
    State(String, PlaybackStatus),
}

impl ReadOnlyService for MprisPlayerService {
    type UpdateEvent = MprisPlayerEvent;
    type Error = ();

    fn update(&mut self, event: Self::UpdateEvent) {
        match event {
            MprisPlayerEvent::Refresh(data) => self.data = data,
            MprisPlayerEvent::Metadata(service, metadata) => {
                let s = self.data.iter_mut().find(|d| d.service == service);
                if let Some(s) = s {
                    s.metadata = metadata;
                }
            }
            MprisPlayerEvent::Volume(service, volume) => {
                let s = self.data.iter_mut().find(|d| d.service == service);
                if let Some(s) = s {
                    s.volume = volume;
                }
            }
            MprisPlayerEvent::State(service, state) => {
                let s = self.data.iter_mut().find(|d| d.service == service);
                if let Some(s) = s {
                    s.state = state;
                }
            }
        }
    }

    fn subscribe() -> Subscription<ServiceEvent<Self>> {
        let id = TypeId::of::<Self>();

        Subscription::run_with_id(
            id,
            channel(10, async |mut output| {
                let mut state = State::Init;

                loop {
                    state = Self::start_listening(state, &mut output).await;
                }
            }),
        )
    }
}

const MPRIS_PLAYER_SERVICE_PREFIX: &str = "org.mpris.MediaPlayer2.";

#[derive(Debug)]
enum Event {
    NameOwner,
    Metadata(String, Option<MprisPlayerMetadata>),
    Volume(String, Option<f64>),
    State(String, PlaybackStatus),
}

impl MprisPlayerService {
    async fn initialize_data(conn: &zbus::Connection) -> anyhow::Result<Vec<MprisPlayerData>> {
        let dbus = DBusProxy::new(conn).await?;
        let names: Vec<String> = dbus
            .list_names()
            .await?
            .iter()
            .filter_map(|a| {
                if a.starts_with(MPRIS_PLAYER_SERVICE_PREFIX) {
                    Some(a.to_string())
                } else {
                    None
                }
            })
            .collect();

        debug!("Found MPRIS player services: {names:?}");

        Ok(Self::get_mpris_player_data(conn, &names).await)
    }

    async fn get_mpris_player_data(
        conn: &zbus::Connection,
        names: &[String],
    ) -> Vec<MprisPlayerData> {
        join_all(names.iter().map(|s| async {
            match MprisPlayerProxy::new(conn, s.to_string()).await {
                Ok(proxy) => {
                    let metadata = proxy
                        .metadata()
                        .await
                        .map_or(None, |m| Some(MprisPlayerMetadata::from(m)));

                    let volume = proxy.volume().await.map(|v| v * 100.0).ok();
                    let state = proxy
                        .playback_status()
                        .await
                        .map(PlaybackStatus::from)
                        .unwrap_or_default();

                    Some(MprisPlayerData {
                        service: s.to_string(),
                        metadata,
                        volume,
                        state,
                        proxy,
                    })
                }
                Err(_) => None,
            }
        }))
        .await
        .into_iter()
        .flatten()
        .collect()
    }

    async fn events(conn: &zbus::Connection) -> anyhow::Result<impl Stream<Item = Event> + use<>> {
        let dbus = DBusProxy::new(conn).await?;
        let data = Self::initialize_data(conn).await?;

        let mut combined = SelectAll::new();

        combined.push(
            dbus.receive_name_owner_changed()
                .await?
                .filter_map(|s| async move {
                    match s.args() {
                        Ok(a) => a
                            .name
                            .starts_with(MPRIS_PLAYER_SERVICE_PREFIX)
                            .then_some(Event::NameOwner),
                        Err(_) => None,
                    }
                })
                .boxed(),
        );

        for s in data.iter() {
            let cache = Arc::new(s.metadata.clone());

            combined.push(
                s.proxy
                    .receive_metadata_changed()
                    .await
                    .filter_map({
                        let cache = cache.clone();
                        let service = s.service.clone();

                        move |m| {
                            let cache = cache.clone();
                            let service = service.clone();

                            async move {
                                let new_metadata =
                                    m.get().await.map(MprisPlayerMetadata::from).ok();
                                if &new_metadata == cache.as_ref() {
                                    None
                                } else {
                                    debug!("Metadata changed: {new_metadata:?}");

                                    Some(Event::Metadata(service, new_metadata))
                                }
                            }
                        }
                    })
                    .boxed(),
            );
        }

        for s in data.iter() {
            let volume = s.volume;

            combined.push(
                s.proxy
                    .receive_volume_changed()
                    .await
                    .filter_map({
                        let service = s.service.clone();
                        move |v| {
                            let service = service.clone();
                            async move {
                                let new_volume = v.get().await.ok();
                                if volume == new_volume {
                                    None
                                } else {
                                    debug!("Volume changed: {new_volume:?}");

                                    Some(Event::Volume(service, new_volume))
                                }
                            }
                        }
                    })
                    .boxed(),
            );
        }

        for s in data.iter() {
            let state = s.state;

            combined.push(
                s.proxy
                    .receive_playback_status_changed()
                    .await
                    .filter_map({
                        let service = s.service.clone();
                        move |v| {
                            let service = service.clone();
                            async move {
                                let new_state =
                                    v.get().await.map(PlaybackStatus::from).unwrap_or_default();
                                if state == new_state {
                                    None
                                } else {
                                    debug!("PlaybackStatus changed: {new_state:?}");

                                    Some(Event::State(service, new_state))
                                }
                            }
                        }
                    })
                    .boxed(),
            );
        }

        Ok(combined)
    }

    async fn start_listening(state: State, output: &mut Sender<ServiceEvent<Self>>) -> State {
        match state {
            State::Init => match zbus::Connection::session().await {
                Ok(conn) => {
                    let data = Self::initialize_data(&conn).await;
                    match data {
                        Ok(data) => {
                            info!("MPRIS player service initialized");

                            let _ = output
                                .send(ServiceEvent::Init(MprisPlayerService {
                                    data,
                                    conn: conn.clone(),
                                }))
                                .await;

                            State::Active(conn)
                        }
                        Err(err) => {
                            error!("Failed to initialize MPRIS player service: {err}");

                            State::Error
                        }
                    }
                }
                Err(err) => {
                    error!("Failed to connect to system bus for MPRIS player: {err}");
                    State::Error
                }
            },
            State::Active(conn) => match Self::events(&conn).await {
                Ok(events) => {
                    let mut chunks = events.ready_chunks(10);

                    while let Some(chunk) = chunks.next().await {
                        debug!("MPRIS player service receive events: {chunk:?}");

                        let mut need_refresh = false;

                        for event in chunk {
                            match event {
                                Event::NameOwner => {
                                    debug!("MPRIS player service name owner changed");
                                    need_refresh = true;
                                }
                                Event::Metadata(service, metadata) => {
                                    debug!(
                                        "MPRIS player service {service} metadata changed: {metadata:?}"
                                    );
                                    let _ = output
                                        .send(ServiceEvent::Update(MprisPlayerEvent::Metadata(
                                            service, metadata,
                                        )))
                                        .await;
                                }
                                Event::Volume(service, volume) => {
                                    debug!(
                                        "MPRIS player service {service} volume changed: {volume:?}"
                                    );
                                    let _ = output
                                        .send(ServiceEvent::Update(MprisPlayerEvent::Volume(
                                            service, volume,
                                        )))
                                        .await;
                                }
                                Event::State(service, state) => {
                                    debug!(
                                        "MPRIS player service {service} playback status changed: {state:?}"
                                    );
                                    let _ = output
                                        .send(ServiceEvent::Update(MprisPlayerEvent::State(
                                            service, state,
                                        )))
                                        .await;
                                }
                            }
                        }

                        if need_refresh {
                            match Self::initialize_data(&conn).await {
                                Ok(data) => {
                                    debug!("Refreshing MPRIS player data");

                                    let _ = output
                                        .send(ServiceEvent::Update(MprisPlayerEvent::Refresh(data)))
                                        .await;
                                }
                                Err(err) => {
                                    error!("Failed to fetch MPRIS player data: {err}");
                                }
                            }

                            break;
                        }
                    }

                    State::Active(conn)
                }
                Err(err) => {
                    error!("Failed to listen for MPRIS player events: {err}");

                    State::Error
                }
            },
            State::Error => {
                let _ = pending::<u8>().next().await;

                State::Error
            }
        }
    }
}

#[derive(Debug)]
pub struct MprisPlayerCommand {
    pub service_name: String,
    pub command: PlayerCommand,
}

#[derive(Debug)]
pub enum PlayerCommand {
    Prev,
    PlayPause,
    Next,
    Volume(f64),
}

impl Service for MprisPlayerService {
    type Command = MprisPlayerCommand;

    fn command(&mut self, command: Self::Command) -> iced::Task<ServiceEvent<Self>> {
        {
            let names: Vec<String> = self.data.iter().map(|d| d.service.clone()).collect();
            let s = self.data.iter().find(|d| d.service == command.service_name);

            if let Some(s) = s {
                let mpris_player_proxy = s.proxy.clone();
                let conn = self.conn.clone();
                iced::Task::perform(
                    async move {
                        match command.command {
                            PlayerCommand::Prev => {
                                let _ = mpris_player_proxy
                                    .previous()
                                    .await
                                    .inspect_err(|e| error!("Previous command error: {e}"));
                            }
                            PlayerCommand::PlayPause => {
                                let _ = mpris_player_proxy
                                    .play_pause()
                                    .await
                                    .inspect_err(|e| error!("Play/pause command error: {e}"));
                            }
                            PlayerCommand::Next => {
                                let _ = mpris_player_proxy
                                    .next()
                                    .await
                                    .inspect_err(|e| error!("Next command error: {e}"));
                            }
                            PlayerCommand::Volume(v) => {
                                let _ = mpris_player_proxy
                                    .set_volume(v / 100.0)
                                    .await
                                    .inspect_err(|e| error!("Set volume command error: {e}"));
                            }
                        }
                        Self::get_mpris_player_data(&conn, &names).await
                    },
                    |data| ServiceEvent::Update(MprisPlayerEvent::Refresh(data)),
                )
            } else {
                iced::Task::none()
            }
        }
    }
}
