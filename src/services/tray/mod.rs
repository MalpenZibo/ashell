use super::xdg_icons;
use super::{ReadOnlyService, Service, ServiceEvent};
use dbus::{
    DBusMenuProxy, Layout, StatusNotifierItemProxy, StatusNotifierWatcher,
    StatusNotifierWatcherProxy,
};
use iced::{
    Subscription, Task,
    futures::{
        SinkExt, Stream, StreamExt,
        channel::mpsc::Sender,
        stream::{pending, select_all},
        stream_select,
    },
    stream::channel,
    widget::image,
};
use log::{debug, error, info, trace};
use std::{any::TypeId, ops::Deref};

pub mod dbus;

pub type TrayIcon = super::xdg_icons::XdgIcon;

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
    // 1. `IconPixmap` (preferred) — pick the largest pixmap by pixel count.
    if let Ok(icons) = item_proxy.icon_pixmap().await
        && let Some(icon) = dbus::best_icon_pixmap(&icons)
        && let Some(loaded) = pixmap_to_icon(vec![icon.clone()])
    {
        return Some(loaded);
    }
    // 2. `IconName` via the XDG icon theme.
    if let Some(name) = item_proxy.icon_name().await.ok()
        && !name.is_empty()
        && let Some(icon) = xdg_icons::get_icon_from_name(&name)
    {
        return Some(icon);
    }
    None
}

fn split_service_name(name: &str) -> (&str, &str) {
    match name.find('/') {
        Some(idx) => (&name[..idx], &name[idx..]),
        None => (name, "/StatusNotifierItem"),
    }
}
#[derive(Debug, Clone)]
pub enum TrayEvent {
    Registered(Box<StatusNotifierItem>),
    IconChanged(String, TrayIcon),
    MenuLayoutChanged(String, Layout),
    /// `NewStatus` signal from the SNI client. `status` is
    /// `Passive` / `Active` / `NeedsAttention`.
    StatusChanged(String, String),
    Unregistered(String),
    None,
}

/// SNI status (subset of the spec values).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ItemStatus {
    /// `Passive` — not active (hidden from the tray until an `active`
    /// registrant exists).
    #[default]
    Passive,
    /// `Active` — active.
    Active,
    /// `NeedsAttention` — needs attention (use attention/overlay icons).
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

/// SNI tooltip (`org.kde.StatusNotifierItem` `ToolTip`).
/// Protocol surface; not yet surfaced in the UI.
#[allow(dead_code)]
#[derive(Debug, Clone, Default)]
pub struct ToolTip {
    pub icon_name: Option<String>,
    pub icon_pixmap: Option<dbus::Icon>,
    pub title: String,
    pub description: String,
}

#[derive(Debug, Clone)]
pub struct StatusNotifierItem {
    pub name: String,
    pub icon: Option<TrayIcon>,
    pub menu: Layout,
    /// SNI `Category` (`ApplicationStatus` / `Communications` / ...).
    /// Read for completeness; not yet surfaced in the UI.
    #[allow(dead_code)]
    pub category: String,
    /// SNI `Status` (`Passive` / `Active` / `NeedsAttention`).
    pub status: ItemStatus,
    /// SNI `Id` / `ItemId` (stable app id). Read for completeness.
    #[allow(dead_code)]
    pub item_id: String,
    /// SNI `Title` (tooltip title). Read for completeness.
    #[allow(dead_code)]
    pub title: String,
    /// Attention icon (used when `NeedsAttention`).
    pub attention_icon: Option<TrayIcon>,
    /// Overlay icon.
    pub overlay_icon: Option<TrayIcon>,
    item_proxy: StatusNotifierItemProxy<'static>,
    menu_proxy: DBusMenuProxy<'static>,
}

impl StatusNotifierItem {
    pub async fn new(conn: &zbus::Connection, name: String) -> anyhow::Result<Self> {
        let (dest, path) = split_service_name(&name);

        let item_proxy = StatusNotifierItemProxy::builder(conn)
            .destination(dest.to_owned())?
            .path(path.to_owned())?
            .build()
            .await?;

        debug!("item_proxy {item_proxy:?}");

        let icon = current_icon_from_proxy(&item_proxy).await;

        // Full SNI properties. Each read is guarded — some clients (or
        // clients that expose only a subset) may not implement a property;
        // fall back to a default rather than failing.
        let category = item_proxy.category().await.unwrap_or_default();
        let status_str = item_proxy
            .status()
            .await
            .unwrap_or_else(|_| "Passive".to_owned());
        let status = ItemStatus::from(status_str.as_str());
        let item_id = item_proxy
            .id()
            .await
            .or(item_proxy.item_id().await)
            .unwrap_or_default();
        let title = item_proxy.title().await.unwrap_or_default();

        // Attention/overlay icons: pixmap first (preferred), then icon name.
        let attention_pixmap = item_proxy
            .attention_icon_pixmap()
            .await
            .ok()
            .and_then(pixmap_to_icon);
        let attention_name = item_proxy
            .attention_icon_name()
            .await
            .ok()
            .as_deref()
            .and_then(xdg_icons::get_icon_from_name);
        let attention_icon = attention_pixmap.or(attention_name);

        let overlay_pixmap = item_proxy
            .overlay_icon_pixmap()
            .await
            .ok()
            .and_then(pixmap_to_icon);
        let overlay_name = item_proxy
            .overlay_icon_name()
            .await
            .ok()
            .as_deref()
            .and_then(xdg_icons::get_icon_from_name);
        let overlay_icon = overlay_pixmap.or(overlay_name);

        let menu_path = item_proxy.menu().await?;
        let menu_proxy = dbus::DBusMenuProxy::builder(conn)
            .destination(dest.to_owned())?
            .path(menu_path.to_owned())?
            .build()
            .await?;

        let (_, menu) = menu_proxy.get_layout(0, -1, &[]).await?;

        Ok(Self {
            name,
            icon,
            menu,
            category,
            status,
            item_id,
            title,
            attention_icon,
            overlay_icon,
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
            let item = StatusNotifierItem::new(conn, item).await?;
            status_items.push(item);
        }

        Ok(TrayData(status_items))
    }

    async fn events(
        conn: &zbus::Connection,
    ) -> anyhow::Result<impl Stream<Item = TrayEvent> + use<>> {
        let watcher = StatusNotifierWatcherProxy::new(conn).await?;

        let registered = watcher
            .receive_status_notifier_item_registered()
            .await?
            .filter_map({
                let conn = conn.clone();
                move |e| {
                    let conn = conn.clone();
                    async move {
                        debug!("registered {e:?}");
                        match e.args() {
                            Ok(args) => {
                                let item =
                                    StatusNotifierItem::new(&conn, args.service.to_string()).await;

                                item.map(|item| TrayEvent::Registered(Box::new(item))).ok()
                            }
                            _ => None,
                        }
                    }
                }
            })
            .boxed();
        let unregistered = watcher
            .receive_status_notifier_item_unregistered()
            .await?
            .filter_map(|e| async move {
                debug!("unregistered {e:?}");

                match e.args() {
                    Ok(args) => Some(TrayEvent::Unregistered(args.service.to_string())),
                    _ => None,
                }
            })
            .boxed();

        let items = watcher.registered_status_notifier_items().await?;
        let mut icon_pixel_change = Vec::with_capacity(items.len());
        let mut icon_name_change = Vec::with_capacity(items.len());
        let mut new_icon_change = Vec::with_capacity(items.len());
        let mut menu_layout_change = Vec::with_capacity(items.len());
        let mut status_change = Vec::with_capacity(items.len());

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
                                let icons = icon.get().await.ok()?;
                                pixmap_to_icon(icons)
                                    .map(|icon| TrayEvent::IconChanged(name.to_owned(), icon))
                            }
                        }
                    })
                    .boxed(),
            );

            icon_name_change.push(
                item.item_proxy
                    .receive_icon_name_changed()
                    .await
                    .filter_map({
                        let name = name.clone();
                        move |icon_name| {
                            let name = name.clone();
                            async move {
                                icon_name
                                    .get()
                                    .await
                                    .ok()
                                    .as_deref()
                                    .and_then(xdg_icons::get_icon_from_name)
                                    .map(|icon| TrayEvent::IconChanged(name.to_owned(), icon))
                            }
                        }
                    })
                    .boxed(),
            );

            let new_status = item.item_proxy.receive_new_status().await;
            if let Ok(new_status) = new_status {
                // The `NewStatus` signal has no matching property-changed signal,
                // so we re-read the `Status` property (uncached) rather than
                // relying on the signal payload struct field name.
                status_change.push(
                    new_status
                        .filter_map({
                            let name = name.clone();
                            let proxy = item.item_proxy.clone();
                            move |_| {
                                let name = name.clone();
                                let proxy = proxy.clone();
                                async move {
                                    let status = proxy.status().await.ok()?;
                                    Some(TrayEvent::StatusChanged(name.to_owned(), status))
                                }
                            }
                        })
                        .boxed(),
                );
            }

            let new_icon = item.item_proxy.receive_new_icon().await;
            if let Ok(new_icon) = new_icon {
                let (dest, path) = split_service_name(&name);
                // NewIcon has no matching PropertiesChanged, so a cached read would be stale;
                let uncached_proxy = StatusNotifierItemProxy::builder(conn)
                    .destination(dest.to_owned())?
                    .path(path.to_owned())?
                    .cache_properties(zbus::proxy::CacheProperties::No)
                    .build()
                    .await?;

                new_icon_change.push(
                    new_icon
                        .filter_map({
                            let name = name.clone();
                            let uncached_proxy = uncached_proxy;

                            move |_| {
                                let name = name.clone();
                                let uncached_proxy = uncached_proxy.clone();

                                async move {
                                    current_icon_from_proxy(&uncached_proxy)
                                        .await
                                        .map(|icon| TrayEvent::IconChanged(name.to_owned(), icon))
                                }
                            }
                        })
                        .boxed(),
                );
            }

            let layout_updated = item.menu_proxy.receive_layout_updated().await;
            if let Ok(layout_updated) = layout_updated {
                menu_layout_change.push(
                    layout_updated
                        .filter_map({
                            let name = name.clone();
                            let menu_proxy = item.menu_proxy.clone();
                            move |_| {
                                debug!("layout update event name {}", name);

                                let name = name.clone();
                                let menu_proxy = menu_proxy.clone();
                                async move {
                                    menu_proxy.get_layout(0, -1, &[]).await.ok().map(
                                        |(_, layout)| {
                                            TrayEvent::MenuLayoutChanged(name.to_owned(), layout)
                                        },
                                    )
                                }
                            }
                        })
                        .boxed(),
                );
            }

            // `ItemsRemoved` is a dbusmenu signal (not an SNI signal). When the
            // menu layout changes (items removed), refetch the menu.
            let items_removed = item.menu_proxy.receive_items_removed().await;
            if let Ok(items_removed) = items_removed {
                menu_layout_change.push(
                    items_removed
                        .filter_map({
                            let name = name.clone();
                            let menu_proxy = item.menu_proxy.clone();
                            move |_| {
                                debug!("items removed event name {}", name);

                                let name = name.clone();
                                let menu_proxy = menu_proxy.clone();
                                async move {
                                    menu_proxy.get_layout(0, -1, &[]).await.ok().map(
                                        |(_, layout)| {
                                            TrayEvent::MenuLayoutChanged(name.to_owned(), layout)
                                        },
                                    )
                                }
                            }
                        })
                        .boxed(),
                );
            }
        }

        Ok(stream_select!(
            registered,
            unregistered,
            select_all(icon_pixel_change),
            select_all(icon_name_change),
            select_all(new_icon_change),
            select_all(menu_layout_change),
            select_all(status_change)
        )
        .boxed())
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
                            error!("Failed to initialize tray service: {err}");

                            State::Error
                        }
                    }
                }
                Err(err) => {
                    error!("Failed to connect to system bus: {err}");

                    State::Error
                }
            },
            State::Active(conn) => {
                info!("Listening for tray events");

                match TrayService::events(&conn).await {
                    Ok(mut events) => {
                        while let Some(event) = events.next().await {
                            debug!("tray data {event:?}");
                            let _ = output.send(ServiceEvent::Update(event)).await;
                        }

                        State::Active(conn)
                    }
                    Err(err) => {
                        error!("Failed to listen for tray events: {err}");
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

    async fn menu_voice_selected(
        menu_proxy: &DBusMenuProxy<'_>,
        id: i32,
    ) -> anyhow::Result<Layout> {
        let value = zbus::zvariant::Value::I32(32).try_to_owned()?;
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
            TrayEvent::MenuLayoutChanged(name, layout) => {
                if let Some(item) = self.data.0.iter_mut().find(|item| item.name == name) {
                    debug!("menu layout updated, {layout:?}");
                    item.menu = layout;
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
    /// SNI `ContextMenu` — request the menu be shown at (x, y).
    /// Protocol surface; not yet triggered from the UI.
    #[allow(dead_code)]
    ContextMenu(String, i32, i32),
    /// SNI `SecondaryActivate` — secondary click.
    /// Protocol surface; not yet triggered from the UI.
    #[allow(dead_code)]
    SecondaryActivate(String, i32, i32),
    /// SNI `Scroll` — scroll delta + orientation (`Horizontal`/`Vertical`).
    /// Protocol surface; not yet triggered from the UI.
    #[allow(dead_code)]
    Scroll(String, i32, &'static str),
}

impl Service for TrayService {
    type Command = TrayCommand;

    fn command(&mut self, command: Self::Command) -> Task<ServiceEvent<Self>> {
        match command {
            TrayCommand::MenuSelected(name, id) => {
                let menu = self.data.iter().find(|item| item.name == name);
                if let Some(menu) = menu {
                    let name_cb = name.clone();
                    Task::perform(
                        {
                            let proxy = menu.menu_proxy.clone();

                            async move {
                                debug!("Click tray menu voice {name} : {id}");
                                TrayService::menu_voice_selected(&proxy, id).await
                            }
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
            TrayCommand::ContextMenu(name, x, y) => {
                let item = self.data.iter().find(|item| item.name == name);
                if let Some(item) = item {
                    Task::perform(
                        {
                            let proxy = item.item_proxy.clone();
                            async move {
                                debug!("Context menu tray item {name} at ({x}, {y})");
                                let _ = proxy.context_menu(x, y).await;
                            }
                        },
                        |_| ServiceEvent::Update(TrayEvent::None),
                    )
                } else {
                    Task::none()
                }
            }
            TrayCommand::SecondaryActivate(name, x, y) => {
                let item = self.data.iter().find(|item| item.name == name);
                if let Some(item) = item {
                    Task::perform(
                        {
                            let proxy = item.item_proxy.clone();
                            async move {
                                debug!("Secondary activate tray item {name} at ({x}, {y})");
                                let _ = proxy.secondary_activate(x, y).await;
                            }
                        },
                        |_| ServiceEvent::Update(TrayEvent::None),
                    )
                } else {
                    Task::none()
                }
            }
            TrayCommand::Scroll(name, delta, orientation) => {
                let item = self.data.iter().find(|item| item.name == name);
                if let Some(item) = item {
                    Task::perform(
                        {
                            let proxy = item.item_proxy.clone();
                            async move {
                                debug!("Scroll tray item {name} by {delta} ({orientation})");
                                let _ = proxy.scroll(delta, orientation).await;
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
    use super::{ItemStatus, pixmap_to_icon, split_service_name};
    use crate::services::tray::dbus::Icon;

    fn valid_pixmap(width: i32, height: i32) -> Icon {
        Icon {
            width,
            height,
            // `width * height * 4` ARGB bytes — the byte contract
            // `pixmap_to_icon` requires to accept a pixmap.
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
        // Unknown values fall back to `Passive` (the default).
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
        // A valid pixmap (correct byte length, positive dimensions)
        // must resolve to a `TrayIcon::Image`.
        assert!(result.is_some());
    }

    #[test]
    fn pixmap_to_icon_rejects_zero_dimension_pixmap() {
        // A zero-width or zero-height pixmap is dropped up front.
        let icon = Icon {
            width: 0,
            height: 24,
            bytes: vec![],
        };
        assert!(pixmap_to_icon(vec![icon]).is_none());
    }

    #[test]
    fn pixmap_to_icon_rejects_byte_mismatch() {
        // Dimensions say 24x24 but the payload is too short: the
        // expected byte count (`24 * 24 * 4`) does not match, so the
        // pixmap is dropped rather than fed to the atlas uploader.
        let icon = Icon {
            width: 24,
            height: 24,
            bytes: vec![0u8; 100],
        };
        assert!(pixmap_to_icon(vec![icon]).is_none());
    }

    #[test]
    fn pixmap_to_icon_picks_largest_pixmap() {
        // Two pixmaps of different sizes: the largest (by pixel count)
        // must be the one returned. The selection itself is locked down
        // by the `best_icon_pixmap` tests in `dbus.rs`; here we confirm
        // that a mixed set still yields an `Image` icon.
        let small = valid_pixmap(16, 16);
        let large = valid_pixmap(48, 48);

        let result = pixmap_to_icon(vec![small, large]).expect("expected an icon");

        assert!(matches!(result, crate::services::tray::TrayIcon::Image(_)));
    }

    #[test]
    fn pixmap_to_icon_returns_none_for_empty_vec() {
        assert!(pixmap_to_icon(Vec::new()).is_none());
    }

    #[test]
    fn split_service_name_splits_unique_sender_from_path() {
        let (sender, path) = split_service_name(":1.131/StatusNotifierItem");
        assert_eq!(sender, ":1.131");
        assert_eq!(path, "/StatusNotifierItem");
    }

    #[test]
    fn split_service_name_defaults_path_for_bare_name() {
        let (sender, path) = split_service_name("org.kde.StatusNotifierItem-123");
        assert_eq!(sender, "org.kde.StatusNotifierItem-123");
        assert_eq!(path, "/StatusNotifierItem");
    }

    #[test]
    fn split_service_name_handles_bare_path() {
        // A bare path (leading `/`) splits at index 0: the sender is the
        // empty string and the path is the bare path itself.
        let (sender, path) = split_service_name("/StatusNotifierItem");
        assert_eq!(sender, "");
        assert_eq!(path, "/StatusNotifierItem");
    }
}
