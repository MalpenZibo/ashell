use super::{ReadOnlyService, ServiceEvent};
use dbus::{
    DBusMenuProxy, Layout, StatusNotifierItemProxy, StatusNotifierWatcher,
    StatusNotifierWatcherProxy,
};
use iced::{
    futures::{
        channel::mpsc::Sender,
        stream::{pending, select_all},
        stream_select, SinkExt, Stream, StreamExt,
    },
    stream::channel,
    widget::image::Handle,
    Subscription,
};
use log::{debug, error, info, trace};
use std::{any::TypeId, ops::Deref};

mod dbus;

#[derive(Debug, Clone)]
pub enum TrayEvent {
    Registered(StatusNotifierItem),
    IconChanged((String, Handle)),
    Unregistered(String),
}

#[derive(Debug, Clone)]
pub struct StatusNotifierItem {
    pub name: String,
    pub icon_pixmap: Option<Handle>,
    pub menu: Layout,
    item_proxy: StatusNotifierItemProxy<'static>,
    menu_proxy: DBusMenuProxy<'static>,
}

impl StatusNotifierItem {
    pub async fn new(conn: &zbus::Connection, name: String) -> anyhow::Result<Self> {
        let (dest, path) = if let Some(idx) = name.find('/') {
            (&name[..idx], &name[idx..])
        } else {
            (name.as_ref(), "/StatusNotifierItem")
        };

        let item_proxy = StatusNotifierItemProxy::builder(conn)
            .destination(dest.to_owned())?
            .path(path.to_owned())?
            .build()
            .await?;

        let icon_pixmap = item_proxy
            .icon_pixmap()
            .await
            .unwrap_or_default()
            .into_iter()
            .max_by_key(|i| {
                trace!("tray icon w {}, h {}", i.width, i.height);
                (i.width, i.height)
            })
            .map(|mut i| {
                // Convert ARGB to RGBA
                for pixel in i.bytes.chunks_exact_mut(4) {
                    pixel.rotate_left(1);
                }
                Handle::from_rgba(i.width as u32, i.height as u32, i.bytes)
            });

        let menu_path = item_proxy.menu().await?;
        let menu_proxy = dbus::DBusMenuProxy::builder(conn)
            .destination(dest.to_owned())?
            .path(menu_path.to_owned())?
            .build()
            .await?;

        let (_, menu) = menu_proxy.get_layout(0, -1, &[]).await?;

        Ok(Self {
            name,
            icon_pixmap,
            menu,
            item_proxy,
            menu_proxy,
        })
    }
}

#[derive(Debug, Default, Clone)]
pub struct TrayData(Vec<StatusNotifierItem>);

impl Deref for TrayData {
    type Target = Vec<StatusNotifierItem>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug, Clone)]
pub struct TrayService {
    data: TrayData,
    _conn: zbus::Connection,
}

impl Deref for TrayService {
    type Target = TrayData;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

enum State {
    Init,
    Active(zbus::Connection),
    Error,
}

impl TrayService {
    async fn initialize_data(conn: &zbus::Connection) -> anyhow::Result<TrayData> {
        debug!("initializing tray data");
        let proxy = StatusNotifierWatcherProxy::new(conn).await?;

        let items = proxy.registered_status_notifier_items().await?;

        let mut status_items = Vec::with_capacity(items.len());
        for item in items {
            let item = StatusNotifierItem::new(conn, item.to_string()).await?;
            status_items.push(item);
        }

        debug!("created items: {:?}", status_items);

        Ok(TrayData(status_items))
    }

    async fn events(conn: &zbus::Connection) -> anyhow::Result<impl Stream<Item = TrayEvent>> {
        let watcher = StatusNotifierWatcherProxy::new(conn).await?;

        let registered = watcher
            .receive_status_notifier_item_registered()
            .await?
            .filter_map({
                let conn = conn.clone();
                move |e| {
                    let conn = conn.clone();
                    async move {
                        debug!("registered {:?}", e);
                        if let Ok(args) = e.args() {
                            let item =
                                StatusNotifierItem::new(&conn, args.service.to_string()).await;

                            item.map(TrayEvent::Registered).ok()
                        } else {
                            None
                        }
                    }
                }
            })
            .boxed();
        let unregistered = watcher
            .receive_status_notifier_item_unregistered()
            .await?
            .filter_map(|e| async move {
                debug!("unregistered {:?}", e);

                if let Ok(args) = e.args() {
                    Some(TrayEvent::Unregistered(args.service.to_string()))
                } else {
                    None
                }
            })
            .boxed();

        let items = watcher.registered_status_notifier_items().await?;
        let mut icon_pixel_change = Vec::with_capacity(items.len());

        for name in items {
            let item = StatusNotifierItem::new(conn, name.to_string()).await?;

            icon_pixel_change.push(
                item.item_proxy
                    .receive_icon_pixmap_changed()
                    .await
                    .filter_map({
                        let name = name.clone();
                        move |icon| {
                            let name = name.clone();
                            async move {
                                icon.get().await.ok().and_then(|icon| {
                                    icon.into_iter()
                                        .max_by_key(|i| {
                                            trace!("tray icon w {}, h {}", i.width, i.height);
                                            (i.width, i.height)
                                        })
                                        .map(|mut i| {
                                            // Convert ARGB to RGBA
                                            for pixel in i.bytes.chunks_exact_mut(4) {
                                                pixel.rotate_left(1);
                                            }
                                            TrayEvent::IconChanged((
                                                name.to_owned(),
                                                Handle::from_rgba(
                                                    i.width as u32,
                                                    i.height as u32,
                                                    i.bytes,
                                                ),
                                            ))
                                        })
                                })
                            }
                        }
                    })
                    .boxed(),
            );
        }

        Ok(stream_select!(registered, unregistered, select_all(icon_pixel_change)).boxed())
    }

    async fn start_listening(state: State, output: &mut Sender<ServiceEvent<Self>>) -> State {
        match state {
            State::Init => match StatusNotifierWatcher::start_server().await {
                Ok(conn) => {
                    let data = TrayService::initialize_data(&conn).await;

                    match data {
                        Ok(data) => {
                            info!("Tray service initialized");

                            let _ = output
                                .send(ServiceEvent::Init(TrayService {
                                    data,
                                    _conn: conn.clone(),
                                }))
                                .await;

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
                        while let Some(event) = events.next().await {
                            debug!("tray data {:?}", event);

                            let reload_events = matches!(event, TrayEvent::Registered(_));

                            let _ = output.send(ServiceEvent::Update(event)).await;

                            if reload_events {
                                break;
                            }
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
    type UpdateEvent = TrayEvent;
    type Error = ();

    fn update(&mut self, event: Self::UpdateEvent) {
        match event {
            TrayEvent::Registered(new_item) => {
                if let Some(existing_item) = self
                    .data
                    .0
                    .iter_mut()
                    .find(|item| item.name == new_item.name)
                {
                    *existing_item = new_item;
                } else {
                    self.data.0.push(new_item);
                }
            }
            TrayEvent::IconChanged((name, handle)) => {
                if let Some(item) = self.data.0.iter_mut().find(|item| item.name == name) {
                    item.icon_pixmap = Some(handle);
                }
            }
            TrayEvent::Unregistered(name) => {
                self.data.0.retain(|item| item.name != name);
            }
        }
    }

    fn subscribe() -> iced::Subscription<ServiceEvent<Self>> {
        let id = TypeId::of::<Self>();

        Subscription::run_with_id(
            id,
            channel(100, |mut output| async move {
                let mut state = State::Init;

                loop {
                    state = TrayService::start_listening(state, &mut output).await;
                }
            }),
        )
    }
}
