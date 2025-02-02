use super::{ReadOnlyService, Service, ServiceEvent};
use dbus::MprisPlayerProxy;
use iced::{
    futures::{
        channel::mpsc::Sender,
        future::join_all,
        stream::{pending, SelectAll},
        SinkExt, Stream, StreamExt,
    },
    stream::channel,
    Subscription,
};
use log::{error, info};
use std::{any::TypeId, collections::HashMap, ops::Deref};
use zbus::{fdo::DBusProxy, zvariant::OwnedValue};

mod dbus;

#[derive(Debug, Clone)]
pub struct MprisPlayerData {
    pub service: String,
    pub metadata: Option<MprisPlayerMetadata>,
    pub volume: Option<f64>,
    proxy: MprisPlayerProxy<'static>,
}

#[derive(Debug, Clone)]
pub struct MprisPlayerMetadata {
    pub artists: Option<Vec<String>>,
    pub title: Option<String>,
}

impl From<HashMap<String, OwnedValue>> for MprisPlayerMetadata {
    fn from(value: HashMap<String, OwnedValue>) -> Self {
        let artists = match value.get("xesam:artist") {
            Some(v) => match v.clone().try_into() {
                Ok(v) => Some(v),
                Err(_) => None,
            },
            None => None,
        };
        let title = match value.get("xesam:title") {
            Some(v) => match v.clone().try_into() {
                Ok(v) => Some(v),
                Err(_) => None,
            },
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
    Active(zbus::Connection, Vec<String>),
    Error,
}

impl ReadOnlyService for MprisPlayerService {
    type UpdateEvent = Vec<MprisPlayerData>;
    type Error = ();

    fn update(&mut self, event: Self::UpdateEvent) {
        self.data = event;
    }

    fn subscribe() -> Subscription<ServiceEvent<Self>> {
        let id = TypeId::of::<Self>();

        Subscription::run_with_id(
            id,
            channel(10, |mut output| async move {
                let mut state = State::Init;

                loop {
                    state = MprisPlayerService::start_listening(state, &mut output).await;
                }
            }),
        )
    }
}

const MPRIS_PLAYER_SERVICE_PREFIX: &str = "org.mpris.MediaPlayer2.";

enum Event {
    NameOwner,
    Metadata,
    Volume,
}

impl MprisPlayerService {
    async fn initialize_data(
        conn: &zbus::Connection,
    ) -> anyhow::Result<(Vec<String>, Vec<MprisPlayerData>)> {
        let dbus = DBusProxy::new(&conn).await?;
        let names: Vec<String> = dbus
            .list_names()
            .await?
            .iter()
            .filter_map(|a| {
                a.starts_with(MPRIS_PLAYER_SERVICE_PREFIX)
                    .then(|| a.to_string())
            })
            .collect();
        Ok((
            names.clone(),
            MprisPlayerService::get_mpris_player_data(conn, &names).await,
        ))
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
                    Some(MprisPlayerData {
                        service: s.to_string(),
                        metadata,
                        volume,
                        proxy,
                    })
                }
                Err(_) => None,
            }
        }))
        .await
        .iter()
        .filter_map(|d| d.clone())
        .collect()
    }

    async fn events(conn: &zbus::Connection) -> anyhow::Result<impl Stream<Item = Event>> {
        let dbus = DBusProxy::new(conn).await?;
        let services = join_all(
            dbus.list_names()
                .await?
                .iter()
                .map(|n| async move { MprisPlayerProxy::new(conn, n.clone()).await.unwrap() }),
        )
        .await;
        let mut combined = SelectAll::new();
        combined.push(
            dbus.receive_name_owner_changed()
                .await?
                .filter_map(|s| {
                    iced::futures::future::ready(match s.args() {
                        Ok(a) => a
                            .name
                            .starts_with(MPRIS_PLAYER_SERVICE_PREFIX)
                            .then_some(Event::NameOwner),
                        Err(_) => None,
                    })
                })
                .boxed(),
        );
        for s in services.iter() {
            combined.push(
                s.receive_metadata_changed()
                    .await
                    .map(|_| Event::Metadata)
                    .boxed(),
            );
        }
        for s in services.iter() {
            combined.push(
                s.receive_volume_changed()
                    .await
                    .map(|_| Event::Volume)
                    .boxed(),
            );
        }
        Ok(combined)
    }

    async fn start_listening(state: State, output: &mut Sender<ServiceEvent<Self>>) -> State {
        match state {
            State::Init => match zbus::Connection::session().await {
                Ok(conn) => {
                    let data = MprisPlayerService::initialize_data(&conn).await;
                    match data {
                        Ok((names, data)) => {
                            info!("MPRIS player service initialized");

                            let _ = output
                                .send(ServiceEvent::Init(MprisPlayerService {
                                    data,
                                    conn: conn.clone(),
                                }))
                                .await;

                            State::Active(conn, names)
                        }
                        Err(err) => {
                            error!("Failed to initialize MPRIS player service: {}", err);

                            State::Error
                        }
                    }
                }
                Err(err) => {
                    error!("Failed to connect to system bus for MPRIS player: {}", err);
                    State::Error
                }
            },
            State::Active(conn, names) => match MprisPlayerService::events(&conn).await {
                Ok(mut events) => {
                    let mut names = names;
                    while let Some(e) = events.next().await {
                        let data = match e {
                            Event::NameOwner => MprisPlayerService::initialize_data(&conn).await,
                            _ => Ok((
                                names.clone(),
                                MprisPlayerService::get_mpris_player_data(&conn, &names).await,
                            )),
                        };
                        match data {
                            Ok(data) => {
                                info!("MPRIS player service new data");
                                names = data.0;
                                let _ = output.send(ServiceEvent::Update(data.1)).await;
                            }
                            Err(err) => {
                                error!("Failed to fetch MPRIS player data: {}", err);
                            }
                        }
                    }

                    State::Active(conn, names)
                }
                Err(err) => {
                    error!("Failed to listen for MPRIS player events: {}", err);

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
    pub service: String,
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
            let s = self.data.iter().find(|d| d.service == command.service);
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
                                    .inspect_err(|e| error!("Previous command error: {}", e));
                            }
                            PlayerCommand::PlayPause => {
                                let _ = mpris_player_proxy
                                    .play_pause()
                                    .await
                                    .inspect_err(|e| error!("Play/pause command error: {}", e));
                            }
                            PlayerCommand::Next => {
                                let _ = mpris_player_proxy
                                    .next()
                                    .await
                                    .inspect_err(|e| error!("Next command error: {}", e));
                            }
                            PlayerCommand::Volume(v) => {
                                let _ = mpris_player_proxy
                                    .set_volume(v / 100.0)
                                    .await
                                    .inspect_err(|e| error!("Set volume command error: {}", e));
                            }
                        }
                        MprisPlayerService::get_mpris_player_data(&conn, &names).await
                    },
                    |data| ServiceEvent::Update(data),
                )
            } else {
                iced::Task::none()
            }
        }
    }
}
