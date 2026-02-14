use super::{ReadOnlyService, Service, ServiceEvent};
use dbus::{
    DBusMenuProxy, Layout, StatusNotifierItemProxy, StatusNotifierWatcher,
    StatusNotifierWatcherProxy,
};
use freedesktop_icons::lookup;
use iced::{
    Subscription, Task,
    futures::{
        SinkExt, Stream, StreamExt,
        channel::mpsc::Sender,
        stream::{pending, select_all},
        stream_select,
    },
    stream::channel,
    widget::{image, svg},
};
use linicon_theme::get_icon_theme;
use log::{debug, error, info, trace};
use std::{
    any::TypeId,
    collections::BTreeSet,
    env, fs,
    ops::Deref,
    path::{Path, PathBuf},
    sync::LazyLock,
};

pub mod dbus;

static SYSTEM_ICON_NAMES: LazyLock<BTreeSet<String>> = LazyLock::new(load_system_icon_names);
static SYSTEM_ICON_ENTRIES: LazyLock<Vec<(String, String)>> = LazyLock::new(|| {
    SYSTEM_ICON_NAMES
        .iter()
        .map(|name| (name.clone(), normalize_icon_name(name)))
        .collect()
});

fn get_icon_from_name(icon_name: &str) -> Option<TrayIcon> {
    if let Some(path) = find_icon_path(icon_name) {
        return tray_icon_from_path(path);
    }

    if let Some(candidates) = similar_icon_names(icon_name) {
        for candidate in candidates {
            if let Some(path) = find_icon_path(&candidate) {
                return tray_icon_from_path(path);
            }
        }
    }

    if let Some(prefix_candidate) = prefix_match_icon(icon_name)
        && let Some(path) = find_icon_path(&prefix_candidate)
    {
        return tray_icon_from_path(path);
    }

    None
}

fn tray_icon_from_path(path: PathBuf) -> Option<TrayIcon> {
    if path.extension().is_some_and(|ext| ext == "svg") {
        debug!("svg icon found. Path: {path:?}");

        Some(TrayIcon::Svg(svg::Handle::from_path(path)))
    } else {
        debug!("raster icon found. Path: {path:?}");

        Some(TrayIcon::Image(image::Handle::from_path(path)))
    }
}

fn find_icon_path(icon_name: &str) -> Option<PathBuf> {
    let base_lookup = lookup(icon_name).with_cache();

    match get_icon_theme() {
        Some(theme) => base_lookup.with_theme(&theme).find().or_else(|| {
            let fallback_lookup = lookup(icon_name).with_cache();
            fallback_lookup.find()
        }),
        None => base_lookup.find(),
    }
}

fn similar_icon_names(icon_name: &str) -> Option<Vec<String>> {
    if SYSTEM_ICON_NAMES.is_empty() {
        return None;
    }

    let normalized = normalize_icon_name(icon_name);
    let mut matches = Vec::new();

    for candidate in SYSTEM_ICON_NAMES.iter() {
        let candidate_normalized = normalize_icon_name(candidate);

        if candidate_normalized == normalized {
            continue;
        }

        if candidate_normalized.contains(&normalized)
            || normalized.contains(&candidate_normalized)
            || candidate_normalized.contains(&normalized.replace('-', ""))
        {
            matches.push(candidate.clone());
            if matches.len() >= 5 {
                break;
            }
        }
    }

    if matches.is_empty() {
        None
    } else {
        Some(matches)
    }
}

fn normalize_icon_name(name: &str) -> String {
    name.to_lowercase()
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect()
}

fn prefix_match_icon(icon_name: &str) -> Option<String> {
    if SYSTEM_ICON_ENTRIES.is_empty() {
        return None;
    }

    let normalized = normalize_icon_name(icon_name);
    let mut candidates: Vec<&(String, String)> = SYSTEM_ICON_ENTRIES.iter().collect();
    let chars: Vec<char> = normalized.chars().collect();

    for (idx, ch) in chars.iter().enumerate() {
        candidates.retain(|(_, name)| name.chars().nth(idx) == Some(*ch));

        if candidates.len() == 1 {
            return Some(candidates[0].0.clone());
        }

        if candidates.is_empty() {
            break;
        }
    }

    candidates.first().map(|(name, _)| name.clone())
}

fn load_system_icon_names() -> BTreeSet<String> {
    let mut names = BTreeSet::new();

    for dir in icon_directories() {
        if !dir.is_dir() {
            continue;
        }

        collect_icon_names_recursive(&dir, &mut names);
    }

    names
}

fn collect_icon_names_recursive(dir: &Path, names: &mut BTreeSet<String>) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Ok(file_type) = entry.file_type() {
                if file_type.is_dir() {
                    collect_icon_names_recursive(&path, names);
                } else if file_type.is_file()
                    && let Some(stem) = path.file_stem().and_then(|s| s.to_str())
                {
                    names.insert(stem.to_string());
                }
            }
        }
    }
}

fn icon_directories() -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    if let Ok(data_home) = env::var("XDG_DATA_HOME") {
        let base = PathBuf::from(data_home);
        dirs.push(base.join("icons"));
        dirs.push(base.join("pixmaps"));
    }

    if let Ok(home) = env::var("HOME") {
        let base = PathBuf::from(home);
        dirs.push(base.join(".local/share/icons"));
        dirs.push(base.join(".local/share/pixmaps"));
    }

    let data_dirs =
        env::var("XDG_DATA_DIRS").unwrap_or_else(|_| "/usr/local/share:/usr/share".into());
    for dir in data_dirs.split(':') {
        if dir.is_empty() {
            continue;
        }
        let base = PathBuf::from(dir);
        dirs.push(base.join("icons"));
        dirs.push(base.join("pixmaps"));
    }

    dirs.push(PathBuf::from("/usr/share/icons"));
    dirs.push(PathBuf::from("/usr/share/pixmaps"));

    dirs.sort();
    dirs.dedup();
    dirs
}

#[derive(Debug, Clone)]
pub enum TrayIcon {
    Image(image::Handle),
    Svg(svg::Handle),
}

#[derive(Debug, Clone)]
pub enum TrayEvent {
    Registered(StatusNotifierItem),
    IconChanged(String, TrayIcon),
    MenuLayoutChanged(String, Layout),
    Unregistered(String),
    None,
}

#[derive(Debug, Clone)]
pub struct StatusNotifierItem {
    pub name: String,
    pub icon: Option<TrayIcon>,
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

        debug!("item_proxy {item_proxy:?}");

        let icon_pixmap = item_proxy.icon_pixmap().await;

        let icon = match icon_pixmap {
            Ok(icons) => {
                icons
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
                        TrayIcon::Image(image::Handle::from_rgba(
                            i.width as u32,
                            i.height as u32,
                            i.bytes,
                        ))
                    })
            }
            Err(_) => item_proxy
                .icon_name()
                .await
                .ok()
                .as_deref()
                .and_then(get_icon_from_name),
        };

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

                                item.map(TrayEvent::Registered).ok()
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
        let mut menu_layout_change = Vec::with_capacity(items.len());

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
                                            TrayEvent::IconChanged(
                                                name.to_owned(),
                                                TrayIcon::Image(image::Handle::from_rgba(
                                                    i.width as u32,
                                                    i.height as u32,
                                                    i.bytes,
                                                )),
                                            )
                                        })
                                })
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
                                    .and_then(get_icon_from_name)
                                    .map(|icon| TrayEvent::IconChanged(name.to_owned(), icon))
                            }
                        }
                    })
                    .boxed(),
            );

            let layout_updated = item.menu_proxy.receive_layout_updated().await;
            if let Ok(layout_updated) = layout_updated {
                menu_layout_change.push(
                    layout_updated
                        .filter_map({
                            let name = name.clone();
                            let menu_proxy = item.menu_proxy.clone();
                            move |_| {
                                debug!("layout update event name {}", &name);

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
            select_all(menu_layout_change)
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

                            let reload_events = matches!(event, TrayEvent::Registered(_));

                            let _ = output.send(ServiceEvent::Update(event)).await;

                            if reload_events {
                                break;
                            }
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
            TrayEvent::Unregistered(name) => {
                self.data.0.retain(|item| item.name != name);
            }
            TrayEvent::None => {}
        }
    }

    fn subscribe() -> iced::Subscription<ServiceEvent<Self>> {
        let id = TypeId::of::<Self>();

        Subscription::run_with_id(
            id,
            channel(100, async |mut output| {
                let mut state = State::Init;

                loop {
                    state = TrayService::start_listening(state, &mut output).await;
                }
            }),
        )
    }
}

#[derive(Debug, Clone)]
pub enum TrayCommand {
    MenuSelected(String, i32),
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
        }
    }
}
