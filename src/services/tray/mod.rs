use super::xdg_icons;
use super::{ReadOnlyService, Service, ServiceEvent};
use dbus::{
    DBusMenuProxy, Layout, StatusNotifierItemProxy, StatusNotifierWatcher,
    StatusNotifierWatcherProxy,
};
use iced::{
    Subscription, Task,
    futures::{
        FutureExt, SinkExt, Stream, StreamExt,
        channel::mpsc::{self, Sender},
        future::{self, BoxFuture, Either, select},
        stream::{
            self, AbortHandle, Aborted, BoxStream, FuturesUnordered, SelectAll, abortable, pending,
            select_all,
        },
    },
    stream::channel,
    widget::image,
};
use log::{debug, error, info, trace, warn};
use std::{
    any::TypeId,
    collections::HashMap,
    ops::Deref,
    sync::atomic::{AtomicU32, Ordering},
    time::Duration,
};
use tokio_stream::wrappers::UnboundedReceiverStream;
use zbus::{
    fdo::{RequestNameFlags, RequestNameReply},
    names::UniqueName,
};

pub mod dbus;

pub type TrayIcon = super::xdg_icons::XdgIcon;

/// Retry delays for an item that failed to load.
const ITEM_RETRY_DELAYS: [Duration; 3] = [
    Duration::from_millis(500),
    Duration::from_secs(2),
    Duration::from_secs(5),
];

/// The object answered but has no such property.
fn is_missing_property(err: &zbus::Error) -> bool {
    matches!(
        err,
        zbus::Error::FDO(e) if matches!(
            **e,
            zbus::fdo::Error::UnknownProperty(_) | zbus::fdo::Error::InvalidArgs(_)
        )
    )
}

/// Menu paths clients use to say they have no dbusmenu.
fn is_no_menu_path(path: &str) -> bool {
    path.is_empty() || path == "/" || path == "/NO_DBUSMENU"
}

fn pixmap_to_icon(icons: Vec<dbus::Icon>) -> Option<TrayIcon> {
    icons
        .into_iter()
        .filter(|i| {
            // SNI clients sometimes return entries with zero dimensions or a
            // bytes payload that doesn't match width*height*4 (e.g. when only
            // IconName is populated). Feeding those to iced's atlas uploader
            // panics in the padding loop, so drop them up front.
            if i.width <= 0 || i.height <= 0 {
                debug!(
                    "unable to convert pixmap to icon: invalid dimensions {}x{}",
                    i.width, i.height
                );
                return false;
            }
            let expected = (i.width as usize)
                .checked_mul(i.height as usize)
                .and_then(|v| v.checked_mul(4));

            if Some(i.bytes.len()) != expected {
                debug!(
                    "pixmap byte mismatch ({}x{} expected {:?} bytes, got {})",
                    i.width,
                    i.height,
                    expected,
                    i.bytes.len()
                );
                return false;
            }

            true
        })
        .max_by_key(|i| {
            trace!("tray icon w {}, h {}", i.width, i.height);
            (i.width, i.height)
        })
        .map(|mut i| {
            // Convert ARGB to RGBA
            for pixel in i.bytes.as_chunks_mut::<4>().0 {
                pixel.rotate_left(1);
            }
            TrayIcon::Image(image::Handle::from_rgba(
                i.width as u32,
                i.height as u32,
                i.bytes,
            ))
        })
}

async fn current_icon_from_proxy(item_proxy: &StatusNotifierItemProxy<'_>) -> Option<TrayIcon> {
    if let Ok(icons) = item_proxy.icon_pixmap().await
        && let Some(icon) = pixmap_to_icon(icons)
    {
        return Some(icon);
    }
    if let Some(name) = item_proxy.icon_name().await.ok()
        && !name.is_empty()
        && let Some(icon) = icon_from_name(&name, &icon_theme_path(item_proxy).await)
    {
        return Some(icon);
    }
    None
}

/// Looks `name` up in the item's `IconThemePath` first, then the icon theme.
fn icon_from_name(name: &str, theme_path: &str) -> Option<TrayIcon> {
    xdg_icons::get_icon_from_theme_path(theme_path, name)
        .or_else(|| xdg_icons::get_icon_from_name(name))
}

/// `IconThemePath` is a KDE extension; empty when absent.
async fn icon_theme_path(item_proxy: &StatusNotifierItemProxy<'_>) -> String {
    item_proxy.icon_theme_path().await.unwrap_or_default()
}

async fn current_attention_icon_from_proxy(
    item_proxy: &StatusNotifierItemProxy<'_>,
) -> Option<TrayIcon> {
    if let Ok(icons) = item_proxy.attention_icon_pixmap().await
        && let Some(icon) = pixmap_to_icon(icons)
    {
        return Some(icon);
    }
    if let Some(name) = item_proxy.attention_icon_name().await.ok()
        && !name.is_empty()
        && let Some(icon) = icon_from_name(&name, &icon_theme_path(item_proxy).await)
    {
        return Some(icon);
    }
    None
}

/// Runs `read` once per burst of `triggers`, and keeps draining them meanwhile:
/// zbus stops reading the socket, method replies included, while a signal stream
/// holds 64 unread messages.
fn coalesced_reads<T, Fut>(
    triggers: impl Stream<Item = ()> + Send + 'static,
    read: impl FnMut(()) -> Fut + Send + 'static,
) -> BoxStream<'static, T>
where
    T: Send + 'static,
    Fut: Future<Output = Option<T>> + Send + 'static,
{
    let (mut pending_read, reads) = mpsc::channel(0);
    let drain = triggers.filter_map(move |()| {
        // A full channel means a read is already queued.
        let _ = pending_read.try_send(());
        std::future::ready(None)
    });
    stream::select(drain, reads.filter_map(read)).boxed()
}

#[derive(Debug, Clone)]
pub enum TrayEvent {
    Registered(Box<StatusNotifierItem>),
    IconChanged(String, TrayIcon),
    AttentionIconChanged(String, TrayIcon),
    MenuLayoutChanged(String, Layout),
    StatusChanged(String, String),
    Unregistered(String),
    None,
}

/// SNI `Status` value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ItemStatus {
    #[default]
    Passive,
    Active,
    NeedsAttention,
}

impl From<&str> for ItemStatus {
    fn from(s: &str) -> Self {
        match s {
            "Active" => Self::Active,
            "NeedsAttention" => Self::NeedsAttention,
            _ => Self::Passive,
        }
    }
}

#[derive(Debug, Clone)]
pub struct StatusNotifierItem {
    pub name: String,
    pub icon: Option<TrayIcon>,
    /// `None` while the layout is unknown, or when the item has no menu.
    pub menu: Option<Layout>,
    pub status: ItemStatus,
    pub attention_icon: Option<TrayIcon>,
    item_proxy: StatusNotifierItemProxy<'static>,
    menu_proxy: Option<DBusMenuProxy<'static>>,
}

impl StatusNotifierItem {
    pub async fn new(conn: &zbus::Connection, name: String) -> anyhow::Result<Self> {
        let (dest, path) = dbus::split_service_name(&name);

        // No cache: its `GetAll` would decode every pixmap the item has, and
        // NewIcon, NewAttentionIcon and NewStatus come without a PropertiesChanged.
        let item_proxy = StatusNotifierItemProxy::builder(conn)
            .destination(dest.to_owned())
            .path(path.to_owned())
            .cache_properties(zbus::proxy::CacheProperties::No)
            .build()
            .await?;

        debug!("item_proxy {item_proxy:?}");

        // `Menu` is optional; another error means the object is not there yet.
        let menu_path = match item_proxy.menu().await {
            Ok(path) if !is_no_menu_path(path.as_str()) => Some(path),
            Ok(_) => None,
            Err(err) if is_missing_property(&err) => None,
            Err(err) => return Err(err.into()),
        };

        let icon = current_icon_from_proxy(&item_proxy).await;

        // A failed `Status` read means the client skips the property, not
        // that it is passive; only an explicit `Passive` hides an item.
        let status_str = item_proxy
            .status()
            .await
            .unwrap_or_else(|_| "Active".to_owned());
        let status = ItemStatus::from(status_str.as_str());

        let attention_icon = current_attention_icon_from_proxy(&item_proxy).await;

        let menu_proxy = match menu_path {
            Some(menu_path) => Some(
                dbus::DBusMenuProxy::builder(conn)
                    .destination(dest.to_owned())
                    .path(menu_path.to_owned())
                    .build()
                    .await?,
            ),
            None => {
                debug!("tray item {name} has no menu");
                None
            }
        };

        let menu = match &menu_proxy {
            Some(menu_proxy) => match menu_proxy.get_layout(0, -1, &[]).await {
                Ok((_, layout)) => Some(layout),
                Err(err) => {
                    // A later LayoutUpdated fills it in.
                    debug!("tray item {name}: menu layout unavailable: {err}");
                    None
                }
            },
            None => None,
        };

        Ok(Self {
            name,
            icon,
            menu,
            status,
            attention_icon,
            item_proxy,
            menu_proxy,
        })
    }

    pub fn has_menu(&self) -> bool {
        self.menu_proxy.is_some()
    }

    /// Change notifications for this item. Reads are coalesced, so a signal
    /// burst cannot fill zbus's queue and deadlock the socket reader.
    async fn event_stream(&self, conn: &zbus::Connection) -> BoxStream<'static, TrayEvent> {
        let name = self.name.clone();
        let mut streams: Vec<BoxStream<'static, TrayEvent>> = Vec::with_capacity(4);

        // A signal only triggers a fresh read: its args would decode a
        // pixmap one `Variant` per byte.
        let item_inner = self.item_proxy.inner();
        let mut icon_triggers = Vec::with_capacity(4);
        if let Ok(properties) = zbus::fdo::PropertiesProxy::new(
            conn,
            item_inner.destination().to_owned(),
            item_inner.path().to_owned(),
        )
        .await
            && let Ok(properties_changed) = properties.receive_properties_changed().await
        {
            icon_triggers.push(properties_changed.map(|_| ()).boxed());
        }
        if let Ok(new_icon) = self.item_proxy.receive_new_icon().await {
            icon_triggers.push(new_icon.map(|_| ()).boxed());
        }
        if let Ok(new_icon_theme_path) = self.item_proxy.receive_new_icon_theme_path().await {
            icon_triggers.push(new_icon_theme_path.map(|_| ()).boxed());
        }
        // One read once subscribed, so a change that raced `new` is not lost.
        icon_triggers.push(stream::once(std::future::ready(())).boxed());
        streams.push(coalesced_reads(select_all(icon_triggers), {
            let name = name.clone();
            let item_proxy = self.item_proxy.clone();
            move |()| {
                let name = name.clone();
                let item_proxy = item_proxy.clone();
                async move {
                    current_icon_from_proxy(&item_proxy)
                        .await
                        .map(|icon| TrayEvent::IconChanged(name, icon))
                }
            }
        }));

        if let Ok(new_attention_icon) = self.item_proxy.receive_new_attention_icon().await {
            streams.push(coalesced_reads(new_attention_icon.map(|_| ()), {
                let name = name.clone();
                let item_proxy = self.item_proxy.clone();
                move |()| {
                    let name = name.clone();
                    let item_proxy = item_proxy.clone();
                    async move {
                        current_attention_icon_from_proxy(&item_proxy)
                            .await
                            .map(|icon| TrayEvent::AttentionIconChanged(name, icon))
                    }
                }
            }));
        }

        if let Ok(new_status) = self.item_proxy.receive_new_status().await {
            streams.push(
                new_status
                    .filter_map({
                        let name = name.clone();
                        move |signal| {
                            let name = name.clone();
                            async move {
                                let status = signal.args().ok()?.status.to_owned();
                                Some(TrayEvent::StatusChanged(name, status))
                            }
                        }
                    })
                    .boxed(),
            );
        }

        if let Some(menu_proxy) = self.menu_proxy.clone() {
            let mut layout_triggers = Vec::with_capacity(2);
            if let Ok(layout_updated) = menu_proxy.receive_layout_updated().await {
                layout_triggers.push(layout_updated.map(|_| ()).boxed());
            }
            if let Ok(items_removed) = menu_proxy.receive_items_removed().await {
                layout_triggers.push(items_removed.map(|_| ()).boxed());
            }
            streams.push(coalesced_reads(select_all(layout_triggers), {
                let name = name.clone();
                move |()| {
                    debug!("menu layout changed for {name}");

                    let name = name.clone();
                    let menu_proxy = menu_proxy.clone();
                    async move {
                        menu_proxy
                            .get_layout(0, -1, &[])
                            .await
                            .ok()
                            .map(|(_, layout)| TrayEvent::MenuLayoutChanged(name, layout))
                    }
                }
            }));
        }

        select_all(streams).boxed()
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
    pub data: TrayData,
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
    Active {
        conn: zbus::Connection,
        /// Our `org.freedesktop.StatusNotifierHost-<pid>-<n>` name, if we got it.
        host: Option<String>,
        /// Kept across event-stream rebuilds so no change is missed.
        watcher_owner: BoxStream<'static, Option<UniqueName<'static>>>,
        /// Re-register the host and send a fresh `Init`.
        reload: bool,
    },
    Error,
}

static HOST_ID: AtomicU32 = AtomicU32::new(0);

/// Uncached: a foreign watcher need not emit `PropertiesChanged`.
async fn watcher_proxy(
    conn: &zbus::Connection,
) -> zbus::Result<StatusNotifierWatcherProxy<'static>> {
    StatusNotifierWatcherProxy::builder(conn)
        .cache_properties(zbus::proxy::CacheProperties::No)
        .build()
        .await
}

type LoadedItem = (StatusNotifierItem, BoxStream<'static, TrayEvent>);
type ItemLoad = BoxFuture<'static, (String, usize, Result<anyhow::Result<LoadedItem>, Aborted>)>;

/// Per-item loads and streams by name. Loads run as futures, so awaiting one
/// cannot stop polling the streams.
#[derive(Default)]
struct ItemStreams {
    events: SelectAll<BoxStream<'static, TrayEvent>>,
    handles: HashMap<String, AbortHandle>,
    /// In flight or waiting for a retry.
    loads: FuturesUnordered<ItemLoad>,
    load_handles: HashMap<String, AbortHandle>,
}

impl ItemStreams {
    /// Loads `name` after `delay`, unless it is loaded or loading.
    fn load(&mut self, conn: &zbus::Connection, name: String, attempt: usize, delay: Duration) {
        if self.handles.contains_key(&name) || self.load_handles.contains_key(&name) {
            return;
        }

        let conn = conn.clone();
        let (load, handle) = future::abortable({
            let name = name.clone();
            async move {
                tokio::time::sleep(delay).await;
                let item = StatusNotifierItem::new(&conn, name).await?;
                let events = item.event_stream(&conn).await;
                anyhow::Ok((item, events))
            }
        });
        self.load_handles.insert(name.clone(), handle);
        self.loads
            .push(load.map(move |result| (name, attempt, result)).boxed());
    }

    /// Hands a finished load's subscriptions over, or queues a retry.
    fn finish_load(
        &mut self,
        conn: &zbus::Connection,
        name: String,
        attempt: usize,
        result: anyhow::Result<LoadedItem>,
    ) -> Option<StatusNotifierItem> {
        self.load_handles.remove(&name);
        match result {
            Ok((item, events)) => {
                let (events, handle) = abortable(events);
                self.events.push(events.boxed());
                self.handles.insert(name, handle);
                Some(item)
            }
            Err(err) => {
                match ITEM_RETRY_DELAYS.get(attempt) {
                    Some(&delay) => {
                        debug!("tray item {name} not ready (attempt {attempt}): {err}");
                        self.load(conn, name, attempt + 1, delay);
                    }
                    None => warn!("Giving up on tray item {name}: {err}"),
                }
                None
            }
        }
    }

    fn remove(&mut self, name: &str) {
        for handles in [&mut self.handles, &mut self.load_handles] {
            if let Some(handle) = handles.remove(name) {
                handle.abort();
            }
        }
    }
}

impl TrayService {
    async fn request_host_name(conn: &zbus::Connection) -> Option<String> {
        let host = format!(
            "org.freedesktop.StatusNotifierHost-{}-{}",
            std::process::id(),
            HOST_ID.fetch_add(1, Ordering::Relaxed) + 1
        );
        match conn
            .request_name_with_flags(host.as_str(), RequestNameFlags::DO_NOT_QUEUE)
            .await
        {
            Ok(RequestNameReply::PrimaryOwner | RequestNameReply::AlreadyOwner) => Some(host),
            Ok(reply) => {
                warn!("Failed to own {host}: {reply:?}");
                None
            }
            Err(err) => {
                warn!("Failed to own {host}: {err}");
                None
            }
        }
    }

    async fn initialize_data(conn: &zbus::Connection) -> anyhow::Result<TrayData> {
        debug!("initializing tray data");
        let proxy = watcher_proxy(conn).await?;

        let items = proxy.registered_status_notifier_items().await?;

        let mut status_items = Vec::with_capacity(items.len());
        for name in items {
            match StatusNotifierItem::new(conn, name.clone()).await {
                Ok(item) => status_items.push(item),
                Err(err) => warn!("Skipping tray item {name}: {err}"),
            }
        }

        Ok(TrayData(status_items))
    }

    /// Watcher-driven events. Per-item loads run alongside the streams, so a
    /// registration never rebuilds them.
    async fn events(
        conn: &zbus::Connection,
    ) -> anyhow::Result<impl Stream<Item = TrayEvent> + use<>> {
        let watcher = watcher_proxy(conn).await?;

        // Subscribe before reading the list, so an item registering in between
        // is seen by one of the two.
        let mut registered = watcher.receive_status_notifier_item_registered().await?;
        let mut unregistered = watcher.receive_status_notifier_item_unregistered().await?;
        let items = watcher.registered_status_notifier_items().await?;

        let conn = conn.clone();
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        tokio::spawn(async move {
            let mut streams = ItemStreams::default();

            // `Init` loads these too; `Registered` replaces that copy.
            for name in items {
                streams.load(&conn, name, 0, Duration::ZERO);
            }

            loop {
                // No arm awaits: every stream has to keep being polled.
                let event = tokio::select! {
                    _ = tx.closed() => break,
                    Some(signal) = registered.next() => {
                        if let Ok(args) = signal.args() {
                            let name = args.service.to_string();
                            debug!("registered {name}");

                            streams.load(&conn, name, 0, Duration::ZERO);
                        }
                        None
                    }
                    Some((name, attempt, result)) = streams.loads.next() => match result {
                        Ok(result) => streams
                            .finish_load(&conn, name, attempt, result)
                            .map(|item| TrayEvent::Registered(Box::new(item))),
                        // Cancelled by `Unregistered`.
                        Err(Aborted) => None,
                    },
                    Some(signal) = unregistered.next() => {
                        match signal.args() {
                            Ok(args) => {
                                let name = args.service.to_string();
                                debug!("unregistered {name}");

                                streams.remove(&name);
                                Some(TrayEvent::Unregistered(name))
                            }
                            _ => None,
                        }
                    }
                    Some(event) = streams.events.next() => Some(event),
                    else => break,
                };

                if let Some(event) = event
                    && tx.send(event).is_err()
                {
                    break;
                }
            }
        });

        Ok(UnboundedReceiverStream::new(rx))
    }

    async fn start_listening(state: State, output: &mut Sender<ServiceEvent<Self>>) -> State {
        match state {
            State::Init => match StatusNotifierWatcher::start_server().await {
                Ok(conn) => {
                    let watcher_owner = match watcher_proxy(&conn).await {
                        Ok(watcher) => watcher.inner().receive_owner_changed().await,
                        Err(err) => Err(err),
                    };
                    match watcher_owner {
                        Ok(watcher_owner) => State::Active {
                            host: TrayService::request_host_name(&conn).await,
                            conn,
                            watcher_owner: watcher_owner.boxed(),
                            reload: true,
                        },
                        Err(err) => {
                            error!("Failed to watch the tray watcher: {err}");

                            State::Error
                        }
                    }
                }
                Err(err) => {
                    error!("Failed to connect to system bus: {err}");

                    State::Error
                }
            },
            State::Active {
                conn,
                host,
                mut watcher_owner,
                reload,
            } => {
                info!("Listening for tray events");

                if reload
                    && let Some(host) = &host
                    && let Err(err) = async {
                        watcher_proxy(&conn)
                            .await?
                            .register_status_notifier_host(host)
                            .await
                    }
                    .await
                {
                    debug!("Failed to register tray host {host}: {err}");
                }

                // Subscribe before reading the items, so none is missed.
                let events = async {
                    let events = TrayService::events(&conn).await?;
                    let data = if reload {
                        Some(TrayService::initialize_data(&conn).await?)
                    } else {
                        None
                    };
                    anyhow::Ok((events, data))
                }
                .await;
                let mut events = match events {
                    Ok((events, data)) => {
                        if let Some(data) = data {
                            info!("Tray service initialized");

                            let _ = output
                                .send(ServiceEvent::Init(TrayService {
                                    data,
                                    _conn: conn.clone(),
                                }))
                                .await;
                        }
                        events
                    }
                    Err(err) => {
                        // Typically no watcher right now: wait for one.
                        error!("Failed to listen for tray events: {err}");
                        if reload {
                            let _ = output
                                .send(ServiceEvent::Init(TrayService {
                                    data: TrayData::default(),
                                    _conn: conn.clone(),
                                }))
                                .await;
                        }
                        return match watcher_owner.next().await {
                            Some(_) => State::Active {
                                conn,
                                host,
                                watcher_owner,
                                reload: true,
                            },
                            None => State::Error,
                        };
                    }
                };

                loop {
                    let next = match select(events.next(), watcher_owner.next()).await {
                        Either::Left((event, _)) => Either::Left(event),
                        Either::Right((owner, _)) => Either::Right(owner),
                    };
                    match next {
                        Either::Left(Some(event)) => {
                            debug!("tray data {event:?}");

                            // No rebuild on registration: the event stream
                            // adds and drops per-item streams itself.
                            let _ = output.send(ServiceEvent::Update(event)).await;
                        }
                        Either::Left(None) => {
                            break State::Active {
                                conn,
                                host,
                                watcher_owner,
                                reload: false,
                            };
                        }
                        Either::Right(Some(owner)) => {
                            info!("Tray watcher owner changed to {owner:?}, reloading items");
                            break State::Active {
                                conn,
                                host,
                                watcher_owner,
                                reload: true,
                            };
                        }
                        Either::Right(None) => {
                            error!("Tray watcher owner stream ended");
                            break State::Error;
                        }
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

    async fn menu_voice_selected(
        menu_proxy: &DBusMenuProxy<'_>,
        id: i32,
    ) -> anyhow::Result<Layout> {
        let value = zbus::Variant::I32(32).try_to_owned()?;
        menu_proxy
            .event(
                id,
                "clicked",
                &value,
                chrono::offset::Local::now().timestamp_subsec_micros(),
            )
            .await?;

        let (_, layout) = menu_proxy.get_layout(0, -1, &[]).await?;

        Ok(layout)
    }
}

impl ReadOnlyService for TrayService {
    type UpdateEvent = TrayEvent;
    type Error = ();

    fn update(&mut self, event: Self::UpdateEvent) {
        match event {
            TrayEvent::Registered(new_item) => {
                let new_item = *new_item;
                // Replace by name as a defensive fallback, otherwise append.
                match self
                    .data
                    .0
                    .iter_mut()
                    .find(|item| item.name == new_item.name)
                {
                    Some(existing_item) => {
                        *existing_item = new_item;
                    }
                    _ => {
                        self.data.0.push(new_item);
                    }
                }
            }
            TrayEvent::IconChanged(name, handle) => {
                if let Some(item) = self.data.0.iter_mut().find(|item| item.name == name) {
                    item.icon = Some(handle);
                }
            }
            TrayEvent::AttentionIconChanged(name, handle) => {
                if let Some(item) = self.data.0.iter_mut().find(|item| item.name == name) {
                    item.attention_icon = Some(handle);
                }
            }
            TrayEvent::MenuLayoutChanged(name, layout) => {
                if let Some(item) = self.data.0.iter_mut().find(|item| item.name == name) {
                    debug!("menu layout updated, {layout:?}");
                    item.menu = Some(layout);
                }
            }
            TrayEvent::StatusChanged(name, status) => {
                if let Some(item) = self.data.0.iter_mut().find(|item| item.name == name) {
                    item.status = ItemStatus::from(status.as_str());
                }
            }
            TrayEvent::Unregistered(name) => {
                self.data.0.retain(|item| item.name != name);
            }
            TrayEvent::None => {}
        }
    }

    fn subscribe() -> iced::Subscription<ServiceEvent<Self>> {
        Subscription::run_with(TypeId::of::<Self>(), |_| {
            channel(100, async |mut output| {
                let mut state = State::Init;

                loop {
                    state = TrayService::start_listening(state, &mut output).await;
                }
            })
        })
    }
}

#[derive(Debug, Clone)]
pub enum TrayCommand {
    MenuSelected(String, i32),
    Activate(String),
}

impl Service for TrayService {
    type Command = TrayCommand;

    fn command(&mut self, command: Self::Command) -> Task<ServiceEvent<Self>> {
        match command {
            TrayCommand::MenuSelected(name, id) => {
                let menu_proxy = self
                    .data
                    .iter()
                    .find(|item| item.name == name)
                    .and_then(|item| item.menu_proxy.clone());
                if let Some(proxy) = menu_proxy {
                    let name_cb = name.clone();
                    Task::perform(
                        async move {
                            debug!("Click tray menu voice {name} : {id}");
                            TrayService::menu_voice_selected(&proxy, id).await
                        },
                        move |new_layout| match new_layout {
                            Ok(new_layout) => ServiceEvent::Update(TrayEvent::MenuLayoutChanged(
                                name_cb.clone(),
                                new_layout,
                            )),
                            _ => ServiceEvent::Update(TrayEvent::None),
                        },
                    )
                } else {
                    Task::none()
                }
            }
            TrayCommand::Activate(name) => {
                let item = self.data.iter().find(|item| item.name == name);
                if let Some(item) = item {
                    Task::perform(
                        {
                            let proxy = item.item_proxy.clone();
                            async move {
                                debug!("Activate tray item {name}");
                                let _ = proxy.activate(0, 0).await;
                            }
                        },
                        |_| ServiceEvent::Update(TrayEvent::None),
                    )
                } else {
                    Task::none()
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ItemStatus, pixmap_to_icon};
    use crate::services::tray::dbus::Icon;

    fn valid_pixmap(width: i32, height: i32) -> Icon {
        Icon {
            width,
            height,
            bytes: vec![0u8; (width as usize) * (height as usize) * 4],
        }
    }

    #[test]
    fn item_status_parses_active() {
        assert_eq!(ItemStatus::from("Active"), ItemStatus::Active);
    }

    #[test]
    fn item_status_parses_needs_attention() {
        assert_eq!(
            ItemStatus::from("NeedsAttention"),
            ItemStatus::NeedsAttention
        );
    }

    #[test]
    fn item_status_defaults_passive_for_unknown_and_empty() {
        assert_eq!(ItemStatus::from("Passive"), ItemStatus::Passive);
        assert_eq!(ItemStatus::from(""), ItemStatus::Passive);
        assert_eq!(ItemStatus::from("garbage"), ItemStatus::Passive);
    }

    #[test]
    fn item_status_default_is_passive() {
        assert_eq!(ItemStatus::default(), ItemStatus::Passive);
    }

    #[test]
    fn pixmap_to_icon_accepts_valid_pixmap() {
        let icon = valid_pixmap(24, 24);
        let result = pixmap_to_icon(vec![icon]);
        assert!(result.is_some());
    }

    #[test]
    fn pixmap_to_icon_rejects_zero_dimension_pixmap() {
        let icon = Icon {
            width: 0,
            height: 24,
            bytes: vec![],
        };
        assert!(pixmap_to_icon(vec![icon]).is_none());
    }

    #[test]
    fn pixmap_to_icon_rejects_byte_mismatch() {
        let icon = Icon {
            width: 24,
            height: 24,
            bytes: vec![0u8; 100],
        };
        assert!(pixmap_to_icon(vec![icon]).is_none());
    }

    #[test]
    fn pixmap_to_icon_picks_largest_pixmap() {
        let small = valid_pixmap(16, 16);
        let large = valid_pixmap(48, 48);

        let result = pixmap_to_icon(vec![small, large]).expect("expected an icon");

        assert!(matches!(result, crate::services::tray::TrayIcon::Image(_)));
    }

    #[test]
    fn pixmap_to_icon_returns_none_for_empty_vec() {
        assert!(pixmap_to_icon(Vec::new()).is_none());
    }
}
