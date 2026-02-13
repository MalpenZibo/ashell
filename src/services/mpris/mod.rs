use super::{ReadOnlyService, Service, ServiceEvent};
use dbus::MprisPlayerProxy;
use iced::{
    Subscription,
    core::image::Bytes,
    futures::{
        FutureExt, SinkExt, Stream, StreamExt,
        channel::mpsc::Sender,
        future::{BoxFuture, join_all},
        select,
        stream::{AbortHandle, Abortable, Aborted, FuturesUnordered, SelectAll, pending},
    },
    stream::channel,
};
use log::{debug, error, info};
use std::{
    any::TypeId,
    collections::{HashMap, HashSet},
    fmt::Display,
};
use url::Url;
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
    pub album: Option<String>,
    pub art_url: Option<String>,
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
        let album = match value.get("xesam:album") {
            Some(v) => v.clone().try_into().ok(),
            None => None,
        };
        let art_url = match value.get("mpris:artUrl") {
            Some(v) => v.clone().try_into().ok(),
            None => None,
        };

        Self {
            artists,
            title,
            album,
            art_url,
        }
    }
}

#[derive(Debug, Clone)]
pub struct MprisPlayerService {
    data: Vec<MprisPlayerData>,
    conn: zbus::Connection,
    covers: HashMap<String, Bytes>,
}

impl MprisPlayerService {
    pub fn players(&self) -> &[MprisPlayerData] {
        &self.data
    }

    pub fn get_cover(&self, url: &str) -> Option<&Bytes> {
        self.covers.get(url)
    }
}

struct ActiveData {
    conn: zbus::Connection,
    fetched_covers: HashSet<String>,
    in_flight: HashMap<String, AbortHandle>,
    pending_downloads: FuturesUnordered<CoverDownloadFuture>,
}

impl ActiveData {
    fn new(conn: zbus::Connection) -> Self {
        Self {
            conn,
            fetched_covers: HashSet::new(),
            in_flight: HashMap::new(),
            pending_downloads: FuturesUnordered::new(),
        }
    }
}

enum State {
    Init,
    Active(ActiveData),
    Error,
}

#[derive(Debug, Clone)]
pub enum Event {
    MetadataChanged(Vec<MprisPlayerData>),
    CoverFetched(String, Bytes),
}

impl ReadOnlyService for MprisPlayerService {
    type UpdateEvent = Event;
    type Error = ();

    fn update(&mut self, event: Self::UpdateEvent) {
        match event {
            Event::MetadataChanged(data) => {
                self.data = data;
            }
            Event::CoverFetched(url, bytes) => {
                self.covers.insert(url, bytes);
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

type CoverDownloadFuture = BoxFuture<'static, Result<(String, anyhow::Result<Bytes>), Aborted>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DbusEvent {
    DataChanged,
    PlayersChanged,
}

impl MprisPlayerService {
    async fn initialize_data(conn: &zbus::Connection) -> anyhow::Result<Vec<MprisPlayerData>> {
        let dbus = DBusProxy::new(conn).await?;
        let names = Self::get_player_names(&dbus).await?;
        debug!("Found MPRIS player services: {names:?}");

        Ok(Self::get_mpris_player_data(conn, &names).await)
    }

    async fn get_player_names(dbus: &DBusProxy<'_>) -> anyhow::Result<Vec<String>> {
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
        Ok(names)
    }

    async fn create_proxies(
        conn: &zbus::Connection,
        names: &[String],
    ) -> Vec<(String, MprisPlayerProxy<'static>)> {
        let proxies: Vec<_> = join_all(names.iter().map(
            async |s| -> anyhow::Result<(String, MprisPlayerProxy<'static>)> {
                let proxy = MprisPlayerProxy::new(conn, s.to_string()).await?;
                Ok((s.to_string(), proxy))
            },
        ))
        .await
        .into_iter()
        .filter_map(Result::ok)
        .collect();
        proxies
    }

    async fn get_mpris_player_data(
        conn: &zbus::Connection,
        names: &[String],
    ) -> Vec<MprisPlayerData> {
        let proxies = Self::create_proxies(conn, names).await;

        join_all(proxies.into_iter().map(|(name, proxy)| async {
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
                service: name,
                metadata,
                volume,
                state,
                proxy,
            })
        }))
        .await
        .into_iter()
        .flatten()
        .collect()
    }

    async fn dbus_events(
        conn: &zbus::Connection,
    ) -> anyhow::Result<impl Stream<Item = DbusEvent> + use<>> {
        let dbus = DBusProxy::new(conn).await?;

        let mut combined = SelectAll::new();

        combined.push(
            dbus.receive_name_owner_changed()
                .await?
                .filter_map(|s| async move {
                    match s.args() {
                        Ok(a) => a
                            .name
                            .starts_with(MPRIS_PLAYER_SERVICE_PREFIX)
                            .then_some(DbusEvent::PlayersChanged),
                        Err(_) => None,
                    }
                })
                .boxed(),
        );

        let proxies = Self::create_proxies(conn, &Self::get_player_names(&dbus).await?).await;

        for (_, p) in proxies.iter() {
            combined.push(
                p.receive_metadata_changed()
                    .await
                    .filter_map({
                        move |m| async move {
                            let new_metadata = m.get().await.map(MprisPlayerMetadata::from).ok();
                            debug!("Metadata changed: {new_metadata:?}");

                            Some(DbusEvent::DataChanged)
                        }
                    })
                    .boxed(),
            );
        }

        for (_, p) in proxies.iter() {
            combined.push(
                p.receive_volume_changed()
                    .await
                    .filter_map(move |v| async move {
                        let new_volume = v.get().await.ok();
                        debug!("Volume changed: {new_volume:?}");

                        Some(DbusEvent::DataChanged)
                    })
                    .boxed(),
            );
        }

        for (_, p) in proxies.iter() {
            combined.push(
                p.receive_playback_status_changed()
                    .await
                    .filter_map(move |v| async move {
                        let new_state = v.get().await.map(PlaybackStatus::from).unwrap_or_default();
                        debug!("PlaybackStatus changed: {new_state:?}");

                        Some(DbusEvent::DataChanged)
                    })
                    .boxed(),
            );
        }

        Ok(combined)
    }

    async fn start_listening(state: State, output: &mut Sender<ServiceEvent<Self>>) -> State {
        match state {
            State::Init => Self::init(output).await,
            State::Active(data) => Self::active(output, data).await,
            State::Error => {
                let _ = pending::<u8>().next().await;

                State::Error
            }
        }
    }

    async fn init(output: &mut Sender<ServiceEvent<Self>>) -> State {
        match zbus::Connection::session().await {
            Ok(conn) => {
                info!("MPRIS player service initialized");
                let _ = output
                    .send(ServiceEvent::Init(MprisPlayerService {
                        data: Vec::new(),
                        conn: conn.clone(),
                        covers: HashMap::new(),
                    }))
                    .await;
                State::Active(ActiveData::new(conn))
            }
            Err(err) => {
                error!("Failed to connect to system bus for MPRIS player: {err}");
                State::Error
            }
        }
    }

    async fn active(output: &mut Sender<ServiceEvent<Self>>, mut state_data: ActiveData) -> State {
        Self::update_data(output, &mut state_data).await;

        match Self::dbus_events(&state_data.conn).await {
            Ok(dbus_events) => {
                let mut chunks = dbus_events.ready_chunks(10);

                loop {
                    select! {
                        chunk = chunks.next().fuse() => {
                            let Some(chunk) = chunk else {
                                // D-Bus event stream ended, restart listening
                                // TODO: Should we go to Error state instead?
                                break;
                            };
                            debug!("MPRIS player service receive events: {chunk:?}");
                            if chunk.contains(&DbusEvent::PlayersChanged) {
                                // We have to recreate the D-Bus subscriptions with the new players
                                break;
                            }
                            Self::update_data(output, &mut state_data).await;
                        }
                        result = state_data.pending_downloads.select_next_some() => {
                            match result {
                                Err(_) => {
                                    // Aborted fetch, ignore
                                }
                                Ok((url, res)) => {
                                    state_data.in_flight.remove::<String>(&url);

                                    match res {
                                        Ok(bytes) => {
                                            state_data.fetched_covers.insert(url.clone());
                                            let _ = output.send(ServiceEvent::Update(
                                                Event::CoverFetched(url.clone(), bytes)
                                            )).await;
                                        }
                                        Err(err) => {
                                            error!("Failed to fetch cover art from {url}: {err}");
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                State::Active(state_data)
            }
            Err(err) => {
                error!("Failed to listen for MPRIS player events: {err}");

                State::Error
            }
        }
    }

    async fn update_data(output: &mut Sender<ServiceEvent<Self>>, state_data: &mut ActiveData) {
        match Self::initialize_data(&state_data.conn).await {
            Ok(data) => {
                debug!("Refreshing MPRIS player data for {} players", data.len());
                Self::check_cover_update(&data, state_data);
                let _ = output
                    .send(ServiceEvent::Update(Event::MetadataChanged(data)))
                    .await;
            }
            Err(err) => {
                error!("Failed to fetch MPRIS player data: {err}");
            }
        }
    }

    fn check_cover_update(data: &[MprisPlayerData], state_data: &mut ActiveData) {
        let desired_urls: HashSet<String> = data
            .iter()
            .filter_map(|p| p.metadata.as_ref()?.art_url.clone())
            .filter(|url| !state_data.fetched_covers.contains(url))
            .collect();

        for (_, handle) in state_data
            .in_flight
            .extract_if(|url, _| !desired_urls.contains(url))
        {
            handle.abort();
        }

        let in_flight_urls = state_data.in_flight.keys().cloned().collect::<HashSet<_>>();
        for url in desired_urls
            .iter()
            .filter(|&url| !in_flight_urls.contains(url))
        {
            let url = url.clone();
            let (handle, reg) = AbortHandle::new_pair();
            state_data.in_flight.insert(url.clone(), handle);
            state_data.pending_downloads.push(Box::pin(Abortable::new(
                async move {
                    let res = Self::fetch_cover(&url).await;
                    (url, res)
                },
                reg,
            )));
        }
    }

    async fn fetch_cover(url: &str) -> anyhow::Result<Bytes> {
        let url = Url::parse(url)?;
        match url.scheme() {
            "http" | "https" => {
                let response = reqwest::get(url).await?;
                Ok(response.bytes().await?)
            }
            "file" => {
                let path = url
                    .to_file_path()
                    .map_err(|_| anyhow::anyhow!("Invalid file URL {}", url))?;
                Ok(tokio::fs::read(path).await?.into())
            }
            _ => anyhow::bail!("Unsupported URL scheme: {}", url.scheme()),
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
                        Event::MetadataChanged(Self::get_mpris_player_data(&conn, &names).await)
                    },
                    ServiceEvent::Update,
                )
            } else {
                iced::Task::none()
            }
        }
    }
}
