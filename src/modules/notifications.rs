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

/// Versioned snapshot of the notification list (folded on the service task,
/// published to the UI thread).
#[derive(Clone, Default)]
pub struct NotifList {
    version: u64,
    pub items: VecDeque<Notification>,
}

impl PartialEq for NotifList {
    fn eq(&self, other: &Self) -> bool {
        self.version == other.version
    }
}

#[derive(Clone)]
struct ReceivedToast {
    version: u64,
    id: u32,
    timeout_ms: Option<u64>,
}

impl PartialEq for ReceivedToast {
    fn eq(&self, other: &Self) -> bool {
        self.version == other.version
    }
}

#[derive(Clone, PartialEq)]
struct ExpiredToast {
    version: u64,
    id: u32,
}

#[derive(Clone, Copy)]
pub struct NotificationsHandle {
    pub data: ServiceSignal<NotificationsService>,
    pub list: RwSignal<NotifList>,
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

    let list = create_signal(NotifList::default());
    let received = create_signal(None::<ReceivedToast>);
    let list_w = list.writer();
    let recv_w = received.writer();

    let blocklist = config.blocklist.clone();
    let toast_enabled = config.toast;
    let toast_default_ms = config.toast_timeout;

    // The fold lives on the service task; the UI only sees snapshots.
    let mut items: VecDeque<Notification> = VecDeque::new();
    let mut version = 0u64;
    let data = run_readonly_service_hooked::<NotificationsService>(move |ev| {
        let ServiceEvent::Update(ev) = ev else {
            return;
        };
        match ev {
            NotificationEvent::Received(n) => {
                if blocklist.iter().any(|re| re.0.is_match(&n.app_name)) {
                    return;
                }
                items.retain(|x| x.id != n.id);
                items.push_front((**n).clone());
                version += 1;
                list_w.set(NotifList {
                    version,
                    items: items.clone(),
                });
                if toast_enabled {
                    recv_w.set(Some(ReceivedToast {
                        version,
                        id: n.id,
                        timeout_ms: toast_timeout_ms(n, toast_default_ms),
                    }));
                }
            }
            NotificationEvent::Closed(id) => {
                items.retain(|x| x.id != *id);
                version += 1;
                list_w.set(NotifList {
                    version,
                    items: items.clone(),
                });
            }
        }
    });

    let toasts = create_signal(Vec::<u32>::new());
    let handle = NotificationsHandle {
        data,
        list,
        toasts,
        expanded_groups: create_signal(HashSet::new()),
    };

    if config.toast {
        let expired = create_signal(None::<ExpiredToast>);
        let expired_w = expired.writer();
        // Latest toast generation per id, so a stale timer can't dismiss a
        // re-shown notification early.
        let generations: Rc<RefCell<HashMap<u32, u64>>> = Rc::new(RefCell::new(HashMap::new()));

        let limit = config.toast_limit;
        let gens = generations.clone();
        create_effect(move || {
            let Some(r) = received.get() else {
                return;
            };
            toasts.update(|t| {
                t.retain(|&x| x != r.id);
                if limit == 0 {
                    t.clear();
                    return;
                }
                while t.len() >= limit {
                    t.remove(0);
                }
                t.push(r.id);
            });
            if limit == 0 {
                return;
            }
            gens.borrow_mut().insert(r.id, r.version);
            if let Some(ms) = r.timeout_ms {
                let (version, id) = (r.version, r.id);
                tokio::spawn(async move {
                    tokio::time::sleep(Duration::from_millis(ms)).await;
                    expired_w.set(Some(ExpiredToast { version, id }));
                });
            }
        })
        .detach();

        let gens = generations.clone();
        create_effect(move || {
            if let Some(e) = expired.get()
                && gens.borrow().get(&e.id) == Some(&e.version)
            {
                toasts.update(|t| t.retain(|&x| x != e.id));
            }
        })
        .detach();

        // Toasts of notifications closed through the daemon disappear too
        create_effect(move || {
            let ids: HashSet<u32> = list.with(|l| l.items.iter().map(|n| n.id).collect());
            toasts.update(|t| t.retain(|x| ids.contains(x)));
        })
        .detach();

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
    let content_wr = create_widget_ref();

    // Surface exists only while at least one toast is queued
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
                    .height(1)
                    .anchor(anchor)
                    .layer(Layer::Overlay)
                    .margin(8, 8, 8, 8)
                    .exclusive_zone(Some(0))
                    .keyboard_interactivity(KeyboardInteractivity::None)
                    .background_color(Color::TRANSPARENT)
                    .namespace("ashell-toast"),
                move || toast_view(handle, view_config.clone(), theme, content_wr),
            ));
        } else if !has_toasts && let Some(h) = slot_ref.take() {
            h.close();
        }
    })
    .detach();

    // Track the measured content height
    let slot = surface;
    create_effect(move || {
        let rect = content_wr.rect().get();
        if let Some(h) = &*slot.borrow()
            && rect.height > 0.0
        {
            h.set_size(TOAST_WIDTH, rect.height.ceil() as u32);
        }
    })
    .detach();
}

fn toast_view(
    handle: NotificationsHandle,
    config: NotificationsModuleConfig,
    theme: ThemeColors,
    content_wr: WidgetRef,
) -> impl Widget {
    container()
        .width(fill())
        .widget_ref(content_wr)
        .layout(Flex::column().spacing(8))
        .children(move || {
            let ids = handle.toasts.get();
            handle.list.with(|l| {
                ids.iter()
                    .filter_map(|id| l.items.iter().find(|n| n.id == *id))
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
    let has_notifications = create_memo(move || handle.list.with(|l| !l.items.is_empty()));

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
                    let has = handle.list.with(|l| !l.items.is_empty());
                    has.then(|| {
                        icon_button()
                            .icon(StaticIcon::Delete)
                            .size(ButtonSize::Small)
                            .kind(ButtonKind::Transparent)
                            .on_click(move || {
                                let ids: Vec<u32> =
                                    handle.list.with(|l| l.items.iter().map(|n| n.id).collect());
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
            if handle.list.with(|l| l.items.is_empty()) {
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
                        .items
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
                    for n in &l.items {
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
