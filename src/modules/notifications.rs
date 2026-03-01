use crate::{
    components::icons::{StaticIcon, icon},
    config::{NotificationsModuleConfig, ToastPosition},
    menu::MenuSize,
    services::{
        ReadOnlyService, ServiceEvent,
        notifications::{
            Notification, NotificationsService,
            dbus::{NotificationDaemon, NotificationEvent},
        },
    },
    theme::AshellTheme,
};
use chrono::{DateTime, Local};
use iced::{
    Alignment, Background, Border, Color, Element, Length, Radius, Subscription, Task, Theme,
    widget::{
        Space, button, column, container, horizontal_rule, image, row, scrollable, svg, text,
    },
};
use log::error;
use std::{
    collections::{HashMap, HashSet},
    path::Path,
    time::Duration,
};
use zbus::Connection;

// Constants for UI dimensions and styling
const ICON_SIZE: f32 = 20.0;
const HORIZONTAL_RULE_HEIGHT: f32 = 0.2;

// --- Shared text style helpers ---

fn strong_text_style(theme: &Theme) -> text::Style {
    text::Style {
        color: Some(theme.extended_palette().secondary.strong.text),
    }
}

fn weak_text_style(theme: &Theme) -> text::Style {
    text::Style {
        color: Some(theme.extended_palette().secondary.weak.text),
    }
}

fn palette_text_style(theme: &Theme) -> text::Style {
    text::Style {
        color: Some(theme.palette().text),
    }
}

// --- Shared icon helper ---

#[derive(Debug, Clone)]
enum CachedNotificationIcon {
    Raster(image::Handle),
    Vector(svg::Handle),
    Bell,
}

fn resolve_notification_icon(notification: &Notification) -> CachedNotificationIcon {
    if let Some(path) = notification.resolved_icon_path.as_ref() {
        let is_svg = Path::new(path)
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| ext.eq_ignore_ascii_case("svg"));

        if is_svg {
            CachedNotificationIcon::Vector(svg::Handle::from_path(path.clone()))
        } else {
            CachedNotificationIcon::Raster(image::Handle::from_path(path.clone()))
        }
    } else {
        CachedNotificationIcon::Bell
    }
}

fn notification_icon<'a, M: 'a>(cached_icon: Option<&CachedNotificationIcon>) -> Element<'a, M> {
    match cached_icon {
        Some(CachedNotificationIcon::Vector(handle)) => svg(handle.clone())
            .width(Length::Fixed(ICON_SIZE))
            .height(Length::Fixed(ICON_SIZE))
            .into(),
        Some(CachedNotificationIcon::Raster(handle)) => image(handle.clone())
            .width(Length::Fixed(ICON_SIZE))
            .height(Length::Fixed(ICON_SIZE))
            .into(),
        Some(CachedNotificationIcon::Bell) | None => icon(StaticIcon::Bell)
            .size(ICON_SIZE)
            .style(palette_text_style)
            .into(),
    }
}

fn notification_icon_with_frame<'a, M: 'a>(
    cached_icon: Option<&CachedNotificationIcon>,
) -> Element<'a, M> {
    container(notification_icon(cached_icon))
        .center_x(Length::Fixed(ICON_SIZE))
        .center_y(Length::Fixed(ICON_SIZE))
        .width(Length::Fixed(ICON_SIZE))
        .height(Length::Fixed(ICON_SIZE))
        .into()
}

// Invokes the first action (if present) and closes the notification via D-Bus.
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
            id
        },
        Message::NotificationClosed,
    )
}

#[derive(Debug, Clone)]
pub enum Message {
    ConfigReloaded(NotificationsModuleConfig),
    NotificationClicked(u32),
    NotificationClosed(u32),
    CloseNotificationById(u32),
    ClearNotifications,
    NotificationsCleared,
    ClearGroup(String),
    GroupCleared(String),
    Event(ServiceEvent<NotificationsService>),
    ToggleGroup(String),
    ExpireToast(u32),
    DismissToast(u32),
}
pub enum NotificationStyle {
    Rectangular,
    Rounded,
    BottomRounded,
}
pub enum Action {
    None,
    Task(Task<Message>),
    Show(Task<Message>),
    Hide(Task<Message>),
}

struct ToastEntry {
    id: u32,
}

pub struct Notifications {
    config: NotificationsModuleConfig,
    notifications: Vec<Notification>,
    notification_icons: HashMap<u32, CachedNotificationIcon>,
    service: Option<NotificationsService>,
    expanded_groups: HashSet<String>,
    toasts: Vec<ToastEntry>,
}

impl Notifications {
    pub fn new(config: NotificationsModuleConfig) -> Self {
        Self {
            config,
            notifications: Vec::new(),
            notification_icons: HashMap::new(),
            service: None,
            expanded_groups: HashSet::new(),
            toasts: Vec::new(),
        }
    }

    fn sync_notification_icons(&mut self) {
        let desired_ids: HashSet<u32> = self.notifications.iter().map(|n| n.id).collect();
        self.notification_icons
            .retain(|id, _| desired_ids.contains(id));

        for notification in &self.notifications {
            self.notification_icons
                .entry(notification.id)
                .or_insert_with(|| resolve_notification_icon(notification));
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
                    self.notifications = service.notifications.values().cloned().collect();
                    self.notifications
                        .sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
                    self.sync_notification_icons();
                    self.service = Some(service);
                    Action::None
                }
                ServiceEvent::Update(update_event) => {
                    if let Some(service) = self.service.as_mut() {
                        let toast_action = if self.config.toast {
                            match &update_event {
                                NotificationEvent::Received(notification) => {
                                    let was_empty = self.toasts.is_empty();
                                    while self.toasts.len() >= self.config.toast_max_visible {
                                        self.toasts.remove(0);
                                    }
                                    self.toasts.push(ToastEntry {
                                        id: notification.id,
                                    });

                                    let notification_id = notification.id;
                                    let timeout = match notification.expire_timeout {
                                        -1 => Some(Duration::from_millis(
                                            self.config.toast_default_timeout,
                                        )),
                                        0 => None,
                                        t if t > 0 => Some(Duration::from_millis(t as u64)),
                                        _ => None,
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

                                    if was_empty {
                                        Action::Show(timer_task)
                                    } else {
                                        Action::Task(timer_task)
                                    }
                                }
                                NotificationEvent::Closed(id) => {
                                    let id = *id;
                                    let was_showing = !self.toasts.is_empty();
                                    self.toasts.retain(|t| t.id != id);
                                    if was_showing && self.toasts.is_empty() {
                                        Action::Hide(Task::none())
                                    } else {
                                        Action::None
                                    }
                                }
                            }
                        } else {
                            Action::None
                        };

                        service.update(update_event);
                        self.notifications = service.notifications.values().cloned().collect();
                        self.notifications
                            .sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
                        self.sync_notification_icons();

                        toast_action
                    } else {
                        Action::None
                    }
                }
                ServiceEvent::Error(_) => Action::None,
            },
            Message::NotificationClicked(id) => {
                let connection = self.service.as_ref().map(|s| s.connection.clone());
                let action_key = self
                    .notifications
                    .iter()
                    .find(|n| n.id == id)
                    .filter(|n| !n.actions.is_empty())
                    .and_then(|n| n.actions.first())
                    .cloned();

                Action::Task(invoke_and_close_task(connection, id, action_key))
            }
            Message::NotificationClosed(id) => {
                self.notifications.retain(|n| n.id != id);
                self.notification_icons.remove(&id);
                Action::None
            }
            Message::ClearNotifications => {
                let connection = self.service.as_ref().map(|s| s.connection.clone());
                let notification_ids: Vec<u32> = self.notifications.iter().map(|n| n.id).collect();

                Action::Task(Task::perform(
                    async move {
                        if let Some(connection) = connection {
                            for id in notification_ids {
                                if let Err(e) =
                                    NotificationDaemon::close_notification_by_id(&connection, id)
                                        .await
                                {
                                    error!("Failed to close notification id {}: {}", id, e);
                                }
                            }
                        }
                    },
                    |_| Message::NotificationsCleared,
                ))
            }
            Message::NotificationsCleared => {
                self.notifications.clear();
                self.notification_icons.clear();
                let had_toasts = !self.toasts.is_empty();
                self.toasts.clear();
                if had_toasts {
                    Action::Hide(Task::none())
                } else {
                    Action::None
                }
            }
            Message::ClearGroup(app_name) => {
                let connection = self.service.as_ref().map(|s| s.connection.clone());
                let notification_ids: Vec<u32> = self
                    .notifications
                    .iter()
                    .filter(|n| n.app_name == app_name)
                    .map(|n| n.id)
                    .collect();

                Action::Task(Task::perform(
                    async move {
                        if let Some(connection) = connection {
                            for id in notification_ids {
                                if let Err(e) =
                                    NotificationDaemon::close_notification_by_id(&connection, id)
                                        .await
                                {
                                    error!("Failed to close notification id {}: {}", id, e);
                                }
                            }
                        }
                        app_name
                    },
                    Message::GroupCleared,
                ))
            }
            Message::GroupCleared(app_name) => {
                let group_ids: Vec<u32> = self
                    .notifications
                    .iter()
                    .filter(|n| n.app_name == app_name)
                    .map(|n| n.id)
                    .collect();
                self.notifications.retain(|n| n.app_name != app_name);
                for id in group_ids.iter() {
                    self.notification_icons.remove(id);
                }
                self.expanded_groups.remove(&app_name);
                let had_toasts = !self.toasts.is_empty();
                self.toasts.retain(|t| !group_ids.contains(&t.id));
                if had_toasts && self.toasts.is_empty() {
                    Action::Hide(Task::none())
                } else {
                    Action::None
                }
            }
            Message::ToggleGroup(app_name) => {
                if !self.expanded_groups.remove(&app_name) {
                    self.expanded_groups.insert(app_name);
                }
                Action::None
            }
            Message::ExpireToast(id) => {
                self.toasts.retain(|t| t.id != id);
                if self.toasts.is_empty() {
                    Action::Hide(Task::none())
                } else {
                    Action::None
                }
            }
            Message::CloseNotificationById(id) => {
                let connection = self.service.as_ref().map(|s| s.connection.clone());
                self.notifications.retain(|n| n.id != id);
                self.notification_icons.remove(&id);
                let had_toasts = !self.toasts.is_empty();
                self.toasts.retain(|t| t.id != id);

                let task = Task::perform(
                    async move {
                        if let Some(connection) = connection
                            && let Err(e) =
                                NotificationDaemon::close_notification_by_id(&connection, id).await
                        {
                            error!("Failed to close notification id {}: {}", id, e);
                        }
                        id
                    },
                    Message::NotificationClosed,
                );

                if self.toasts.is_empty() && had_toasts {
                    Action::Hide(task)
                } else {
                    Action::Task(task)
                }
            }
            Message::DismissToast(id) => {
                let connection = self.service.as_ref().map(|s| s.connection.clone());
                let action_key = self
                    .notifications
                    .iter()
                    .find(|n| n.id == id)
                    .filter(|n| !n.actions.is_empty())
                    .and_then(|n| n.actions.first())
                    .cloned();

                self.toasts.retain(|t| t.id != id);
                let task = invoke_and_close_task(connection, id, action_key);

                if self.toasts.is_empty() {
                    Action::Hide(task)
                } else {
                    Action::Task(task)
                }
            }
        }
    }

    fn format_timestamp(&self, timestamp: std::time::SystemTime) -> String {
        let datetime: DateTime<Local> = timestamp.into();
        datetime.format(&self.config.format).to_string()
    }

    fn notification_button_style(
        theme: &AshellTheme,
        style: NotificationStyle,
    ) -> impl Fn(&Theme, iced::widget::button::Status) -> iced::widget::button::Style {
        let theme = theme.clone();
        move |iced_theme: &Theme, status| {
            let mut button_style = iced::widget::button::Style::default();
            match status {
                iced::widget::button::Status::Hovered => {
                    match style {
                        NotificationStyle::Rectangular => (),
                        NotificationStyle::Rounded => {
                            button_style.border = Border::default().rounded(theme.radius.md)
                        }
                        NotificationStyle::BottomRounded => {
                            button_style.border = Border::default().rounded(
                                Radius::default()
                                    .bottom_left(theme.radius.md)
                                    .bottom_right(theme.radius.md),
                            )
                        }
                    }
                    button_style.background = Some(Background::Color(
                        iced_theme
                            .extended_palette()
                            .background
                            .weak
                            .color
                            .scale_alpha(theme.opacity),
                    ));
                }
                _ => {
                    button_style.background = Some(Background::Color(iced::Color::TRANSPARENT));
                }
            }
            button_style
        }
    }

    fn group_header_button_style(
        theme: &AshellTheme,
    ) -> impl Fn(&Theme, iced::widget::button::Status) -> iced::widget::button::Style {
        let theme = theme.clone();
        move |iced_theme: &Theme, status| {
            let mut style = iced::widget::button::Style::default();
            let border = Border::default().rounded(
                Radius::default()
                    .top_left(theme.radius.md)
                    .top_right(theme.radius.md),
            );
            match status {
                iced::widget::button::Status::Hovered => {
                    style.background = Some(Background::Color(
                        iced_theme
                            .extended_palette()
                            .background
                            .weak
                            .color
                            .scale_alpha(theme.opacity),
                    ));
                    style.border = border;
                }
                _ => {
                    style.background = Some(Background::Color(iced::Color::TRANSPARENT));
                    style.border = border;
                }
            }
            style
        }
    }

    fn item_container_style(theme: &AshellTheme) -> impl Fn(&Theme) -> container::Style {
        let theme = theme.clone();
        move |app_theme: &Theme| container::Style {
            background: Background::Color(
                app_theme
                    .extended_palette()
                    .secondary
                    .strong
                    .color
                    .scale_alpha(theme.opacity),
            )
            .into(),
            border: Border::default().rounded(theme.radius.md),
            ..container::Style::default()
        }
    }

    fn build_notification_card<'a>(
        &'a self,
        notification: &'a Notification,
        theme: &'a AshellTheme,
        show_body: bool,
        on_press: Message,
    ) -> Element<'a, Message> {
        let timestamp_element: Element<'_, Message> = if self.config.show_timestamps {
            text(self.format_timestamp(notification.timestamp))
                .size(theme.font_size.sm)
                .style(weak_text_style)
                .into()
        } else {
            Space::with_width(Length::Shrink).into()
        };

        let body_element: Element<'_, Message> = if show_body && !notification.body.is_empty() {
            text(&notification.body)
                .size(theme.font_size.sm)
                .wrapping(text::Wrapping::WordOrGlyph)
                .style(strong_text_style)
                .into()
        } else {
            Space::with_height(Length::Shrink).into()
        };

        let notification_id = notification.id;
        let app_icon_button = button(notification_icon_with_frame(
            self.notification_icons.get(&notification_id),
        ))
        .on_press(Message::CloseNotificationById(notification_id))
        .style(move |iced_theme: &Theme, status| button::Style {
            background: Some(Background::Color(Color::TRANSPARENT)),
            text_color: match status {
                button::Status::Hovered => iced_theme.palette().danger,
                _ => iced_theme.palette().text,
            },
            ..Default::default()
        })
        .padding(0);

        let card = container(
            column!(
                row!(
                    app_icon_button,
                    text(&notification.app_name)
                        .size(theme.font_size.md)
                        .style(palette_text_style),
                    Space::with_width(Length::Fill),
                    timestamp_element,
                )
                .spacing(theme.space.xs)
                .align_y(Alignment::Center),
                text(&notification.summary)
                    .size(theme.font_size.sm)
                    .style(strong_text_style),
                body_element,
            )
            .spacing(theme.space.xxs),
        )
        .style(Self::item_container_style(theme))
        .padding(theme.space.sm)
        .width(Length::Fill);

        button(card)
            .on_press(on_press)
            .style(Self::notification_button_style(
                theme,
                NotificationStyle::Rounded,
            ))
            .padding(0)
            .into()
    }

    fn build_full_item<'a>(
        &'a self,
        notification: &'a Notification,
        is_last: bool,
        theme: &'a AshellTheme,
    ) -> Element<'a, Message> {
        button(
            column!(
                row!(
                    text(&notification.summary)
                        .size(theme.font_size.md)
                        .style(strong_text_style),
                    Space::with_width(Length::Fill),
                    text(self.format_timestamp(notification.timestamp))
                        .size(theme.font_size.sm)
                        .style(weak_text_style)
                ),
                text(&notification.body)
                    .size(theme.font_size.sm)
                    .wrapping(text::Wrapping::WordOrGlyph)
                    .style(weak_text_style)
            )
            .padding(theme.space.xs)
            .spacing(theme.space.xs),
        )
        .width(Length::Fill)
        .style(Self::notification_button_style(
            theme,
            if is_last {
                NotificationStyle::BottomRounded
            } else {
                NotificationStyle::Rectangular
            },
        ))
        .on_press(Message::NotificationClicked(notification.id))
        .into()
    }

    pub fn view(&'_ self, _: &AshellTheme) -> Element<'_, Message> {
        if self.notifications.is_empty() {
            icon(StaticIcon::Bell).into()
        } else {
            icon(StaticIcon::BellBadge).into()
        }
    }

    pub fn subscription(&self) -> Subscription<Message> {
        NotificationsService::subscribe().map(Message::Event)
    }

    fn grouped_notifications<'a>(&'a self, theme: &'a AshellTheme) -> Element<'a, Message> {
        let mut grouped: HashMap<String, Vec<&Notification>> = HashMap::new();
        for notification in &self.notifications {
            grouped
                .entry(notification.app_name.clone())
                .or_default()
                .push(notification);
        }

        let mut content = column!().spacing(theme.space.sm);
        for (app_name, notifications) in grouped {
            let is_expanded = self.expanded_groups.contains(&app_name);
            let app_icon: Element<'a, Message> = notifications
                .first()
                .map(|n| notification_icon_with_frame(self.notification_icons.get(&n.id)))
                .unwrap_or_else(|| icon(StaticIcon::Bell).size(ICON_SIZE).into());

            let clear_msg = Message::ClearGroup(app_name.clone());
            let toggle_msg = Message::ToggleGroup(app_name.clone());

            let header = row!(
                button(app_icon)
                    .style(move |iced_theme: &Theme, status| button::Style {
                        background: Some(Background::Color(Color::TRANSPARENT)),
                        text_color: match status {
                            button::Status::Hovered => iced_theme.palette().danger,
                            _ => iced_theme.palette().text,
                        },
                        ..Default::default()
                    })
                    .on_press(clear_msg),
                text(app_name)
                    .size(theme.font_size.md)
                    .style(palette_text_style),
                Space::with_width(Length::Fill),
                text(format!("{} new", notifications.len(),))
                    .size(theme.font_size.sm)
                    .style(weak_text_style),
            )
            .spacing(theme.space.xs)
            .align_y(Alignment::Center);

            let mut preview = column!();
            if is_expanded {
                for (i, notification) in notifications.iter().enumerate() {
                    let is_last = i == notifications.len() - 1;
                    preview = preview.push(column!(
                        horizontal_rule(HORIZONTAL_RULE_HEIGHT),
                        self.build_full_item(notification, is_last, theme)
                    ));
                }
            } else if let Some(first_notification) = notifications.first() {
                preview = preview.push(horizontal_rule(HORIZONTAL_RULE_HEIGHT));
                preview = preview.push(self.build_full_item(first_notification, true, theme))
            }

            let item = column!(
                button(header)
                    .width(Length::Fill)
                    .style(Self::group_header_button_style(theme))
                    .on_press(toggle_msg),
                preview
            );
            content = content.push(
                container(item)
                    .style(Self::item_container_style(theme))
                    .width(Length::Fill),
            );
        }
        content.into()
    }

    fn list_notifications<'a>(&'a self, theme: &'a AshellTheme) -> Element<'a, Message> {
        let mut notifications_refs: Vec<&Notification> = self.notifications.iter().collect();

        if let Some(max) = self.config.max_notifications {
            notifications_refs.truncate(max);
        }

        let mut content = column!().spacing(theme.space.sm);
        for notification in notifications_refs {
            content = content.push(self.build_notification_card(
                notification,
                theme,
                self.config.show_bodies,
                Message::NotificationClicked(notification.id),
            ));
        }
        content.into()
    }

    pub fn menu_view<'a>(&'a self, theme: &'a AshellTheme) -> Element<'a, Message> {
        let content = if self.notifications.is_empty() {
            container(text("No notifications").size(theme.font_size.md))
                .width(Length::Fill)
                .height(Length::Fixed(self.config.empty_state_height))
                .center_x(Length::Fill)
                .center_y(Length::Fixed(self.config.empty_state_height))
                .into()
        } else if self.config.grouped {
            self.grouped_notifications(theme)
        } else {
            self.list_notifications(theme)
        };
        column!(
            row!(
                text("Notifications").size(theme.font_size.lg),
                if !self.notifications.is_empty() {
                    container(
                        button("Clear")
                            .style(move |iced_theme: &Theme, _status| button::Style {
                                background: Some(Background::Color(Color::TRANSPARENT)),
                                text_color: (iced_theme.palette().text),
                                border: Border::default().rounded(theme.radius.md),
                                ..button::Style::default()
                            })
                            .on_press(Message::ClearNotifications),
                    )
                    .width(Length::Fill)
                    .align_x(Alignment::End)
                } else {
                    container(Space::new(Length::Fill, Length::Shrink))
                        .width(Length::Fill)
                        .align_x(Alignment::End)
                }
            ),
            scrollable(content).scrollbar_width(0.0).scroller_width(0.0),
        )
        .width(MenuSize::Medium)
        .spacing(theme.space.sm)
        .into()
    }

    pub fn toast_view<'a>(&'a self, theme: &'a AshellTheme) -> Element<'a, Message> {
        if self.toasts.is_empty() {
            return Space::new(Length::Fill, Length::Fill).into();
        }

        let mut toast_column = column!()
            .spacing(theme.space.sm)
            .padding(theme.space.sm)
            .width(380);

        for toast_entry in &self.toasts {
            if let Some(notification) = self.notifications.iter().find(|n| n.id == toast_entry.id) {
                toast_column = toast_column.push(self.build_notification_card(
                    notification,
                    theme,
                    true,
                    Message::DismissToast(notification.id),
                ));
            }
        }

        let (h_align, v_align) = match self.config.toast_position {
            ToastPosition::TopLeft => (Alignment::Start, Alignment::Start),
            ToastPosition::TopRight => (Alignment::End, Alignment::Start),
            ToastPosition::BottomLeft => (Alignment::Start, Alignment::End),
            ToastPosition::BottomRight => (Alignment::End, Alignment::End),
        };

        container(toast_column)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(h_align)
            .align_y(v_align)
            .into()
    }

    /// Maximum number of toasts that may be visible (from configuration).
    pub fn toast_max_visible(&self) -> usize {
        self.config.toast_max_visible
    }

    /// Configured corner where toasts appear.
    pub fn toast_position(&self) -> ToastPosition {
        self.config.toast_position
    }
}
