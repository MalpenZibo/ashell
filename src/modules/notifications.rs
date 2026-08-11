use std::cell::RefCell;
use std::collections::{HashMap, HashSet, VecDeque};
use std::rc::Rc;
use std::time::Duration;

use guido::prelude::*;
use itertools::Itertools;

use crate::components::{ButtonKind, ButtonSize, StaticIcon, buttons::icon_button, icon};
use crate::config::{Config, NotificationsModuleConfig, ToastPosition};
use crate::services::compat::{ServiceEvent, ServiceSignal, run_readonly_service_hooked, svg};
use crate::services::notifications::dbus::{NotificationDaemon, NotificationEvent};
use crate::services::notifications::{
    Notification, NotificationIcon, NotificationsService, Urgency,
};
use crate::theme::ThemeColors;

const TOAST_WIDTH: u32 = 360;
const CARD_ICON_SIZE: f32 = 36.0;

#[derive(Clone, Copy)]
pub struct NotificationsHandle {
    pub data: ServiceSignal<NotificationsService>,
    /// The notification list, newest first (`set_always`: upstream's
    /// `Notification` has no equality).
    pub list: RwSignal<VecDeque<Notification>>,
    /// Ids of the toasts currently on screen, oldest first.
    pub toasts: RwSignal<Vec<u32>>,
    expanded_groups: RwSignal<HashSet<String>>,
}

/// Toast timeout per the freedesktop spec: critical urgency never expires,
/// expire_timeout -1 uses the config default, 0 never expires.
fn toast_timeout_ms(n: &Notification, config_ms: u64) -> Option<u64> {
    if n.urgency == Urgency::Critical {
        return None;
    }
    match n.expire_timeout {
        -1 => Some(config_ms),
        0 => None,
        ms => Some(ms as u64),
    }
}

pub fn create() -> NotificationsHandle {
    let config = with_context::<Config, _>(|c| c.notifications.clone()).unwrap_or_default();

    let list = create_signal(VecDeque::<Notification>::new());
    let toasts = create_signal(Vec::<u32>::new());
    let list_w = list.writer();
    let toasts_w = toasts.writer();

    // Raw daemon events cross from the service hook into the manager task
    let (ev_tx, mut ev_rx) = tokio::sync::mpsc::unbounded_channel::<NotificationEvent>();
    let data = run_readonly_service_hooked::<NotificationsService>(move |ev| {
        if let ServiceEvent::Update(ev) = ev {
            let _ = ev_tx.send(ev.clone());
        }
    });

    let blocklist = config.blocklist.clone();
    let toast_enabled = config.toast;
    let toast_default_ms = config.toast_timeout;
    let limit = config.toast_limit;

    // The manager task owns the whole timeline: the list fold, the toast
    // queue, and the expiry deadlines. ONE sleeper on the earliest
    // deadline, recomputed on every change — a re-shown notification just
    // gets a new deadline, so stale timers cannot exist and the UI renders
    // pure state.
    let _mgr = create_service::<(), _, _>(move |_rx, _ctx| async move {
        use tokio::time::{Instant, sleep_until};
        let mut items: VecDeque<Notification> = VecDeque::new();
        let mut live: Vec<u32> = Vec::new();
        let mut deadlines: HashMap<u32, Instant> = HashMap::new();

        loop {
            let next = deadlines.values().min().copied();
            tokio::select! {
                ev = ev_rx.recv() => {
                    let Some(ev) = ev else { break };
                    match ev {
                        NotificationEvent::Received(n) => {
                            if blocklist.iter().any(|re| re.0.is_match(&n.app_name)) {
                                continue;
                            }
                            let id = n.id;
                            items.retain(|x| x.id != id);
                            items.push_front((*n).clone());
                            list_w.set_always(items.clone());

                            if toast_enabled {
                                live.retain(|&x| x != id);
                                deadlines.remove(&id);
                                if limit == 0 {
                                    live.clear();
                                    deadlines.clear();
                                } else {
                                    while live.len() >= limit {
                                        let dropped = live.remove(0);
                                        deadlines.remove(&dropped);
                                    }
                                    live.push(id);
                                    if let Some(ms) = toast_timeout_ms(&n, toast_default_ms) {
                                        deadlines.insert(
                                            id,
                                            Instant::now() + Duration::from_millis(ms),
                                        );
                                    }
                                }
                                toasts_w.set(live.clone());
                            }
                        }
                        NotificationEvent::Closed(id) => {
                            items.retain(|x| x.id != id);
                            list_w.set_always(items.clone());
                            live.retain(|&x| x != id);
                            deadlines.remove(&id);
                            toasts_w.set(live.clone());
                        }
                    }
                }
                _ = async {
                    match next {
                        Some(d) => sleep_until(d).await,
                        None => std::future::pending().await,
                    }
                } => {
                    let now = Instant::now();
                    let expired: Vec<u32> = deadlines
                        .iter()
                        .filter(|(_, d)| **d <= now)
                        .map(|(id, _)| *id)
                        .collect();
                    for id in expired {
                        deadlines.remove(&id);
                        live.retain(|&x| x != id);
                    }
                    toasts_w.set(live.clone());
                }
            }
        }
    });

    let handle = NotificationsHandle {
        data,
        list,
        toasts,
        expanded_groups: create_signal(HashSet::new()),
    };

    if config.toast {
        spawn_toast_layer(handle, config);
    }

    handle
}

// ── Toast layer surface ──────────────────────────────────────────────────────

fn spawn_toast_layer(handle: NotificationsHandle, config: NotificationsModuleConfig) {
    let theme = expect_context::<ThemeColors>();
    let anchor = match config.toast_position {
        ToastPosition::TopLeft => Anchor::TOP | Anchor::LEFT,
        ToastPosition::TopRight => Anchor::TOP | Anchor::RIGHT,
        ToastPosition::BottomLeft => Anchor::BOTTOM | Anchor::LEFT,
        ToastPosition::BottomRight => Anchor::BOTTOM | Anchor::RIGHT,
    };

    let surface: Rc<RefCell<Option<SurfaceHandle>>> = Rc::new(RefCell::new(None));

    // Surface exists only while at least one toast is queued; auto_height
    // follows the stack's natural size (measured by guido, no rect
    // round-trip)
    let slot = surface.clone();
    let view_config = config.clone();
    create_effect(move || {
        let has_toasts = handle.toasts.with(|t| !t.is_empty());
        let mut slot_ref = slot.borrow_mut();
        if has_toasts && slot_ref.is_none() {
            let view_config = view_config.clone();
            *slot_ref = Some(spawn_surface(
                SurfaceConfig::new()
                    .width(TOAST_WIDTH)
                    .height(content())
                    .anchor(anchor)
                    .layer(Layer::Overlay)
                    .margin(8, 8, 8, 8)
                    .keyboard_interactivity(KeyboardInteractivity::None)
                    .background_color(Color::TRANSPARENT)
                    .namespace("ashell-toast"),
                move || toast_view(handle, view_config.clone(), theme),
            ));
        } else if !has_toasts && let Some(h) = slot_ref.take() {
            h.close();
        }
    })
    .detach();
}

fn toast_view(
    handle: NotificationsHandle,
    config: NotificationsModuleConfig,
    theme: ThemeColors,
) -> impl Widget {
    container()
        .width(fill())
        .layout(Flex::column().spacing(8))
        .children(move || {
            let ids = handle.toasts.get();
            handle.list.with(|l| {
                ids.iter()
                    .filter_map(|id| l.iter().find(|n| n.id == *id))
                    .map(|n| {
                        notification_card(handle, n, &config, theme, true)
                            .corner_radius(16)
                            .background(theme.background)
                    })
                    .collect::<Vec<_>>()
            })
        })
}

// ── Cards ────────────────────────────────────────────────────────────────────

fn notification_icon_source(icon: &NotificationIcon) -> ImageSource {
    match icon {
        NotificationIcon::Svg(svg::Handle(path)) => ImageSource::SvgPath(path.clone()),
        NotificationIcon::Image(crate::services::compat::image::Handle::Path(p)) => {
            ImageSource::Path(p.clone())
        }
        NotificationIcon::Image(crate::services::compat::image::Handle::Bytes(b)) => {
            ImageSource::Bytes(std::sync::Arc::from(b.as_ref()))
        }
    }
}

fn close_by_id(handle: NotificationsHandle, id: u32) {
    let conn = handle
        .data
        .with(|s| s.as_ref().map(|x| x.connection.clone()));
    if let Some(conn) = conn {
        tokio::spawn(async move {
            if let Err(e) = NotificationDaemon::close_notification_by_id(&conn, id).await {
                log::error!("Failed to close notification id {id}: {e}");
            }
        });
    }
}

fn invoke_and_close(handle: NotificationsHandle, id: u32, action_key: Option<String>) {
    let conn = handle
        .data
        .with(|s| s.as_ref().map(|x| x.connection.clone()));
    if let Some(conn) = conn {
        tokio::spawn(async move {
            if let Some(key) = action_key
                && let Err(e) = NotificationDaemon::invoke_action(&conn, id, key).await
            {
                log::error!("Failed to invoke notification action for id {id}: {e}");
            }
            if let Err(e) = NotificationDaemon::close_notification_by_id(&conn, id).await {
                log::error!("Failed to close notification id {id}: {e}");
            }
        });
    }
}

fn first_action(n: &Notification) -> Option<String> {
    (!n.actions.is_empty())
        .then(|| n.actions.first().cloned())
        .flatten()
}

fn notification_card(
    handle: NotificationsHandle,
    n: &Notification,
    config: &NotificationsModuleConfig,
    theme: ThemeColors,
    toast: bool,
) -> Container {
    let id = n.id;
    let action = first_action(n);
    let critical = n.urgency == Urgency::Critical;

    let timestamp = config.show_timestamps.then(|| {
        let dt: chrono::DateTime<chrono::Local> = n.timestamp.into();
        dt.format(&config.format).to_string()
    });

    let header = container()
        .width(fill())
        .layout(
            Flex::row()
                .spacing(8)
                .cross_alignment(CrossAlignment::Center),
        )
        .maybe_child(n.icon.as_ref().map(|ic| {
            image(notification_icon_source(ic))
                .width(CARD_ICON_SIZE)
                .height(CARD_ICON_SIZE)
        }))
        .child(
            container()
                .width(fill())
                .child(text(n.app_name.clone()).color(theme.text).font_size(12)),
        )
        .maybe_child(timestamp.map(|ts| text(ts).color(theme.text).font_size(11)))
        .child(
            icon_button()
                .icon(StaticIcon::Close)
                .size(ButtonSize::Small)
                .kind(ButtonKind::Transparent)
                .on_click(move || close_by_id(handle, id)),
        );

    let show_body = !toast || config.show_bodies;

    let mut card = container()
        .width(fill())
        .padding(12)
        .layout(Flex::column().spacing(4))
        .child(header)
        .child(text(n.summary.clone()).color(theme.text).font_size(13));
    if show_body && !n.body.is_empty() {
        card = card.child(text(n.body.clone()).color(theme.text).font_size(12));
    }
    if critical {
        card = card.border(1, theme.danger);
    }
    if toast {
        card = card
            .height(at_most(config.toast_max_height as f32))
            .overflow(Overflow::Hidden)
            // Clicking a toast invokes its default action and dismisses it
            .on_click(move || invoke_and_close(handle, id, action.clone()));
    }
    card
}

// ── Bar + menu ───────────────────────────────────────────────────────────────

pub fn view(handle: NotificationsHandle) -> impl Widget {
    let theme = expect_context::<ThemeColors>();
    let has_notifications = create_memo(move || handle.list.with(|l| !l.is_empty()));

    container().child(move || {
        Some(
            icon()
                .kind(if has_notifications.get() {
                    StaticIcon::BellBadge
                } else {
                    StaticIcon::Bell
                })
                .color(theme.text),
        )
    })
}

pub fn menu_view(handle: NotificationsHandle) -> impl Widget {
    let theme = expect_context::<ThemeColors>();
    let config = with_context::<Config, _>(|c| c.notifications.clone()).unwrap_or_default();

    container()
        .width(fill())
        .layout(Flex::column().spacing(8))
        .child(
            container()
                .width(fill())
                .layout(
                    Flex::row()
                        .cross_alignment(CrossAlignment::Center)
                        .main_alignment(MainAlignment::SpaceBetween),
                )
                .child(text("Notifications").color(theme.text).font_size(16))
                .child(container().child(move || {
                    let has = handle.list.with(|l| !l.is_empty());
                    has.then(|| {
                        icon_button()
                            .icon(StaticIcon::Delete)
                            .size(ButtonSize::Small)
                            .kind(ButtonKind::Transparent)
                            .on_click(move || {
                                let ids: Vec<u32> =
                                    handle.list.with(|l| l.iter().map(|n| n.id).collect());
                                for id in ids {
                                    close_by_id(handle, id);
                                }
                            })
                    })
                })),
        )
        .child(crate::components::divider())
        .child(move || {
            let config = config.clone();
            if handle.list.with(|l| l.is_empty()) {
                return Some(
                    container()
                        .width(fill())
                        .padding(24)
                        .layout(Flex::row().main_alignment(MainAlignment::Center))
                        .child(text("No notifications").color(theme.text).font_size(13))
                        .into_any(),
                );
            }

            let mut col = container()
                .width(fill())
                .height(at_most(400))
                .scrollable(ScrollAxis::Vertical)
                .layout(Flex::column().spacing(8));

            if config.grouped {
                col = handle.list.with(|l| {
                    let mut col = col;
                    let expanded = handle.expanded_groups.get();
                    let groups = l
                        .iter()
                        .sorted_by(|a, b| a.app_name.cmp(&b.app_name))
                        .chunk_by(|n| n.app_name.clone());
                    for (app_name, group) in &groups {
                        let group: Vec<&Notification> = group.collect();
                        if group.len() == 1 {
                            col = col.child(
                                notification_card(handle, group[0], &config, theme, false)
                                    .corner_radius(12)
                                    .background(theme.background.lighter(0.05)),
                            );
                            continue;
                        }
                        let is_expanded = expanded.contains(&app_name);
                        let ids: Vec<u32> = group.iter().map(|n| n.id).collect();
                        let toggle_name = app_name.clone();
                        let clear_ids = ids.clone();
                        // Group header: name + count, delete, expand toggle
                        col = col.child(
                            container()
                                .width(fill())
                                .padding([4, 8])
                                .corner_radius(12)
                                .background(theme.background.lighter(0.1))
                                .hover_state(|s| s.lighter(0.05))
                                .on_click(move || {
                                    handle.expanded_groups.update(|e| {
                                        if !e.remove(&toggle_name) {
                                            e.insert(toggle_name.clone());
                                        }
                                    });
                                })
                                .layout(
                                    Flex::row()
                                        .spacing(8)
                                        .cross_alignment(CrossAlignment::Center)
                                        .main_alignment(MainAlignment::SpaceBetween),
                                )
                                .child(
                                    text(format!("{} ({})", app_name, group.len()))
                                        .color(theme.text)
                                        .font_size(12),
                                )
                                .child(
                                    icon_button()
                                        .icon(StaticIcon::Delete)
                                        .size(ButtonSize::Small)
                                        .kind(ButtonKind::Transparent)
                                        .on_click(move || {
                                            for id in clear_ids.clone() {
                                                close_by_id(handle, id);
                                            }
                                        }),
                                ),
                        );
                        let visible: Vec<&Notification> = if is_expanded {
                            group
                        } else {
                            group.into_iter().take(1).collect()
                        };
                        for n in visible {
                            col = col.child(
                                notification_card(handle, n, &config, theme, false)
                                    .corner_radius(6)
                                    .background(theme.background.lighter(0.05)),
                            );
                        }
                    }
                    col
                });
            } else {
                col = handle.list.with(|l| {
                    let mut col = col;
                    for n in l {
                        col = col.child(
                            notification_card(handle, n, &config, theme, false)
                                .corner_radius(12)
                                .background(theme.background.lighter(0.05)),
                        );
                    }
                    col
                });
            }

            Some(col.into_any())
        })
}
