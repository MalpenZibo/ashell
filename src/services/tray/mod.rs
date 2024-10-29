use std::any::TypeId;

use dbus::{
    StatusNotifierItemRegistered, StatusNotifierWatcher, StatusNotifierWatcherProxy,
    StatusNotifierWatcherSignals,
};
use iced::{
    futures::{channel::mpsc::Sender, stream::pending, FutureExt, SinkExt, Stream, StreamExt},
    subscription::channel,
};
use log::{debug, error, info, warn};

use super::{ReadOnlyService, ServiceEvent};

mod dbus;

#[derive(Debug, Clone, Default)]
pub struct TrayData {
    pub current: u32,
    pub max: u32,
}

#[derive(Debug, Clone)]
pub struct TrayService {
    data: TrayData,
    device_name: String,
    conn: zbus::Connection,
}

enum State {
    Init,
    Active(zbus::Connection),
    Error,
}

impl TrayService {
    async fn initialize_data(conn: &zbus::Connection) -> anyhow::Result<TrayData> {
        debug!("initializing tray data");
        let proxy = StatusNotifierWatcherProxy::new(&conn).await?;

        debug!("created proxy");
        let test = proxy.registered_status_notifier_items().await?;

        debug!("get registered items");
        println!("{:?}", test);

        Ok(TrayData::default())
    }

    async fn events(conn: &zbus::Connection) -> anyhow::Result<impl Stream<Item = Vec<String>>> {
        let proxy = StatusNotifierWatcherProxy::new(&conn).await?;

        Ok(proxy
            .receive_registered_status_notifier_items_changed()
            .await
            .map(move |_| {
                proxy
                    .cached_registered_status_notifier_items()
                    .unwrap_or_default()
                    .unwrap_or_default()
            }))
    }

    async fn start_listening(state: State, output: &mut Sender<ServiceEvent<Self>>) -> State {
        match state {
            State::Init => match StatusNotifierWatcher::start_server().await {
                Ok(conn) => {
                    let data = TrayService::initialize_data(&conn).await;

                    match data {
                        Ok(data) => {
                            info!("Tray service initialized");

                            // let _ = output
                            //     .send(ServiceEvent::Init(TrayService {
                            //         data,
                            //         conn: conn.clone(),
                            //     }))
                            //     .await;

                            State::Active(conn)
                        }
                        Err(err) => {
                            error!("Failed to initialize tray service: {}", err);

                            State::Error
                        }
                    }
                }
                Err(err) => {
                    error!("Failed to connect to system bus: {}", err);

                    State::Error
                }
            },
            State::Active(conn) => {
                info!("Listening for tray events");

                match TrayService::events(&conn).await {
                    Ok(mut events) => {
                        while let Some(data) = events.next().await {
                            warn!("tray data {:?}", data);
                            // let _ = output.send(ServiceEvent::Update(data)).await;
                        }

                        State::Active(conn)
                    }
                    Err(err) => {
                        error!("Failed to listen for tray events: {}", err);
                        State::Error
                    }
                }
            }
            State::Error => {
                error!("Tray service error");

                let _ = pending::<u8>().next().await;
                State::Error
            }
        }
    }
}

impl ReadOnlyService for TrayService {
    type UpdateEvent = TrayData;
    type Error = ();

    fn update(&mut self, event: Self::UpdateEvent) {
        self.data = event;
    }

    fn subscribe() -> iced::Subscription<ServiceEvent<Self>> {
        let id = TypeId::of::<Self>();

        channel(id, 100, |mut output| async move {
            let mut state = State::Init;

            loop {
                state = TrayService::start_listening(state, &mut output).await;
            }
        })
    }
}
