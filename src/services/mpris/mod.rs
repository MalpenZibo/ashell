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
    sync::Arc,
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
    pub covers: HashMap<String, Bytes>,
}

impl MprisPlayerService {
    pub fn players(&self) -> &Vec<MprisPlayerData> {
        &self.data
    }

    pub fn get_cover(&self, url: &str) -> Option<&Bytes> {
        self.covers.get(url)
    }
}

enum State {
    Init,
    Active(zbus::Connection, HashSet<String>),
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

    async fn dbus_events(
        conn: &zbus::Connection,
    ) -> anyhow::Result<impl Stream<Item = ()> + use<>> {
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
                            .then_some(()),
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

                        move |m| {
                            let cache = cache.clone();

                            async move {
                                let new_metadata =
                                    m.get().await.map(MprisPlayerMetadata::from).ok();
                                if &new_metadata == cache.as_ref() {
                                    None
                                } else {
                                    debug!("Metadata changed: {new_metadata:?}");

                                    Some(())
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
                    .filter_map(move |v| async move {
                        let new_volume = v.get().await.ok();
                        if volume == new_volume {
                            None
                        } else {
                            debug!("Volume changed: {new_volume:?}");

                            Some(())
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
                    .filter_map(move |v| async move {
                        let new_state = v.get().await.map(PlaybackStatus::from).unwrap_or_default();
                        if state == new_state {
                            None
                        } else {
                            debug!("PlaybackStatus changed: {new_state:?}");

                            Some(())
                        }
                    })
                    .boxed(),
            );
        }

        Ok(combined)
    }

    async fn start_listening(state: State, output: &mut Sender<ServiceEvent<Self>>) -> State {
        match state {
            State::Init => Self::init(output).await,
            State::Active(conn, fetched_covers) => Self::active(conn, output, fetched_covers).await,
            State::Error => {
                let _ = pending::<u8>().next().await;

                State::Error
            }
        }
    }

    async fn init(output: &mut Sender<ServiceEvent<Self>>) -> State {
        match zbus::Connection::session().await {
            Ok(conn) => {
                let data = Self::initialize_data(&conn).await;
                match data {
                    Ok(data) => {
                        info!("MPRIS player service initialized");

                        let _ = output
                            .send(ServiceEvent::Init(MprisPlayerService {
                                data,
                                conn: conn.clone(),
                                covers: HashMap::new(),
                            }))
                            .await;

                        State::Active(conn, HashSet::new())
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
        }
    }

    async fn active(
        conn: zbus::Connection,
        output: &mut Sender<ServiceEvent<Self>>,
        mut fetched_covers: HashSet<String>,
    ) -> State {
        match Self::dbus_events(&conn).await {
            Ok(dbus_events) => {
                let mut chunks = dbus_events.ready_chunks(10);

                let mut pending_downloads = FuturesUnordered::new();
                let mut in_flight: HashMap<String, AbortHandle> = HashMap::new();

                loop {
                    select! {
                        chunk = chunks.next().fuse() => {
                            let Some(chunk) = chunk else { break; };
                            debug!("MPRIS player service receive events: {chunk:?}");
                            match Self::initialize_data(&conn).await {
                                Ok(data) => {
                                    debug!("Refreshing MPRIS player data");

                                    Self::check_cover_update(&data, &fetched_covers, &mut in_flight, &mut pending_downloads);

                                    let _ = output.send(ServiceEvent::Update(Event::MetadataChanged(data))).await;
                                }
                                Err(err) => {
                                    error!("Failed to fetch MPRIS player data: {err}");
                                }
                            }
                        }
                        result = pending_downloads.select_next_some() => {
                            match result {
                                Err(_) => {
                                    // Aborted fetch, ignore
                                }
                                Ok((url, res)) => {
                                    in_flight.remove::<String>(&url);

                                    match res {
                                        Ok(bytes) => {
                                            fetched_covers.insert(url.clone());
                                            let _ = output.send(ServiceEvent::Update(Event::CoverFetched(url.clone(), bytes))).await;
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
                log::warn!("Exited event loop?");

                State::Active(conn, fetched_covers)
            }
            Err(err) => {
                error!("Failed to listen for MPRIS player events: {err}");

                State::Error
            }
        }
    }

    fn check_cover_update(
        data: &[MprisPlayerData],
        already_fetched: &HashSet<String>,
        in_flight: &mut HashMap<String, AbortHandle>,
        pending_downloads: &mut FuturesUnordered<CoverDownloadFuture>,
    ) {
        let desired_urls: HashSet<String> = data
            .iter()
            .filter_map(|p| p.metadata.as_ref()?.art_url.clone())
            .filter(|url| !already_fetched.contains(url))
            .collect();

        for (_, handle) in in_flight.extract_if(|url, _| !desired_urls.contains(url)) {
            handle.abort();
        }

        let in_flight_urls = in_flight.keys().cloned().collect::<HashSet<_>>();
        for url in desired_urls
            .iter()
            .filter(|&url| !in_flight_urls.contains(url))
        {
            let url = url.clone();
            let (handle, reg) = AbortHandle::new_pair();
            in_flight.insert(url.clone(), handle);
            pending_downloads.push(Box::pin(Abortable::new(
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
