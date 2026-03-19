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
use freedesktop_icons::lookup;
use iced::{
    Alignment, Background, Border, Color, Element, Length, Radius, Subscription, Task, Theme,
    widget::{
        Space, button, column, container, horizontal_rule, image, row, scrollable, svg, text,
    },
};
use linicon_theme::get_icon_theme;
use log::error;
use std::{
    collections::{HashMap, HashSet, VecDeque},
    path::{Path, PathBuf},
    time::Duration,
};
use zbus::Connection;
use zbus::zvariant::OwnedValue;

const ICON_SIZE: f32 = 20.0;
const HORIZONTAL_RULE_HEIGHT: f32 = 0.2;

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

#[derive(Debug, Clone)]
enum NotificationIcon {
    Raster(image::Handle),
    Vector(svg::Handle),
    Bell,
}

fn resolve_notification_icon(notification: &Notification) -> NotificationIcon {
    match resolve_notification_icon_path(
        &notification.app_name,
        &notification.app_icon,
        &notification.hints,
    ) {
        Some(path) => {
            let is_svg = Path::new(&path)
                .extension()
                .and_then(|ext| ext.to_str())
                .is_some_and(|ext| ext.eq_ignore_ascii_case("svg"));

            if is_svg {
                NotificationIcon::Vector(svg::Handle::from_path(path))
            } else {
                NotificationIcon::Raster(image::Handle::from_path(path))
            }
        }
        None => NotificationIcon::Bell,
    }
}

fn non_empty_owned_value_string(value: Option<&OwnedValue>) -> Option<String> {
    value
        .and_then(|v| v.clone().try_into().ok())
        .map(|s: String| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn parse_file_url(value: &str) -> Option<PathBuf> {
    if !value.starts_with("file://") {
        return None;
    }

    let decoded = url::Url::parse(value).ok()?.to_file_path().ok()?;
    decoded.exists().then_some(decoded)
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

fn resolve_notification_icon_path(
    app_name: &str,
    app_icon: &str,
    hints: &HashMap<String, OwnedValue>,
) -> Option<String> {
    let mut candidates = Vec::new();

    if !app_icon.trim().is_empty() {
        candidates.push(app_icon.trim().to_string());
    }

    for key in [
        "image-path",
        "image_path",
        "icon-name",
        "icon_name",
        "desktop-entry",
    ] {
        if let Some(value) = non_empty_owned_value_string(hints.get(key)) {
            candidates.push(value);
        }
    }

    if !app_name.trim().is_empty() {
        candidates.push(app_name.trim().to_string());
    }

    for candidate in candidates {
        if let Some(path) = parse_file_url(&candidate) {
            return Some(path.to_string_lossy().into_owned());
        }

        let candidate_path = PathBuf::from(&candidate);
        if (candidate.contains('/') || candidate.starts_with('.')) && candidate_path.exists() {
            return Some(candidate_path.to_string_lossy().into_owned());
        }

        if let Some(path) = find_icon_path(&candidate) {
            return Some(path.to_string_lossy().into_owned());
        }

        if let Some(stripped) = candidate.strip_suffix(".desktop")
            && let Some(path) = find_icon_path(stripped)
        {
            return Some(path.to_string_lossy().into_owned());
        }
    }

    None
}

fn notification_icon_with_frame<'a, M: 'a>(icon_kind: &NotificationIcon) -> Element<'a, M> {
    let inner: Element<'a, M> = match icon_kind {
        NotificationIcon::Vector(handle) => svg(handle.clone())
            .width(Length::Fixed(ICON_SIZE))
            .height(Length::Fixed(ICON_SIZE))
            .into(),
        NotificationIcon::Raster(handle) => image(handle.clone())
            .width(Length::Fixed(ICON_SIZE))
            .height(Length::Fixed(ICON_SIZE))
            .into(),
        NotificationIcon::Bell => icon(StaticIcon::Bell)
            .size(ICON_SIZE)
            .style(palette_text_style)
            .into(),
    };
    container(inner)
        .center_x(Length::Fixed(ICON_SIZE))
        .center_y(Length::Fixed(ICON_SIZE))
        .width(Length::Fixed(ICON_SIZE))
        .height(Length::Fixed(ICON_SIZE))
        .into()
}

fn icon_button_style(theme: &Theme, status: button::Status) -> button::Style {
    button::Style {
        background: Some(Background::Color(Color::TRANSPARENT)),
        text_color: match status {
            button::Status::Hovered => theme.palette().danger,
            _ => theme.palette().text,
        },
        ..Default::default()
    }
}

fn clear_button_style(radius: u16) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |iced_theme: &Theme, _status| button::Style {
        background: Some(Background::Color(Color::TRANSPARENT)),
        text_color: iced_theme.palette().text,
        border: Border::default().rounded(radius),
        ..button::Style::default()
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

fn toast_timeout(expire_timeout: i32, default_timeout_ms: u64) -> Option<Duration> {
    match expire_timeout {
        -1 => Some(Duration::from_millis(default_timeout_ms)),
        0 => None,
        t if t > 0 => Some(Duration::from_millis(t as u64)),
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

pub struct Notifications {
    config: NotificationsModuleConfig,
    connection: Option<Connection>,
    notifications: VecDeque<Notification>,
    expanded_groups: HashSet<String>,
    toasts: VecDeque<u32>,
    icons: HashMap<u32, NotificationIcon>,
}

impl Notifications {
    pub fn new(config: NotificationsModuleConfig) -> Self {
        Self {
            config,
            connection: None,
            notifications: VecDeque::new(),
            expanded_groups: HashSet::new(),
            toasts: VecDeque::new(),
            icons: HashMap::new(),
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

    fn grouped_notifications_by_app(&self) -> Vec<(String, Vec<&Notification>)> {
        let mut grouped: HashMap<String, Vec<&Notification>> = HashMap::new();
        for notification in &self.notifications {
            grouped
                .entry(notification.app_name.clone())
                .or_default()
                .push(notification);
        }

        let mut grouped: Vec<(String, Vec<&Notification>)> = grouped.into_iter().collect();
        grouped.sort_by(|(left, _), (right, _)| left.cmp(right));
        grouped
    }

    fn icon_for_notification(&self, id: u32) -> &NotificationIcon {
        self.icons.get(&id).unwrap_or(&NotificationIcon::Bell)
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
                let was_empty = self.toasts.is_empty();
                while self.toasts.len() >= self.config.toast_max_visible {
                    self.toasts.pop_front();
                }
                self.toasts.push_back(notification.id);

                let notification_id = notification.id;
                let timeout = toast_timeout(
                    notification.expire_timeout,
                    self.config.toast_default_timeout,
                );

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
                let was_showing = self.remove_toast(id);
                self.hide_toasts_if_empty(was_showing)
            }
        }
    }

    fn apply_update_event(&mut self, update_event: NotificationEvent) {
        match update_event {
            NotificationEvent::Received(notification) => {
                self.icons
                    .insert(notification.id, resolve_notification_icon(&notification));
                self.notifications.push_front(*notification);
            }
            NotificationEvent::Closed(id) => {
                self.icons.remove(&id);
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
                self.icons.clear();
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
                for id in &group_ids {
                    self.icons.remove(id);
                }
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
                            .scale_alpha(theme.menu.opacity),
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
                            .scale_alpha(theme.menu.opacity),
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
                    .scale_alpha(theme.menu.opacity),
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
        let icon = self.icon_for_notification(notification_id);
        let app_icon_button = button(notification_icon_with_frame(icon))
            .on_press(Message::CloseNotificationById(notification_id))
            .style(icon_button_style)
            .padding(0);

        let card = container(
            column!(
                row!(
                    app_icon_button,
                    container(
                        text(&notification.app_name)
                            .size(theme.font_size.md)
                            .wrapping(text::Wrapping::WordOrGlyph)
                            .style(palette_text_style)
                    )
                    .width(Length::Fill),
                    timestamp_element,
                )
                .spacing(theme.space.xs)
                .align_y(Alignment::Center),
                text(&notification.summary)
                    .size(theme.font_size.sm)
                    .wrapping(text::Wrapping::WordOrGlyph)
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
                    container(
                        text(&notification.summary)
                            .size(theme.font_size.md)
                            .wrapping(text::Wrapping::WordOrGlyph)
                            .style(strong_text_style)
                    )
                    .width(Length::Fill),
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
        if !self.notifications.is_empty() {
            icon(StaticIcon::BellBadge).into()
        } else {
            icon(StaticIcon::Bell).into()
        }
    }

    pub fn subscription(&self) -> Subscription<Message> {
        NotificationsService::subscribe().map(Message::Event)
    }

    fn grouped_notifications<'a>(&'a self, theme: &'a AshellTheme) -> Element<'a, Message> {
        let mut content = column!().spacing(theme.space.sm);
        for (app_name, notifications) in self.grouped_notifications_by_app() {
            let is_expanded = self.expanded_groups.contains(&app_name);
            let app_icon: Element<'a, Message> = notifications
                .first()
                .map(|notification| self.icon_for_notification(notification.id))
                .map(notification_icon_with_frame)
                .unwrap_or_else(|| icon(StaticIcon::Bell).size(ICON_SIZE).into());

            let clear_msg = Message::ClearGroup(app_name.clone());
            let toggle_msg = Message::ToggleGroup(app_name.clone());

            let header = row!(
                button(app_icon)
                    .style(icon_button_style)
                    .on_press(clear_msg),
                container(
                    text(app_name)
                        .size(theme.font_size.md)
                        .wrapping(text::Wrapping::WordOrGlyph)
                        .style(palette_text_style)
                )
                .width(Length::Fill),
                text(format!("{} new", notifications.len()))
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
        let is_empty = self.notifications.is_empty();
        let content = if is_empty {
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
                if !is_empty {
                    container(
                        button("Clear")
                            .style(clear_button_style(theme.radius.md))
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
            .width(self.config.toast_width);

        for &toast_id in &self.toasts {
            if let Some(notification) = self.find_notification(toast_id) {
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
            .padding(theme.space.sm)
            .align_x(h_align)
            .align_y(v_align)
            .into()
    }

    pub fn toast_layer_size(&self, theme: &AshellTheme) -> (u32, u32) {
        let n = self.config.toast_max_visible as u32;
        let margin = theme.space.sm as u32;
        let line_height = theme.font_size.sm as u32 + theme.space.xxs as u32;
        let card_height = ICON_SIZE as u32
            + (self.config.toast_summary_line_budget + self.config.toast_body_line_budget)
                * line_height
            + 3 * theme.space.sm as u32;
        let spacing = theme.space.sm as u32;
        let width = self.config.toast_width as u32 + 2 * margin;
        let height = n * card_height + n.saturating_sub(1) * spacing + 2 * margin;
        (width, height)
    }

    pub fn toast_position(&self) -> ToastPosition {
        self.config.toast_position
    }
}
