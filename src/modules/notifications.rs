use crate::{
    components::icons::{StaticIcon, icon, icon_button},
    components::menu::MenuSize,
    components::{ButtonHierarchy, ButtonKind, ButtonSize},
    config::{NotificationsModuleConfig, ToastPosition},
    services::{
        ReadOnlyService, ServiceEvent,
        notifications::{
            Notification, NotificationIcon, NotificationsService, Urgency,
            dbus::{NotificationDaemon, NotificationEvent},
        },
    },
    theme::AshellTheme,
};
use chrono::{DateTime, Local};
use iced::{
    Alignment, Border, Column, Element, Length, Padding, Row, Size, Subscription, Task, Theme,
    widget::{Space, button, column, container, image, row, scrollable, sensor, svg, text},
};
use itertools::Itertools;
use log::error;
use std::{
    collections::{HashSet, VecDeque},
    time::Duration,
};
use zbus::Connection;

const ICON_SIZE: f32 = 36.0;

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

fn invoke_and_close_task(
    connection: Option<Connection>,
    id: u32,
    action_key: Option<String>,
) -> Task<Message> {
    Task::perform(
        async move {
            if let Some(connection) = connection {
                if let Some(action_key) = action_key
                    && let Err(e) =
                        NotificationDaemon::invoke_action(&connection, id, action_key).await
                {
                    error!("Failed to invoke notification action for id {}: {}", id, e);
                }
                if let Err(e) = NotificationDaemon::close_notification_by_id(&connection, id).await
                {
                    error!("Failed to close notification id {}: {}", id, e);
                }
            }
        },
        |_| Message::NotificationClosed,
    )
}

async fn close_notification_ids(connection: Option<Connection>, notification_ids: &[u32]) {
    if let Some(connection) = connection {
        for id in notification_ids {
            if let Err(e) = NotificationDaemon::close_notification_by_id(&connection, *id).await {
                error!("Failed to close notification id {}: {}", id, e);
            }
        }
    }
}

fn close_notification_by_id_task(connection: Option<Connection>, id: u32) -> Task<Message> {
    Task::perform(
        async move {
            if let Some(connection) = connection
                && let Err(e) = NotificationDaemon::close_notification_by_id(&connection, id).await
            {
                error!("Failed to close notification id {}: {}", id, e);
            }
        },
        |_| Message::NotificationClosed,
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
    NotificationClosed,
    CloseNotificationById(u32),
    ClearNotifications,
    NotificationsCleared,
    ClearGroup(String),
    GroupCleared(String, Vec<u32>),
    Event(ServiceEvent<NotificationsService>),
    ToggleGroup(String),
    ExpireToast(u32),
    DismissToast(u32),
    ToastResized(Size),
}

#[derive(Debug, PartialEq)]
pub enum NotificationStyle {
    Toast,
    Standalone,
    GroupHeader,
    GroupItem,
    GroupLast,
}

pub enum Action {
    None,
    Task(Task<Message>),
    Show(Task<Message>),
    Hide(Task<Message>),
    UpdateToastInputRegion(Size),
}

pub struct Notifications {
    config: NotificationsModuleConfig,
    connection: Option<Connection>,
    notifications: VecDeque<Notification>,
    expanded_groups: HashSet<String>,
    toasts: VecDeque<u32>,
}

impl Notifications {
    pub fn new(config: NotificationsModuleConfig) -> Self {
        Self {
            config,
            connection: None,
            notifications: VecDeque::new(),
            expanded_groups: HashSet::new(),
            toasts: VecDeque::new(),
        }
    }

    fn find_notification(&self, id: u32) -> Option<&Notification> {
        self.notifications.iter().find(|n| n.id == id)
    }

    fn find_first_action_key(&self, id: u32) -> Option<String> {
        self.find_notification(id)
            .filter(|n| !n.actions.is_empty())
            .and_then(|n| n.actions.first())
            .cloned()
    }

    fn clear_toasts(&mut self) -> bool {
        let had_toasts = !self.toasts.is_empty();
        self.toasts.clear();
        had_toasts
    }

    fn remove_toast(&mut self, id: u32) -> bool {
        let had_toasts = !self.toasts.is_empty();
        self.toasts.retain(|&toast_id| toast_id != id);
        had_toasts
    }

    fn remove_toasts(&mut self, ids: &[u32]) -> bool {
        let had_toasts = !self.toasts.is_empty();
        let ids: HashSet<u32> = ids.iter().copied().collect();
        self.toasts.retain(|toast_id| !ids.contains(toast_id));
        had_toasts
    }

    fn notification_ids_for_app(&self, app_name: &str) -> Vec<u32> {
        self.notifications
            .iter()
            .filter(|notification| notification.app_name == app_name)
            .map(|notification| notification.id)
            .collect()
    }

    fn hide_toasts_if_empty(&self, had_toasts: bool) -> Action {
        if had_toasts && self.toasts.is_empty() {
            Action::Hide(Task::none())
        } else {
            Action::None
        }
    }

    fn hide_toasts_if_empty_with_task(&self, had_toasts: bool, task: Task<Message>) -> Action {
        if had_toasts && self.toasts.is_empty() {
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
                    self.toasts.clear();
                    return Action::None;
                }

                while self.toasts.len() >= self.config.toast_limit {
                    self.toasts.pop_front();
                }
                self.toasts.push_back(notification.id);

                let notification_id = notification.id;
                // Critical notifications are persistent per the freedesktop
                // spec: they must be acknowledged by the user.
                let timeout = if notification.urgency == Urgency::Critical {
                    None
                } else {
                    toast_timeout(notification.expire_timeout, self.config.toast_timeout)
                };

                let timer_task = if let Some(timeout) = timeout {
                    Task::perform(
                        async move {
                            tokio::time::sleep(timeout).await;
                            notification_id
                        },
                        Message::ExpireToast,
                    )
                } else {
                    Task::none()
                };

                Action::Show(timer_task)
            }
            NotificationEvent::Closed(id) => {
                let id = *id;
                let was_showing = self.remove_toast(id);
                self.hide_toasts_if_empty(was_showing)
            }
        }
    }

    fn apply_update_event(&mut self, update_event: NotificationEvent) {
        match update_event {
            NotificationEvent::Received(notification) => {
                self.notifications.push_front(*notification);
            }
            NotificationEvent::Closed(id) => {
                if let Some(pos) = self.notifications.iter().position(|n| n.id == id) {
                    self.notifications.remove(pos);
                }
            }
        }
    }

    pub fn update(&mut self, message: Message) -> Action {
        match message {
            Message::ConfigReloaded(config) => {
                let hide = !config.toast && self.config.toast && !self.toasts.is_empty();
                self.config = config;
                if hide {
                    self.toasts.clear();
                    Action::Hide(Task::none())
                } else {
                    Action::None
                }
            }
            Message::Event(event) => match event {
                ServiceEvent::Init(service) => {
                    self.connection = Some(service.connection);
                    Action::None
                }
                ServiceEvent::Update(update_event) => {
                    let toast_action = self.toast_action_for_update_event(&update_event);
                    self.apply_update_event(update_event);

                    toast_action
                }
                ServiceEvent::Error(_) => Action::None,
            },
            Message::NotificationClicked(id) => {
                let connection = self.connection.clone();
                let action_key = self.find_first_action_key(id);
                Action::Task(invoke_and_close_task(connection, id, action_key))
            }
            Message::NotificationClosed => Action::None,
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
                let had_toasts = self.remove_toast(id);
                self.hide_toasts_if_empty(had_toasts)
            }
            Message::CloseNotificationById(id) => {
                let connection = self.connection.clone();
                let had_toasts = self.remove_toast(id);

                let task = close_notification_by_id_task(connection, id);
                self.hide_toasts_if_empty_with_task(had_toasts, task)
            }
            Message::DismissToast(id) => {
                let connection = self.connection.clone();
                let action_key = self.find_first_action_key(id);
                let had_toasts = self.remove_toast(id);
                let task = invoke_and_close_task(connection, id, action_key);
                self.hide_toasts_if_empty_with_task(had_toasts, task)
            }
            Message::ToastResized(size) => Action::UpdateToastInputRegion(size),
        }
    }

    fn format_timestamp(&self, timestamp: std::time::SystemTime) -> String {
        let datetime: DateTime<Local> = timestamp.into();
        datetime.format(&self.config.format).to_string()
    }

    fn notification_button_style(
        theme: &AshellTheme,
        style: NotificationStyle,
        urgency: Urgency,
    ) -> impl Fn(&Theme, iced::widget::button::Status) -> iced::widget::button::Style {
        move |iced_theme: &Theme, status| {
            let mut border = match style {
                NotificationStyle::Standalone | NotificationStyle::Toast => {
                    Border::default().rounded(theme.radius.lg)
                }
                NotificationStyle::GroupHeader => Border::default().rounded(iced::border::Radius {
                    top_left: theme.radius.lg,
                    top_right: theme.radius.lg,
                    bottom_left: theme.radius.sm,
                    bottom_right: theme.radius.sm,
                }),
                NotificationStyle::GroupItem => Border::default().rounded(theme.radius.sm),
                NotificationStyle::GroupLast => Border::default().rounded(iced::border::Radius {
                    top_left: 0.0,
                    top_right: 0.0,
                    bottom_left: theme.radius.lg,
                    bottom_right: theme.radius.lg,
                }),
            };

            if urgency == Urgency::Critical {
                border.width = 2.0;
                border.color = iced_theme.palette().danger;
            }

            let mut button_style = iced::widget::button::Style {
                text_color: iced_theme.palette().text,
                border,
                ..iced::widget::button::Style::default()
            };
            match status {
                iced::widget::button::Status::Hovered => {
                    if style == NotificationStyle::Toast {
                        button_style.background = Some(
                            iced_theme
                                .extended_palette()
                                .background
                                .weak
                                .color
                                .scale_alpha(theme.menu.opacity)
                                .into(),
                        );
                    } else {
                        button_style.background = Some(
                            iced_theme
                                .extended_palette()
                                .background
                                .strong
                                .color
                                .scale_alpha(theme.menu.opacity)
                                .into(),
                        );
                    }
                }
                _ => {
                    button_style.background = if style == NotificationStyle::Toast {
                        Some(iced_theme.palette().background.into())
                    } else {
                        Some(iced_theme.extended_palette().background.weak.color.into())
                    }
                }
            }
            button_style
        }
    }

    fn notification_card<'a>(
        &'a self,
        notification: &'a Notification,
        theme: &'a AshellTheme,
        on_press: Message,
        toast: bool,
    ) -> Element<'a, Message> {
        let timestamp_element = if self.config.show_timestamps {
            Some(
                container(
                    text(self.format_timestamp(notification.timestamp)).size(theme.font_size.xs),
                )
                .padding([0., theme.space.xxs]),
            )
        } else {
            None
        };

        let body_element = if (!toast || self.config.show_bodies) && !notification.body.is_empty() {
            Some(
                text(&notification.body)
                    .size(theme.font_size.sm)
                    .wrapping(text::Wrapping::WordOrGlyph),
            )
        } else {
            None
        };

        let notification_id = notification.id;

        let app_icon_button = notification_icon(notification.icon.as_ref());

        let mut card = container(
            column!(
                Row::new()
                    .push(
                        Row::new()
                            .push(app_icon_button)
                            .push(column!(
                                text(&notification.app_name)
                                    .size(theme.font_size.md)
                                    .wrapping(text::Wrapping::WordOrGlyph),
                                timestamp_element
                            ))
                            .width(Length::Fill)
                            .spacing(theme.space.xxs)
                            .padding(theme.space.xs)
                            .align_y(Alignment::Center),
                    )
                    .push(
                        icon_button(theme, StaticIcon::Close)
                            .kind(ButtonKind::Transparent)
                            .hierarchy(ButtonHierarchy::Danger)
                            .on_press(Message::CloseNotificationById(notification_id))
                    ),
                column!(
                    text(&notification.summary).wrapping(text::Wrapping::WordOrGlyph),
                    body_element,
                )
                .spacing(theme.space.xxs)
                .padding(Padding::new(theme.space.xxs).top(0.))
            )
            .spacing(theme.space.xxs),
        );

        if toast {
            card = card.max_height(self.config.toast_max_height);
        }

        button(card)
            .on_press(on_press)
            .width(Length::Fill)
            .style(Self::notification_button_style(
                theme,
                if toast {
                    NotificationStyle::Toast
                } else {
                    NotificationStyle::Standalone
                },
                notification.urgency,
            ))
            .into()
    }

    fn group_item<'a>(
        &'a self,
        notification: &'a Notification,
        is_last: bool,
        theme: &'a AshellTheme,
    ) -> Element<'a, Message> {
        button(
            column!(
                row!(
                    text(&notification.summary)
                        .wrapping(text::Wrapping::WordOrGlyph)
                        .width(Length::Fill),
                    text(self.format_timestamp(notification.timestamp)).size(theme.font_size.sm)
                ),
                text(&notification.body).wrapping(text::Wrapping::WordOrGlyph)
            )
            .padding(theme.space.xs)
            .spacing(theme.space.xs),
        )
        .style(Self::notification_button_style(
            theme,
            if is_last {
                NotificationStyle::GroupLast
            } else {
                NotificationStyle::GroupItem
            },
            notification.urgency,
        ))
        .on_press(Message::NotificationClicked(notification.id))
        .into()
    }

    pub fn view(&'_ self, _: &AshellTheme) -> Element<'_, Message> {
        if !self.notifications.is_empty() {
            icon(StaticIcon::BellBadge).into()
        } else {
            icon(StaticIcon::Bell).into()
        }
    }

    pub fn menu_view<'a>(&'a self, theme: &'a AshellTheme) -> Element<'a, Message> {
        let is_empty = self.notifications.is_empty();

        let content = if is_empty {
            container(text("No notifications").size(theme.font_size.md))
                .width(Length::Fill)
                .center_x(Length::Fill)
                .padding(theme.space.xxl)
                .into()
        } else if self.config.grouped {
            self.grouped_notifications(theme)
        } else {
            self.list_notifications(theme)
        };

        column!(
            Row::new()
                .push(
                    text("Notifications")
                        .width(Length::Fill)
                        .size(theme.font_size.lg)
                )
                .push((!is_empty).then(|| {
                    icon_button(theme, StaticIcon::Delete).on_press(Message::ClearNotifications)
                })),
            container(scrollable(content).spacing(theme.space.xs)).max_height(400.),
        )
        .width(MenuSize::Medium)
        .spacing(theme.space.sm)
        .into()
    }

    pub fn toast_view<'a>(&'a self, theme: &'a AshellTheme) -> Element<'a, Message> {
        if self.toasts.is_empty() {
            return Space::new().into();
        }

        let mut toast_column = column!().spacing(theme.space.sm);

        for &toast_id in &self.toasts {
            if let Some(notification) = self.find_notification(toast_id) {
                toast_column = toast_column.push(self.notification_card(
                    notification,
                    theme,
                    Message::DismissToast(notification.id),
                    true,
                ));
            }
        }

        let v_align = match self.config.toast_position {
            ToastPosition::TopLeft | ToastPosition::TopRight => Alignment::Start,
            ToastPosition::BottomLeft | ToastPosition::BottomRight => Alignment::End,
        };

        // Sensor wraps the padded toast content to report its rendered size.
        // The outer container fills the full-height surface and aligns vertically.
        let toast_content = sensor(
            container(toast_column)
                .width(MenuSize::Medium)
                .padding(theme.space.sm),
        )
        .on_resize(Message::ToastResized);

        container(toast_content)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_y(v_align)
            .into()
    }

    pub fn subscription(&self) -> Subscription<Message> {
        NotificationsService::subscribe().map(Message::Event)
    }

    fn grouped_notifications<'a>(&'a self, theme: &'a AshellTheme) -> Element<'a, Message> {
        let mut content = column!().spacing(theme.space.sm).padding(
            Padding::default()
                .right(theme.space.md)
                .left(theme.space.xs),
        );

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
                    theme,
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
                group_notifications.push(self.group_item(first, false, theme));
                while let Some(notification) = iter.next() {
                    count += 1;
                    has_critical = has_critical || notification.urgency == Urgency::Critical;
                    let is_last = iter.peek().is_none();
                    group_notifications.push(self.group_item(notification, is_last, theme));
                }
            } else {
                for notification in iter {
                    count += 1;
                    has_critical = has_critical || notification.urgency == Urgency::Critical;
                }
                group_notifications.push(self.group_item(first, true, theme));
            }

            let clear_msg = Message::ClearGroup(app_name.clone());
            let toggle_msg = Message::ToggleGroup(app_name.clone());

            let header = row!(
                app_icon,
                text(app_name)
                    .size(theme.font_size.md)
                    .wrapping(text::Wrapping::WordOrGlyph)
                    .width(Length::Fill),
                text(format!("{count} new")),
                icon_button(theme, StaticIcon::Delete)
                    .on_press(clear_msg)
                    .size(ButtonSize::Large)
                    .kind(ButtonKind::Transparent)
                    .hierarchy(ButtonHierarchy::Danger)
            )
            .spacing(theme.space.xs)
            .align_y(Alignment::Center);

            let item = Column::new()
                .push(
                    button(header)
                        .style(Self::notification_button_style(
                            theme,
                            NotificationStyle::GroupHeader,
                            if has_critical {
                                Urgency::Critical
                            } else {
                                Urgency::Normal
                            },
                        ))
                        .on_press(toggle_msg),
                )
                .extend(group_notifications)
                .spacing(theme.space.xxs);

            content = content.push(item);
        }
        content.into()
    }

    fn list_notifications<'a>(&'a self, theme: &'a AshellTheme) -> Element<'a, Message> {
        Column::with_children(
            self.notifications
                .iter()
                .map(|notification| {
                    self.notification_card(
                        notification,
                        theme,
                        Message::NotificationClicked(notification.id),
                        false,
                    )
                })
                .collect::<Vec<Element<'a, Message>>>(),
        )
        .padding(
            Padding::default()
                .right(theme.space.md)
                .left(theme.space.xs),
        )
        .spacing(theme.space.sm)
        .into()
    }

    pub fn toast_position(&self) -> ToastPosition {
        self.config.toast_position
    }
}
