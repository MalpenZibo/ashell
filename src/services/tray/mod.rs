//! Tray service: hosts the StatusNotifierWatcher, tracks registered items
//! (icon + DBusMenu layout) and pushes them into a reactive signal.
//!
//! The service task owns the D-Bus proxies; the UI only ever sees plain
//! `TrayItem` data through the signal and talks back via `TrayCmd`.

pub mod dbus;

use std::collections::HashMap;
use std::time::Duration;

use futures::stream::{BoxStream, StreamExt, select_all};
use guido::prelude::*;
use log::{debug, error, info, warn};

use dbus::{
    DBusMenuProxy, Layout, StatusNotifierItemProxy, StatusNotifierWatcher,
    StatusNotifierWatcherProxy,
};

use super::xdg_icons::{self, XdgIcon};

#[derive(Debug, Clone, PartialEq)]
pub struct TrayItem {
    pub name: String,
    pub icon: Option<XdgIcon>,
    pub menu: Layout,
}

#[derive(Clone, Debug)]
pub enum TrayCmd {
    /// Ask the application to activate itself (primary action).
    /// Unwired until containers grow right-click support.
    #[allow(dead_code)]
    Activate(String),
    /// Send a "clicked" event for a menu entry, then refresh the layout
    MenuClick(String, i32),
    /// Announce a submenu is about to open; lazy apps populate its children
    /// then (a returned/true refresh lands via LayoutUpdated or the refetch)
    AboutToShow(String, i32),
}

enum TrayEvent {
    /// A new item appeared — triggers a full rebuild of items and streams
    Registered,
    Unregistered(String),
    IconChanged(String, XdgIcon),
    MenuLayoutChanged(String, Layout),
}

struct ItemProxies {
    item: StatusNotifierItemProxy<'static>,
    menu: DBusMenuProxy<'static>,
}

fn split_service_name(name: &str) -> (&str, &str) {
    match name.find('/') {
        Some(idx) => (&name[..idx], &name[idx..]),
        None => (name, "/StatusNotifierItem"),
    }
}

fn pixmap_to_icon(icons: Vec<dbus::Icon>) -> Option<XdgIcon> {
    icons
        .into_iter()
        .filter(|i| {
            // SNI clients sometimes return entries with zero dimensions or a
            // bytes payload that doesn't match width*height*4 (e.g. when only
            // IconName is populated). Drop them up front.
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
        .max_by_key(|i| (i.width, i.height))
        .map(|mut i| {
            // Convert ARGB to RGBA
            for pixel in i.bytes.chunks_exact_mut(4) {
                pixel.rotate_left(1);
            }
            XdgIcon::Image(ImageSource::Rgba {
                width: i.width as u32,
                height: i.height as u32,
                pixels: i.bytes.into(),
            })
        })
}

async fn current_icon_from_proxy(item_proxy: &StatusNotifierItemProxy<'_>) -> Option<XdgIcon> {
    match item_proxy.icon_pixmap().await.ok().and_then(pixmap_to_icon) {
        Some(icon) => Some(icon),
        None => item_proxy
            .icon_name()
            .await
            .ok()
            .as_deref()
            .and_then(xdg_icons::get_icon_from_name),
    }
}

async fn build_item(
    conn: &zbus::Connection,
    name: &str,
) -> anyhow::Result<(TrayItem, ItemProxies)> {
    let (dest, path) = split_service_name(name);

    let item_proxy = StatusNotifierItemProxy::builder(conn)
        .destination(dest.to_owned())?
        .path(path.to_owned())?
        .build()
        .await?;

    let icon = current_icon_from_proxy(&item_proxy).await;

    let menu_path = item_proxy.menu().await?;
    let menu_proxy = DBusMenuProxy::builder(conn)
        .destination(dest.to_owned())?
        .path(menu_path.to_owned())?
        .build()
        .await?;

    let (_, menu) = menu_proxy.get_layout(0, -1, &[]).await?;

    Ok((
        TrayItem {
            name: name.to_owned(),
            icon,
            menu,
        },
        ItemProxies {
            item: item_proxy,
            menu: menu_proxy,
        },
    ))
}

/// Per-item change streams: icon updates (three signal flavors) and menu
/// layout updates, each mapped to a TrayEvent.
async fn item_event_streams(
    conn: &zbus::Connection,
    name: &str,
    proxies: &ItemProxies,
    streams: &mut Vec<BoxStream<'static, TrayEvent>>,
) -> anyhow::Result<()> {
    streams.push(
        proxies
            .item
            .receive_icon_pixmap_changed()
            .await
            .filter_map({
                let name = name.to_owned();
                move |icon| {
                    let name = name.clone();
                    async move {
                        let icons = icon.get().await.ok()?;
                        pixmap_to_icon(icons).map(|icon| TrayEvent::IconChanged(name, icon))
                    }
                }
            })
            .boxed(),
    );

    streams.push(
        proxies
            .item
            .receive_icon_name_changed()
            .await
            .filter_map({
                let name = name.to_owned();
                move |icon_name| {
                    let name = name.clone();
                    async move {
                        icon_name
                            .get()
                            .await
                            .ok()
                            .as_deref()
                            .and_then(xdg_icons::get_icon_from_name)
                            .map(|icon| TrayEvent::IconChanged(name, icon))
                    }
                }
            })
            .boxed(),
    );

    if let Ok(new_icon) = proxies.item.receive_new_icon().await {
        // NewIcon has no matching PropertiesChanged, so a cached read would
        // be stale — use an uncached proxy for the re-read.
        let (dest, path) = split_service_name(name);
        let uncached_proxy = StatusNotifierItemProxy::builder(conn)
            .destination(dest.to_owned())?
            .path(path.to_owned())?
            .cache_properties(zbus::proxy::CacheProperties::No)
            .build()
            .await?;

        streams.push(
            new_icon
                .filter_map({
                    let name = name.to_owned();
                    move |_| {
                        let name = name.clone();
                        let uncached_proxy = uncached_proxy.clone();
                        async move {
                            current_icon_from_proxy(&uncached_proxy)
                                .await
                                .map(|icon| TrayEvent::IconChanged(name, icon))
                        }
                    }
                })
                .boxed(),
        );
    }

    if let Ok(layout_updated) = proxies.menu.receive_layout_updated().await {
        streams.push(
            layout_updated
                .filter_map({
                    let name = name.to_owned();
                    let menu_proxy = proxies.menu.clone();
                    move |_| {
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
                })
                .boxed(),
        );
    }

    Ok(())
}

/// Send a "clicked" event to a menu entry and re-fetch the layout.
async fn menu_click(menu_proxy: &DBusMenuProxy<'_>, id: i32) -> anyhow::Result<Layout> {
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

pub fn start_tray_service(writer: WriteSignal<Vec<TrayItem>>) -> Service<TrayCmd> {
    create_service::<TrayCmd, _, _>(move |mut rx, ctx| async move {
        // Icon-theme scan and .desktop index are filesystem-heavy: build them
        // once on a blocking worker before any icon lookup needs them.
        xdg_icons::warm_cache_async().await;

        let conn = loop {
            match StatusNotifierWatcher::start_server().await {
                Ok(conn) => break conn,
                Err(err) => {
                    error!("Failed to start StatusNotifierWatcher: {err}");
                    for _ in 0..30 {
                        if !ctx.is_running() {
                            return;
                        }
                        tokio::time::sleep(Duration::from_secs(1)).await;
                    }
                }
            }
        };
        info!("Tray service initialized");

        // Rebuilt from scratch whenever a new item registers (streams must
        // be re-created to cover it, mirroring ashell's behavior).
        'rebuild: while ctx.is_running() {
            let watcher = match StatusNotifierWatcherProxy::new(&conn).await {
                Ok(w) => w,
                Err(err) => {
                    error!("Failed to connect to StatusNotifierWatcher: {err}");
                    return;
                }
            };
            let names = watcher
                .registered_status_notifier_items()
                .await
                .unwrap_or_default();

            let mut items: Vec<TrayItem> = Vec::with_capacity(names.len());
            let mut proxies: HashMap<String, ItemProxies> = HashMap::new();
            let mut streams: Vec<BoxStream<'static, TrayEvent>> = Vec::new();

            match watcher.receive_status_notifier_item_registered().await {
                Ok(s) => streams.push(
                    s.filter_map(|e| async move { e.args().ok().map(|_| TrayEvent::Registered) })
                        .boxed(),
                ),
                Err(err) => {
                    error!("Failed to listen for tray registrations: {err}");
                    return;
                }
            }
            if let Ok(s) = watcher.receive_status_notifier_item_unregistered().await {
                streams.push(
                    s.filter_map(|e| async move {
                        e.args()
                            .ok()
                            .map(|args| TrayEvent::Unregistered(args.service.to_string()))
                    })
                    .boxed(),
                );
            }

            for name in &names {
                match build_item(&conn, name).await {
                    Ok((item, px)) => {
                        if let Err(err) = item_event_streams(&conn, name, &px, &mut streams).await {
                            warn!("Failed to watch tray item {name}: {err}");
                        }
                        items.push(item);
                        proxies.insert(name.clone(), px);
                    }
                    Err(err) => warn!("Failed to read tray item {name}: {err}"),
                }
            }

            writer.set(items.clone());
            let mut events = select_all(streams);

            while ctx.is_running() {
                tokio::select! {
                    cmd = rx.recv() => match cmd {
                        Some(TrayCmd::Activate(name)) => {
                            if let Some(px) = proxies.get(&name) {
                                debug!("Activate tray item {name}");
                                let _ = px.item.activate(0, 0).await;
                            }
                        }
                        Some(TrayCmd::MenuClick(name, id)) => {
                            if let Some(px) = proxies.get(&name) {
                                debug!("Tray menu click {name}: {id}");
                                match menu_click(&px.menu, id).await {
                                    Ok(layout) => {
                                        if let Some(item) =
                                            items.iter_mut().find(|i| i.name == name)
                                        {
                                            item.menu = layout;
                                            writer.set(items.clone());
                                        }
                                    }
                                    Err(err) => debug!("Tray menu click failed: {err}"),
                                }
                            }
                        }
                        Some(TrayCmd::AboutToShow(name, id)) => {
                            if let Some(px) = proxies.get(&name) {
                                // true = the layout changed; refetch in case
                                // the app doesn't also emit LayoutUpdated
                                if let Ok(true) = px.menu.about_to_show(id).await
                                    && let Ok((_, layout)) = px.menu.get_layout(0, -1, &[]).await
                                    && let Some(item) = items.iter_mut().find(|i| i.name == name)
                                {
                                    item.menu = layout;
                                    writer.set(items.clone());
                                }
                            }
                        }
                        None => break 'rebuild,
                    },
                    ev = events.next() => match ev {
                        Some(TrayEvent::Registered) => continue 'rebuild,
                        Some(TrayEvent::Unregistered(name)) => {
                            debug!("Tray item unregistered: {name}");
                            items.retain(|i| i.name != name);
                            proxies.remove(&name);
                            writer.set(items.clone());
                        }
                        Some(TrayEvent::IconChanged(name, icon)) => {
                            if let Some(item) = items.iter_mut().find(|i| i.name == name) {
                                item.icon = Some(icon);
                                writer.set(items.clone());
                            }
                        }
                        Some(TrayEvent::MenuLayoutChanged(name, layout)) => {
                            if let Some(item) = items.iter_mut().find(|i| i.name == name) {
                                item.menu = layout;
                                writer.set(items.clone());
                            }
                        }
                        // All streams ended — should not happen while the
                        // watcher streams are alive; back off and rebuild.
                        None => {
                            tokio::time::sleep(Duration::from_secs(1)).await;
                            continue 'rebuild;
                        }
                    },
                }
            }
        }
    })
}
