use crate::{
    components::collapsible::{self, collapsible},
    components::countdown_bar::{Countdown, countdown_bar},
    components::icons::{StaticIcon, icon, icon_button},
    components::scrollable,
    components::slide::{self, SlideDirection, slide},
    components::{ButtonHierarchy, ButtonKind, ButtonSize, MenuSize, styled_button},
    config::{NotificationsModuleConfig, Surface, ToastPosition},
    services::{
        ReadOnlyService, ServiceEvent, compositor, connection_pid,
        notifications::{
            Notification, NotificationIcon, NotificationsService, Urgency,
            dbus::{CloseGuard, DEFAULT_ACTION_KEY, NotificationDaemon, NotificationEvent},
        },
    },
    theme::{Paint, surface_border, use_theme},
};
use ashell_i18n::t;
use chrono::{DateTime, Local};
use iced::{
    Alignment, Border, Column, Element, Length, Padding, Row, Size, Subscription, Task, Theme,
    widget::{
        Container, Space, blur, button, column, container, image, mouse_area, row, sensor, svg,
        text,
    },
};
use itertools::Itertools;
use log::{debug, error};
use std::{
    collections::{HashMap, HashSet, VecDeque},
    time::{Duration, Instant},
};
use zbus::Connection;

const ICON_SIZE: f32 = 24.0;

fn notification_icon<'a, M: 'a>(icon_kind: Option<&NotificationIcon>) -> Element<'a, M> {
    match icon_kind {
        Some(NotificationIcon::Svg(handle)) => svg(handle.clone())
            .width(Length::Fixed(ICON_SIZE))
            .height(Length::Fixed(ICON_SIZE))
            .into(),
        Some(NotificationIcon::Image(handle)) => image(handle.clone())
            .width(Length::Fixed(ICON_SIZE))
            .height(Length::Fixed(ICON_SIZE))
            .into(),
        None => icon(StaticIcon::Bell).size(ICON_SIZE).into(),
    }
}

/// The window to bring up once a click invokes a notification's default
/// action: on Wayland the sender can't raise its own window without an
/// `xdg-activation` token, so the compositor focuses it.
#[derive(Debug)]
struct FocusTarget {
    sender: Option<String>,
    apps: Vec<String>,
}

fn invoke_and_close_task(
    connection: Option<Connection>,
    id: u32,
    action_key: Option<String>,
    guard: CloseGuard,
    focus: Option<FocusTarget>,
) -> Task<Message> {
    Task::perform(
        async move {
            let Some(connection) = connection else {
                return;
            };
            match NotificationDaemon::invoke_and_close(&connection, id, action_key, guard).await {
                Ok(true) => {
                    if let Some(FocusTarget { sender, apps }) = focus {
                        let pid = match sender {
                            Some(sender) => connection_pid(&connection, &sender)
                                .await
                                .inspect_err(|e| debug!("No pid for sender {sender}: {e}"))
                                .ok(),
                            None => None,
                        };
                        if let Err(e) = compositor::focus_app_window(pid, apps, None).await {
                            debug!("Can't focus the window of notification {id}: {e}");
                        }
                    }
                }
                Ok(false) => {}
                Err(e) => error!("Failed to close notification id {}: {}", id, e),
            }
        },
        |_| Message::NotificationClosed,
    )
}

async fn close_notification_ids(connection: Option<Connection>, notification_ids: &[u32]) {
    if let Some(connection) = connection {
        for id in notification_ids {
            if let Err(e) =
                NotificationDaemon::invoke_and_close(&connection, *id, None, CloseGuard::Any).await
            {
                error!("Failed to close notification id {}: {}", id, e);
            }
        }
    }
}

fn evict_task(connection: Option<Connection>, evicted: Vec<(u32, u64)>) -> Task<Message> {
    Task::perform(
        async move {
            if let Some(connection) = connection
                && let Err(e) = NotificationDaemon::evict_notifications(&connection, &evicted).await
            {
                error!("Failed to close evicted notifications: {e}");
            }
        },
        |_| Message::NotificationClosed,
    )
}

fn delayed_toast_message(delay: Duration, id: u32, msg: fn(u32) -> Message) -> Task<Message> {
    Task::perform(
        async move {
            tokio::time::sleep(delay).await;
            id
        },
        msg,
    )
}

fn toast_timeout(required_timeout: i32, timeout_ms: u64) -> Option<Duration> {
    match required_timeout {
        -1 => Some(Duration::from_millis(timeout_ms)),
        0 => None,
        required if required > 0 => Some(Duration::from_millis(required as u64)),
        _ => None,
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    ConfigReloaded(NotificationsModuleConfig),
    NotificationClicked(u32),
    InvokeAction(u32, String),
    NotificationClosed,
    CloseNotificationById(u32),
    ClearNotifications,
    NotificationsCleared,
    ClearGroup(String),
    GroupCleared(String, Vec<u32>),
    Event(ServiceEvent<NotificationsService>),
    ToggleGroup(String),
    ToggleDnd,
    ExpireToast(u32),
    DismissToast(u32),
    StartCollapse(u32),
    DismissAnimationComplete(u32),
    ToastResized(Size),
    ToastsHovered,
    ToastsUnhovered,
}

#[derive(Debug, PartialEq)]
enum NotificationStyle {
    Toast,
    Card,
}

pub enum Action {
    None,
    Task(Task<Message>),
    /// Run the task and close the menu, e.g. to make way for the window of a
    /// clicked notification.
    TaskAndCloseMenu(Task<Message>),
    Show(Task<Message>),
    Hide(Task<Message>),
    UpdateToastInputRegion(Size),
}

impl Action {
    fn with_task(self, task: Task<Message>) -> Self {
        match self {
            Action::None => Action::Task(task),
            Action::Task(t) => Action::Task(Task::batch([t, task])),
            Action::TaskAndCloseMenu(t) => Action::TaskAndCloseMenu(Task::batch([t, task])),
            Action::Show(t) => Action::Show(Task::batch([t, task])),
            Action::Hide(t) => Action::Hide(Task::batch([t, task])),
            Action::UpdateToastInputRegion(_) => Action::Task(task),
        }
    }
}

const COUNTDOWN_BAR_HEIGHT: f32 = 3.0;

// Must match the widget durations so task delays align with animation end.
const SLIDE_ANIMATION: Duration = slide::DEFAULT_DURATION;
const COLLAPSE_ANIMATION: Duration = collapsible::DEFAULT_DURATION;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DismissPhase {
    Sliding,
    Collapsing,
}

#[derive(Debug, Clone, Copy)]
enum ToastTimer {
    Running {
        deadline: Instant,
        total: Duration,
    },
    Paused {
        remaining: Duration,
        total: Duration,
        /// Latest time an `ExpireToast` already in flight lands, if any.
        wakeup: Option<Instant>,
    },
}

impl ToastTimer {
    fn wakeup(self) -> Option<Instant> {
        match self {
            Self::Running { deadline, .. } => Some(deadline),
            Self::Paused { wakeup, .. } => wakeup,
        }
    }
}

impl From<ToastTimer> for Countdown {
    fn from(timer: ToastTimer) -> Self {
        match timer {
            ToastTimer::Running { deadline, total } => Self::Running { deadline, total },
            ToastTimer::Paused {
                remaining, total, ..
            } => Self::Paused { remaining, total },
        }
    }
}

fn needs_wakeup(previous: Option<ToastTimer>, deadline: Instant) -> bool {
    previous
        .and_then(ToastTimer::wakeup)
        .is_none_or(|wakeup| deadline < wakeup)
}

pub struct Notifications {
    config: NotificationsModuleConfig,
    connection: Option<Connection>,
    notifications: VecDeque<Notification>,
    expanded_groups: HashSet<String>,
    blocklist: Vec<crate::config::RegexCfg>,
    toasts: VecDeque<u32>,
    dismiss_phases: HashMap<u32, DismissPhase>,
    toast_timers: HashMap<u32, ToastTimer>,
    toasts_hovered: bool,
    animations_enabled: bool,
    dnd: bool,
}

impl Notifications {
    pub fn new(config: NotificationsModuleConfig, animations_enabled: bool) -> Self {
        let blocklist = config.blocklist.clone();
        Self {
            config,
            connection: None,
            notifications: VecDeque::new(),
            expanded_groups: HashSet::new(),
            blocklist,
            toasts: VecDeque::new(),
            dismiss_phases: HashMap::new(),
            toast_timers: HashMap::new(),
            toasts_hovered: false,
            animations_enabled,
            dnd: false,
        }
    }

    pub fn set_animations_enabled(&mut self, enabled: bool) {
        self.animations_enabled = enabled;
    }

    pub fn dnd(&self) -> bool {
        self.dnd
    }

    pub fn toggle_dnd(&mut self) {
        self.dnd = !self.dnd;
    }

    fn is_blocklisted(&self, app_name: &str) -> bool {
        self.blocklist
            .iter()
            .any(|pattern| pattern.is_match(app_name))
    }

    fn find_notification(&self, id: u32) -> Option<&Notification> {
        self.notifications.iter().find(|n| n.id == id)
    }

    fn close_guard(&self, id: u32) -> CloseGuard {
        self.find_notification(id)
            .map_or(CloseGuard::Any, |n| CloseGuard::Revision(n.revision))
    }

    /// `id`'s default action and the window it brings up; the action buttons
    /// don't focus it.
    fn default_action(&self, id: u32) -> Option<(String, FocusTarget)> {
        let notification = self.find_notification(id)?;
        notification
            .action_pairs()
            .any(|(key, _)| key == DEFAULT_ACTION_KEY)
            .then(|| {
                let target = FocusTarget {
                    sender: notification.sender.clone(),
                    apps: notification.app_ids(),
                };
                (DEFAULT_ACTION_KEY.to_string(), target)
            })
    }

    fn invoke_and_dismiss(
        &mut self,
        id: u32,
        action_key: Option<String>,
        focus: Option<FocusTarget>,
    ) -> Action {
        let connection = self.connection.clone();
        let guard = self.close_guard(id);
        let invoke_task = invoke_and_close_task(connection, id, action_key, guard, focus);
        if !self.toasts.contains(&id) || self.dismiss_phases.contains_key(&id) {
            return Action::Task(invoke_task);
        }
        if !self.animations_enabled {
            let had_toasts = self.remove_toast(id);
            return self.hide_toasts_if_empty_with_task(had_toasts, invoke_task);
        }
        self.dismiss_phases.insert(id, DismissPhase::Sliding);
        Action::Task(Task::batch(vec![
            invoke_task,
            delayed_toast_message(SLIDE_ANIMATION, id, Message::StartCollapse),
        ]))
    }

    fn clear_toasts(&mut self) -> bool {
        let had_toasts = !self.toasts.is_empty();
        self.toasts.clear();
        self.dismiss_phases.clear();
        self.toast_timers.clear();
        self.toasts_hovered = false;
        had_toasts
    }

    fn remove_toast(&mut self, id: u32) -> bool {
        let had_toasts = !self.toasts.is_empty();
        self.toasts.retain(|&toast_id| toast_id != id);
        self.toast_timers.remove(&id);
        had_toasts
    }

    fn remove_toasts(&mut self, ids: &[u32]) -> bool {
        let had_toasts = !self.toasts.is_empty();
        let ids: HashSet<u32> = ids.iter().copied().collect();
        self.toasts.retain(|toast_id| !ids.contains(toast_id));
        self.toast_timers.retain(|id, _| !ids.contains(id));
        had_toasts
    }

    fn notification_ids_for_app(&self, app_name: &str) -> Vec<u32> {
        self.notifications
            .iter()
            .filter(|notification| notification.app_name == app_name)
            .map(|notification| notification.id)
            .collect()
    }

    fn hide_toasts_if_empty(&mut self, had_toasts: bool) -> Action {
        if had_toasts && self.toasts.is_empty() {
            self.toasts_hovered = false;
            Action::Hide(Task::none())
        } else {
            Action::None
        }
    }

    fn hide_toasts_if_empty_with_task(&mut self, had_toasts: bool, task: Task<Message>) -> Action {
        if had_toasts && self.toasts.is_empty() {
            self.toasts_hovered = false;
            Action::Hide(task)
        } else {
            Action::Task(task)
        }
    }

    fn toast_action_for_update_event(&mut self, update_event: &NotificationEvent) -> Action {
        if !self.config.toast {
            return Action::None;
        }

        match update_event {
            NotificationEvent::Received(notification) => {
                if self.config.toast_limit == 0 {
                    let had_toasts = self.clear_toasts();
                    return self.hide_toasts_if_empty(had_toasts);
                }

                if !self.toasts.contains(&notification.id) {
                    while self.toasts.len() >= self.config.toast_limit
                        && let Some(evicted) = self.toasts.pop_front()
                    {
                        self.dismiss_phases.remove(&evicted);
                        self.toast_timers.remove(&evicted);
                    }
                    self.toasts.push_back(notification.id);
                }

                // A replace cancels an in-flight dismissal so the replacement is
                // shown instead of being swallowed by the slide-out.
                self.dismiss_phases.remove(&notification.id);

                // Critical notifications are persistent per the freedesktop
                // spec: they must be acknowledged by the user.
                let timeout = if notification.urgency == Urgency::Critical {
                    None
                } else {
                    toast_timeout(notification.expire_timeout, self.config.toast_timeout)
                };

                let task = if let Some(timeout) = timeout {
                    let previous = self.toast_timers.get(&notification.id).copied();
                    if self.toasts_hovered {
                        self.toast_timers.insert(
                            notification.id,
                            ToastTimer::Paused {
                                remaining: timeout,
                                total: timeout,
                                wakeup: previous.and_then(ToastTimer::wakeup),
                            },
                        );
                        Task::none()
                    } else {
                        let deadline = Instant::now() + timeout;
                        self.toast_timers.insert(
                            notification.id,
                            ToastTimer::Running {
                                deadline,
                                total: timeout,
                            },
                        );
                        if needs_wakeup(previous, deadline) {
                            delayed_toast_message(timeout, notification.id, Message::ExpireToast)
                        } else {
                            Task::none()
                        }
                    }
                } else {
                    self.toast_timers.remove(&notification.id);
                    Task::none()
                };

                Action::Show(task)
            }
            NotificationEvent::Closed(id) => {
                let id = *id;
                if self.dismiss_phases.contains_key(&id) {
                    return Action::None;
                }
                let was_showing = self.remove_toast(id);
                self.hide_toasts_if_empty(was_showing)
            }
        }
    }

    fn apply_update_event(&mut self, update_event: NotificationEvent) -> Vec<(u32, u64)> {
        match update_event {
            NotificationEvent::Received(notification) => {
                self.notifications.retain(|n| n.id != notification.id);
                self.notifications.push_front(*notification);
                self.evict_over_limit()
            }
            NotificationEvent::Closed(id) => {
                self.notifications.retain(|n| n.id != id);
                Vec::new()
            }
        }
    }

    /// Skips visible toasts, which can be older than the limit under DND.
    fn evict_over_limit(&mut self) -> Vec<(u32, u64)> {
        let mut evicted = Vec::new();
        while self.notifications.len() > self.config.history_limit
            && let Some(pos) = self
                .notifications
                .iter()
                .rposition(|n| !self.toasts.contains(&n.id))
            && let Some(notification) = self.notifications.remove(pos)
        {
            evicted.push((notification.id, notification.revision));
        }
        evicted
    }

    fn with_eviction(&self, action: Action, evicted: Vec<(u32, u64)>) -> Action {
        if evicted.is_empty() {
            action
        } else {
            action.with_task(evict_task(self.connection.clone(), evicted))
        }
    }

    pub fn update(&mut self, message: Message) -> Action {
        match message {
            Message::ConfigReloaded(config) => {
                let hide = !config.toast && self.config.toast && !self.toasts.is_empty();
                self.blocklist = config.blocklist.clone();
                self.config = config;
                let action = if hide {
                    self.clear_toasts();
                    Action::Hide(Task::none())
                } else {
                    Action::None
                };
                let evicted = self.evict_over_limit();
                self.with_eviction(action, evicted)
            }
            Message::Event(event) => match event {
                ServiceEvent::Init(service) => {
                    self.connection = Some(service.connection);
                    Action::None
                }
                ServiceEvent::Update(update_event) => {
                    if let NotificationEvent::Received(notification) = &update_event
                        && self.is_blocklisted(&notification.app_name)
                    {
                        // The UI never shows it, so nothing else would close it
                        // and the daemon would keep its revision tracked for the
                        // lifetime of the process. Dismiss it at the source.
                        return Action::Task(evict_task(
                            self.connection.clone(),
                            vec![(notification.id, notification.revision)],
                        ));
                    }

                    let suppress_toast = self.dnd
                        && matches!(
                            &update_event,
                            NotificationEvent::Received(notification)
                                if notification.urgency != Urgency::Critical
                        );
                    let toast_action = if suppress_toast {
                        Action::None
                    } else {
                        self.toast_action_for_update_event(&update_event)
                    };
                    let evicted = self.apply_update_event(update_event);
                    self.with_eviction(toast_action, evicted)
                }
                ServiceEvent::Error(_) => Action::None,
            },
            Message::NotificationClicked(id) => {
                let connection = self.connection.clone();
                let (action_key, focus) = self.default_action(id).unzip();
                let guard = self.close_guard(id);
                let close_menu = focus.is_some();
                let task = invoke_and_close_task(connection, id, action_key, guard, focus);
                if close_menu {
                    Action::TaskAndCloseMenu(task)
                } else {
                    Action::Task(task)
                }
            }
            Message::InvokeAction(id, action_key) => {
                self.invoke_and_dismiss(id, Some(action_key), None)
            }
            Message::NotificationClosed => Action::None,
            Message::ToggleDnd => {
                self.toggle_dnd();
                Action::None
            }
            Message::ClearNotifications => {
                let connection = self.connection.clone();
                let notification_ids: Vec<u32> = self.notifications.iter().map(|n| n.id).collect();

                Action::Task(Task::perform(
                    async move {
                        close_notification_ids(connection, &notification_ids).await;
                    },
                    |_| Message::NotificationsCleared,
                ))
            }
            Message::NotificationsCleared => {
                let had_toasts = self.clear_toasts();
                self.hide_toasts_if_empty(had_toasts)
            }
            Message::ClearGroup(app_name) => {
                let connection = self.connection.clone();
                let notification_ids = self.notification_ids_for_app(&app_name);

                Action::Task(Task::perform(
                    async move {
                        close_notification_ids(connection, &notification_ids).await;
                        (app_name, notification_ids)
                    },
                    |(app_name, ids)| Message::GroupCleared(app_name, ids),
                ))
            }
            Message::GroupCleared(app_name, group_ids) => {
                self.expanded_groups.remove(&app_name);
                let had_toasts = self.remove_toasts(&group_ids);
                self.hide_toasts_if_empty(had_toasts)
            }
            Message::ToggleGroup(app_name) => {
                if !self.expanded_groups.remove(&app_name) {
                    self.expanded_groups.insert(app_name);
                }
                Action::None
            }
            Message::ExpireToast(id) => {
                let Some(timer) = self.toast_timers.get_mut(&id) else {
                    return Action::None;
                };
                let deadline = match timer {
                    ToastTimer::Running { deadline, .. } => *deadline,
                    ToastTimer::Paused { wakeup, .. } => {
                        *wakeup = None;
                        return Action::None;
                    }
                };
                let now = Instant::now();
                if now < deadline {
                    // Replaced with a later deadline: wait out the remainder.
                    return Action::Task(delayed_toast_message(
                        deadline - now,
                        id,
                        Message::ExpireToast,
                    ));
                }
                self.toast_timers.remove(&id);

                if self.toasts.contains(&id) && !self.dismiss_phases.contains_key(&id) {
                    if !self.animations_enabled {
                        let had_toasts = self.remove_toast(id);
                        return self.hide_toasts_if_empty(had_toasts);
                    }
                    self.dismiss_phases.insert(id, DismissPhase::Sliding);
                    Action::Task(delayed_toast_message(
                        SLIDE_ANIMATION,
                        id,
                        Message::StartCollapse,
                    ))
                } else {
                    Action::None
                }
            }
            Message::CloseNotificationById(id) => {
                let connection = self.connection.clone();
                let guard = self.close_guard(id);
                let had_toasts = self.remove_toast(id);
                self.dismiss_phases.remove(&id);

                let task = invoke_and_close_task(connection, id, None, guard, None);
                self.hide_toasts_if_empty_with_task(had_toasts, task)
            }
            Message::DismissToast(id) => {
                if self.toasts.contains(&id) && !self.dismiss_phases.contains_key(&id) {
                    let (action_key, focus) = self.default_action(id).unzip();
                    self.invoke_and_dismiss(id, action_key, focus)
                } else {
                    Action::None
                }
            }
            Message::StartCollapse(id) => {
                if self.dismiss_phases.get(&id) == Some(&DismissPhase::Sliding) {
                    self.dismiss_phases.insert(id, DismissPhase::Collapsing);
                    Action::Task(delayed_toast_message(
                        COLLAPSE_ANIMATION,
                        id,
                        Message::DismissAnimationComplete,
                    ))
                } else {
                    Action::None
                }
            }
            Message::DismissAnimationComplete(id) => {
                if self.dismiss_phases.remove(&id).is_none() {
                    // Cancelled by a replace: the toast is on screen again.
                    return Action::None;
                }
                let had_toasts = self.remove_toast(id);
                self.hide_toasts_if_empty(had_toasts)
            }
            Message::ToastResized(size) => Action::UpdateToastInputRegion(size),
            Message::ToastsHovered => {
                if self.toasts_hovered {
                    return Action::None;
                }
                self.toasts_hovered = true;

                let now = Instant::now();
                for timer in self.toast_timers.values_mut() {
                    if let ToastTimer::Running { deadline, total } = *timer {
                        *timer = ToastTimer::Paused {
                            remaining: deadline.saturating_duration_since(now),
                            total,
                            wakeup: Some(deadline),
                        };
                    }
                }
                Action::None
            }
            Message::ToastsUnhovered => {
                if !self.toasts_hovered {
                    return Action::None;
                }
                self.toasts_hovered = false;

                let now = Instant::now();
                let tasks = self
                    .toast_timers
                    .iter_mut()
                    .filter_map(|(&id, timer)| {
                        let previous = *timer;
                        let ToastTimer::Paused {
                            remaining, total, ..
                        } = previous
                        else {
                            return None;
                        };
                        let deadline = now + remaining;
                        *timer = ToastTimer::Running { deadline, total };
                        needs_wakeup(Some(previous), deadline)
                            .then(|| delayed_toast_message(remaining, id, Message::ExpireToast))
                    })
                    .collect::<Vec<_>>();

                if tasks.is_empty() {
                    Action::None
                } else {
                    Action::Task(Task::batch(tasks))
                }
            }
        }
    }

    fn format_timestamp(&self, timestamp: std::time::SystemTime) -> String {
        let datetime: DateTime<Local> = timestamp.into();
        datetime.format(&self.config.format).to_string()
    }

    fn notification_button_style(
        style: NotificationStyle,
        urgency: Urgency,
    ) -> impl Fn(&Theme, iced::widget::button::Status) -> iced::widget::button::Style + use<> {
        let radius = use_theme(|t| t.radius);
        move |iced_theme: &Theme, status| {
            let mut border = match style {
                NotificationStyle::Toast => surface_border(iced_theme, radius.lg),
                NotificationStyle::Card => Border::default().rounded(radius.md),
            };

            if urgency == Urgency::Critical {
                border.width = 1.0;
                border.color = iced_theme.palette().danger;
            }

            let mut button_style = iced::widget::button::Style {
                text_color: iced_theme.palette().text,
                border,
                ..iced::widget::button::Style::default()
            };
            let bg = Paint::surface(
                iced_theme,
                if style == NotificationStyle::Toast {
                    iced_theme.extended_palette().background.base.color
                } else {
                    iced_theme.extended_palette().background.weak.color
                },
            );
            button_style.background = Some(
                if matches!(status, iced::widget::button::Status::Hovered) {
                    bg.hovered(iced_theme)
                } else {
                    bg
                }
                .into(),
            );
            button_style
        }
    }

    fn toast_countdown(&self, id: u32, toast: bool) -> Option<Countdown> {
        if !toast || !self.config.toast_timeout_bar {
            return None;
        }

        self.toast_timers.get(&id).copied().map(Countdown::from)
    }

    fn notification_card<'a>(
        &'a self,
        notification: &'a Notification,
        on_press: Message,
        toast: bool,
    ) -> Element<'a, Message> {
        let (space, font_size, palette) = use_theme(|t| (t.space, t.font_size, t.palette));
        let muted = palette.text.scale_alpha(0.55);
        let faint = palette.text.scale_alpha(0.4);

        let timestamp = self.config.show_timestamps.then(|| {
            text(self.format_timestamp(notification.timestamp))
                .size(font_size.xs)
                .color(faint)
        });

        let meta = column![
            text(&notification.app_name)
                .size(font_size.xs)
                .color(muted)
                .wrapping(text::Wrapping::WordOrGlyph),
            timestamp,
        ]
        .spacing(1.0);

        let header = row![
            notification_icon(notification.icon.as_ref()),
            meta.width(Length::Fill),
            icon_button(StaticIcon::Close)
                .kind(ButtonKind::Transparent)
                .hierarchy(ButtonHierarchy::Danger)
                .size(ButtonSize::Small)
                .on_press(Message::CloseNotificationById(notification.id)),
        ]
        .spacing(space.xs)
        .align_y(Alignment::Center);

        let body =
            ((!toast || self.config.show_bodies) && !notification.body.is_empty()).then(|| {
                text(&notification.body)
                    .size(font_size.sm)
                    .color(muted)
                    .wrapping(text::Wrapping::WordOrGlyph)
            });

        let text_block = column![
            text(&notification.summary)
                .size(font_size.md)
                .wrapping(text::Wrapping::WordOrGlyph),
            body,
        ]
        .spacing(space.xxs);

        let mut card = container(column![header, text_block].spacing(space.xs));

        if toast {
            card = card.max_height(self.config.toast_max_height).clip(true);
        }

        // Outside the clipped card so a tall body never hides the buttons.
        let actions_row = Self::action_buttons(notification)
            .map(|row| row.padding(Padding::new(space.xs).top(0.)));

        let countdown = self
            .toast_countdown(notification.id, toast)
            .map(|countdown| {
                countdown_bar(
                    countdown,
                    COUNTDOWN_BAR_HEIGHT,
                    use_theme(|t| t.palette.primary),
                )
            });

        button(column!(card, actions_row, countdown))
            .on_press(on_press)
            .width(Length::Fill)
            .padding(space.sm)
            .style(Self::notification_button_style(
                if toast {
                    NotificationStyle::Toast
                } else {
                    NotificationStyle::Card
                },
                notification.urgency,
            ))
            .into()
    }

    /// One button per non-default action; `None` when there are none.
    /// Sized to their labels and right-aligned, so a single short action
    /// doesn't stretch across the whole notification.
    fn action_buttons<'a>(notification: &'a Notification) -> Option<Container<'a, Message>> {
        let space = use_theme(|t| t.space);
        let buttons = notification
            .action_pairs()
            .filter(|(key, _)| *key != DEFAULT_ACTION_KEY)
            .map(|(key, label)| {
                styled_button(label)
                    .kind(ButtonKind::Outline)
                    .size(ButtonSize::Small)
                    .on_press(Message::InvokeAction(notification.id, key.to_string()))
                    .into()
            })
            .collect::<Vec<Element<'a, Message>>>();
        (!buttons.is_empty()).then(|| {
            container(Row::with_children(buttons).spacing(space.xs)).align_right(Length::Fill)
        })
    }

    fn group_item<'a>(&'a self, notification: &'a Notification) -> Element<'a, Message> {
        let (space, font_size, palette) = use_theme(|t| (t.space, t.font_size, t.palette));
        let muted = palette.text.scale_alpha(0.55);

        let body = (!notification.body.is_empty()).then(|| {
            text(&notification.body)
                .size(font_size.sm)
                .color(muted)
                .wrapping(text::Wrapping::WordOrGlyph)
        });

        button(
            column![
                row!(
                    text(&notification.summary)
                        .size(font_size.sm)
                        .wrapping(text::Wrapping::WordOrGlyph)
                        .width(Length::Fill),
                    text(self.format_timestamp(notification.timestamp))
                        .size(font_size.xs)
                        .color(muted)
                )
                .spacing(space.xs)
                .align_y(Alignment::Center),
                body,
                Self::action_buttons(notification)
            ]
            .padding(space.sm)
            .spacing(space.xxs),
        )
        .style(Self::notification_button_style(
            NotificationStyle::Card,
            notification.urgency,
        ))
        .on_press(Message::NotificationClicked(notification.id))
        .into()
    }

    pub fn view(&'_ self) -> Element<'_, Message> {
        let bell_icon = if self.dnd {
            StaticIcon::BellOff
        } else if !self.notifications.is_empty() {
            StaticIcon::BellBadge
        } else {
            StaticIcon::Bell
        };
        icon(bell_icon).into()
    }

    pub fn menu_view<'a>(&'a self) -> Element<'a, Message> {
        let (space, font_size, palette) = use_theme(|t| (t.space, t.font_size, t.palette));
        let is_empty = self.notifications.is_empty();
        let muted = palette.text.scale_alpha(0.55);

        let content = if is_empty {
            container(
                column![
                    icon(StaticIcon::BellOff).size(font_size.xxl).color(muted),
                    text(t!("notifications-empty"))
                        .size(font_size.sm)
                        .color(muted),
                ]
                .spacing(space.sm)
                .align_x(Alignment::Center),
            )
            .width(Length::Fill)
            .center_x(Length::Fill)
            .padding(space.xl)
            .into()
        } else if self.config.grouped {
            self.grouped_notifications()
        } else {
            self.list_notifications()
        };

        column![
            Row::new()
                .push(
                    text(t!("notifications-heading"))
                        .width(Length::Fill)
                        .size(font_size.md)
                )
                .push(
                    icon_button(if self.dnd {
                        StaticIcon::BellOff
                    } else {
                        StaticIcon::Bell
                    })
                    .kind(ButtonKind::Transparent)
                    .size(ButtonSize::Small)
                    .hierarchy(if self.dnd {
                        ButtonHierarchy::Danger
                    } else {
                        ButtonHierarchy::Secondary
                    })
                    .on_press(Message::ToggleDnd)
                )
                .push((!is_empty).then(|| {
                    icon_button(StaticIcon::Delete)
                        .kind(ButtonKind::Transparent)
                        .size(ButtonSize::Small)
                        .on_press(Message::ClearNotifications)
                }))
                .spacing(space.xxs)
                .align_y(Alignment::Center),
            container(scrollable(content).spacing(space.xs)).max_height(400.),
        ]
        .width(MenuSize::Medium)
        .spacing(space.sm)
        .into()
    }

    pub fn toast_view<'a>(&'a self) -> Element<'a, Message> {
        let space = use_theme(|t| t.space);
        if self.toasts.is_empty() {
            return Space::new().into();
        }

        let slide_direction = match self.config.toast_position {
            ToastPosition::TopRight | ToastPosition::BottomRight => SlideDirection::Right,
            ToastPosition::TopLeft | ToastPosition::BottomLeft => SlideDirection::Left,
            ToastPosition::TopCenter => SlideDirection::Up,
            ToastPosition::BottomCenter => SlideDirection::Down,
        };
        let card_width = MenuSize::Medium.size();
        // A toast is alone on its surface, so it publishes its own blur region.
        let (blur_enabled, card_radius) =
            use_theme(|t| (t.surface(Surface::Notifications).blur, t.radius.lg));

        let mut toast_column = column!().spacing(space.sm);

        for &toast_id in &self.toasts {
            if let Some(notification) = self.find_notification(toast_id) {
                let phase = self.dismiss_phases.get(&toast_id).copied();
                let is_dismissing = phase.is_some();
                let is_collapsing = phase == Some(DismissPhase::Collapsing);
                toast_column = toast_column.push(
                    collapsible(
                        !is_collapsing,
                        slide(!is_dismissing, slide_direction, card_width, {
                            let card = self.notification_card(
                                notification,
                                Message::DismissToast(notification.id),
                                true,
                            );
                            if blur_enabled {
                                blur(card_radius, card).into()
                            } else {
                                card
                            }
                        })
                        .animated(self.animations_enabled)
                        .key(toast_id as u64),
                    )
                    .animated(self.animations_enabled)
                    .key(toast_id as u64),
                );
            }
        }

        let v_align = match self.config.toast_position {
            ToastPosition::TopLeft | ToastPosition::TopRight | ToastPosition::TopCenter => {
                Alignment::Start
            }
            ToastPosition::BottomLeft
            | ToastPosition::BottomRight
            | ToastPosition::BottomCenter => Alignment::End,
        };
        let h_align = match self.config.toast_position {
            ToastPosition::TopLeft | ToastPosition::BottomLeft => Alignment::Start,
            ToastPosition::TopRight | ToastPosition::BottomRight => Alignment::End,
            ToastPosition::TopCenter | ToastPosition::BottomCenter => Alignment::Center,
        };

        let toast_content = sensor(
            container(toast_column)
                .width(MenuSize::Medium)
                .padding(space.sm),
        )
        .on_resize(Message::ToastResized);

        let toast_content = mouse_area(toast_content)
            .on_enter(Message::ToastsHovered)
            .on_exit(Message::ToastsUnhovered);

        container(toast_content)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_y(v_align)
            .align_x(h_align)
            .into()
    }

    pub fn subscription(&self) -> Subscription<Message> {
        NotificationsService::subscribe().map(Message::Event)
    }

    fn grouped_notifications<'a>(&'a self) -> Element<'a, Message> {
        let (space, font_size, palette) = use_theme(|t| (t.space, t.font_size, t.palette));
        let muted = palette.text.scale_alpha(0.55);
        let mut content = column!()
            .spacing(space.sm)
            .padding(Padding::default().right(space.md).left(space.xs));

        for (_app_name, group) in self
            .notifications
            .iter()
            .sorted_by(|a, b| a.app_name.cmp(&b.app_name))
            .chunk_by(|n| n.app_name.clone())
            .into_iter()
        {
            let mut iter = group.peekable();
            let first = match iter.next() {
                Some(n) => n,
                None => continue,
            };

            // Single notification in the group — use the normal card layout.
            if iter.peek().is_none() {
                content = content.push(self.notification_card(
                    first,
                    Message::NotificationClicked(first.id),
                    false,
                ));
                continue;
            }

            // Multiple notifications — use the group layout.
            let app_name = first.app_name.clone();
            let is_expanded = self.expanded_groups.contains(&app_name);
            let app_icon = notification_icon(first.icon.as_ref());

            let mut count = 1;
            let mut has_critical = first.urgency == Urgency::Critical;
            let mut group_notifications = vec![];

            if is_expanded {
                group_notifications.push(self.group_item(first));
                for notification in iter {
                    count += 1;
                    has_critical = has_critical || notification.urgency == Urgency::Critical;
                    group_notifications.push(self.group_item(notification));
                }
            } else {
                for notification in iter {
                    count += 1;
                    has_critical = has_critical || notification.urgency == Urgency::Critical;
                }
                group_notifications.push(self.group_item(first));
            }

            let clear_msg = Message::ClearGroup(app_name.clone());
            let toggle_msg = Message::ToggleGroup(app_name.clone());

            let header = row!(
                app_icon,
                text(app_name)
                    .size(font_size.sm)
                    .wrapping(text::Wrapping::WordOrGlyph)
                    .width(Length::Fill),
                text(t!("notifications-group-count", count = count))
                    .size(font_size.xs)
                    .color(muted),
                icon_button(StaticIcon::Delete)
                    .on_press(clear_msg)
                    .size(ButtonSize::Small)
                    .kind(ButtonKind::Transparent)
                    .hierarchy(ButtonHierarchy::Danger)
            )
            .spacing(space.xs)
            .align_y(Alignment::Center);

            let item = Column::new()
                .push(
                    button(header)
                        .padding(space.sm)
                        .style(Self::notification_button_style(
                            NotificationStyle::Card,
                            if has_critical {
                                Urgency::Critical
                            } else {
                                Urgency::Normal
                            },
                        ))
                        .on_press(toggle_msg),
                )
                .extend(group_notifications)
                .spacing(space.xxs);

            content = content.push(item);
        }
        content.into()
    }

    fn list_notifications<'a>(&'a self) -> Element<'a, Message> {
        let space = use_theme(|t| t.space);
        Column::with_children(
            self.notifications
                .iter()
                .map(|notification| {
                    self.notification_card(
                        notification,
                        Message::NotificationClicked(notification.id),
                        false,
                    )
                })
                .collect::<Vec<Element<'a, Message>>>(),
        )
        .padding(Padding::default().right(space.md).left(space.xs))
        .spacing(space.sm)
        .into()
    }

    pub fn toast_position(&self) -> ToastPosition {
        self.config.toast_position
    }
}

#[cfg(test)]
mod tests {
    use super::{Message, Notifications, ToastTimer};
    use crate::{
        config::NotificationsModuleConfig,
        services::{
            ServiceEvent,
            notifications::dbus::{Notification, NotificationEvent, Urgency},
        },
    };
    use std::time::{Duration, Instant, SystemTime};

    fn notification(id: u32, expire_timeout: i32) -> Notification {
        Notification {
            id,
            revision: 0,
            app_name: "test".to_string(),
            app_icon: String::new(),
            summary: "summary".to_string(),
            body: String::new(),
            actions: vec![],
            desktop_entry: None,
            expire_timeout,
            timestamp: SystemTime::now(),
            urgency: Urgency::Normal,
            sender: None,
            icon: None,
        }
    }

    fn receive(module: &mut Notifications, notification: Notification) {
        let _ = module.update(Message::Event(ServiceEvent::Update(
            NotificationEvent::Received(Box::new(notification)),
        )));
    }

    #[test]
    fn hover_pause_and_resume_keep_the_total() {
        let mut module = Notifications::new(NotificationsModuleConfig::default(), false);
        receive(&mut module, notification(1, 10_000));

        let _ = module.update(Message::ToastsHovered);
        let Some(&ToastTimer::Paused {
            remaining, total, ..
        }) = module.toast_timers.get(&1)
        else {
            panic!("hovering pauses the timer");
        };
        assert_eq!(total, Duration::from_secs(10));
        assert!(remaining <= total);

        let _ = module.update(Message::ToastsUnhovered);
        let Some(&ToastTimer::Running { deadline, total }) = module.toast_timers.get(&1) else {
            panic!("leaving resumes the timer");
        };
        assert_eq!(total, Duration::from_secs(10));
        assert!(deadline.saturating_duration_since(Instant::now()) <= remaining);
    }

    #[test]
    fn replace_while_hovered_takes_the_new_total() {
        let mut module = Notifications::new(NotificationsModuleConfig::default(), false);
        receive(&mut module, notification(1, 10_000));
        let _ = module.update(Message::ToastsHovered);

        receive(&mut module, notification(1, 2_000));

        let Some(&ToastTimer::Paused {
            remaining, total, ..
        }) = module.toast_timers.get(&1)
        else {
            panic!("a replace while hovered starts paused");
        };
        assert_eq!(remaining, Duration::from_secs(2));
        assert_eq!(total, Duration::from_secs(2));
    }
}
