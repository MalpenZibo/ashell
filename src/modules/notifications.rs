use crate::{
    components::icons::{DynamicIcon, Icon, StaticIcon, icon},
    config::NotificationsModuleConfig,
    menu::MenuSize,
    services::{
        ReadOnlyService, ServiceEvent,
        notifications::{Notification, NotificationsService, dbus::NotificationDaemon},
    },
    theme::AshellTheme,
};
use chrono::{DateTime, Local};
use iced::{
    Alignment, Background, Border, Color, Element, Length, Radius, Subscription, Task, Theme,
    widget::{Space, button, column, container, horizontal_rule, row, scrollable, text},
};
use log::error;
use std::collections::{HashMap, HashSet};

// Constants for UI dimensions and styling
const EMPTY_STATE_HEIGHT: f32 = 300.0;
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

fn notification_icon<'a, M: 'a>(app_icon: &str) -> Element<'a, M> {
    if app_icon.is_empty() {
        icon(StaticIcon::Bell)
            .size(ICON_SIZE)
            .style(palette_text_style)
            .into()
    } else {
        DynamicIcon(app_icon.to_string())
            .to_text()
            .size(ICON_SIZE)
            .style(palette_text_style)
            .into()
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    ConfigReloaded(NotificationsModuleConfig),
    NotificationClicked(u32),
    NotificationClosed(u32),
    ClearNotifications,
    NotificationsCleared,
    ClearGroup(String),
    GroupCleared(String),
    Event(ServiceEvent<NotificationsService>),
    ToggleGroup(String),
}

pub struct Notifications {
    config: NotificationsModuleConfig,
    notifications: Vec<Notification>,
    service: Option<NotificationsService>,
    collapsed_groups: HashSet<String>,
}

impl Notifications {
    pub fn new(config: NotificationsModuleConfig) -> Self {
        Self {
            config,
            notifications: Vec::new(),
            service: None,
            collapsed_groups: HashSet::new(),
        }
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::ConfigReloaded(notifications_module_config) => {
                self.config = notifications_module_config;
                Task::none()
            }
            Message::Event(event) => match event {
                ServiceEvent::Init(service) => {
                    self.service = Some(service);
                    Task::none()
                }
                ServiceEvent::Update(update_event) => {
                    if let Some(service) = self.service.as_mut() {
                        service.update(update_event);
                        self.notifications = service.notifications.values().cloned().collect();
                        self.notifications
                            .sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
                    }
                    Task::none()
                }
                ServiceEvent::Error(_) => Task::none(),
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

                Task::perform(
                    async move {
                        if let Some(connection) = connection {
                            if let Some(action_key) = action_key
                                && let Err(e) =
                                    NotificationDaemon::invoke_action(&connection, id, action_key)
                                        .await
                            {
                                error!("Failed to invoke notification action for id {}: {}", id, e);
                            }
                            if let Err(e) =
                                NotificationDaemon::close_notification_by_id(&connection, id).await
                            {
                                error!("Failed to close notification id {}: {}", id, e);
                            }
                        }
                        id
                    },
                    Message::NotificationClosed,
                )
            }
            Message::NotificationClosed(id) => {
                self.notifications.retain(|n| n.id != id);
                Task::none()
            }
            Message::ClearNotifications => {
                let connection = self.service.as_ref().map(|s| s.connection.clone());
                let notification_ids: Vec<u32> = self.notifications.iter().map(|n| n.id).collect();

                Task::perform(
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
                )
            }
            Message::NotificationsCleared => {
                self.notifications.clear();
                Task::none()
            }
            Message::ClearGroup(app_name) => {
                let connection = self.service.as_ref().map(|s| s.connection.clone());
                let notification_ids: Vec<u32> = self
                    .notifications
                    .iter()
                    .filter(|n| n.app_name == app_name)
                    .map(|n| n.id)
                    .collect();

                Task::perform(
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
                )
            }
            Message::GroupCleared(app_name) => {
                self.notifications.retain(|n| n.app_name != app_name);
                self.collapsed_groups.remove(&app_name);
                Task::none()
            }
            Message::ToggleGroup(app_name) => {
                if !self.collapsed_groups.remove(&app_name) {
                    self.collapsed_groups.insert(app_name);
                }
                Task::none()
            }
        }
    }

    fn format_timestamp(&self, timestamp: std::time::SystemTime) -> String {
        let datetime: DateTime<Local> = timestamp.into();
        datetime.format(&self.config.format).to_string()
    }

    fn notification_button_style(
        theme: &AshellTheme,
        is_last: bool,
    ) -> impl Fn(&Theme, iced::widget::button::Status) -> iced::widget::button::Style {
        let theme = theme.clone();
        move |iced_theme: &Theme, status| {
            let mut style = iced::widget::button::Style::default();
            match status {
                iced::widget::button::Status::Hovered => {
                    if is_last {
                        style.border = Border::default().rounded(
                            Radius::default()
                                .bottom_left(theme.radius.md)
                                .bottom_right(theme.radius.md),
                        );
                    }
                    style.background = Some(Background::Color(
                        iced_theme
                            .extended_palette()
                            .background
                            .weak
                            .color
                            .scale_alpha(theme.opacity),
                    ));
                }
                _ => {
                    style.background = Some(Background::Color(iced::Color::TRANSPARENT));
                }
            }
            style
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
        .style(Self::notification_button_style(theme, is_last))
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
            let is_collapsed = !self.collapsed_groups.contains(&app_name);
            let app_icon: Element<'a, Message> = notifications
                .first()
                .map(|n| {
                    if n.app_icon.is_empty() {
                        icon(StaticIcon::Bell).size(ICON_SIZE).into()
                    } else {
                        DynamicIcon(n.app_icon.to_string())
                            .to_text()
                            .size(ICON_SIZE)
                            .into()
                    }
                })
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
            if is_collapsed {
                if let Some(first_notification) = notifications.first() {
                    preview = preview.push(horizontal_rule(HORIZONTAL_RULE_HEIGHT));
                    preview = preview.push(self.build_full_item(first_notification, true, theme))
                }
            } else {
                for (i, notification) in notifications.iter().enumerate() {
                    let is_last = i == notifications.len() - 1;
                    preview = preview.push(column!(
                        horizontal_rule(HORIZONTAL_RULE_HEIGHT),
                        self.build_full_item(notification, is_last, theme)
                    ));
                }
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
            let notification_element = container(
                column!(
                    row!(
                        notification_icon(&notification.app_icon),
                        text(&notification.app_name)
                            .size(theme.font_size.md)
                            .style(palette_text_style),
                        Space::with_width(Length::Fill),
                        {
                            let timestamp_element: Element<'_, Message> =
                                if self.config.show_timestamps {
                                    text(self.format_timestamp(notification.timestamp))
                                        .size(theme.font_size.sm)
                                        .style(weak_text_style)
                                        .into()
                                } else {
                                    Space::with_width(Length::Shrink).into()
                                };
                            timestamp_element
                        }
                    )
                    .spacing(theme.space.xs)
                    .align_y(Alignment::Center),
                    text(&notification.summary)
                        .size(theme.font_size.sm)
                        .style(strong_text_style),
                    {
                        let body_element: Element<'_, Message> =
                            if self.config.show_bodies && !notification.body.is_empty() {
                                text(&notification.body)
                                    .size(theme.font_size.sm)
                                    .wrapping(text::Wrapping::WordOrGlyph)
                                    .style(strong_text_style)
                                    .into()
                            } else {
                                Space::with_height(Length::Shrink).into()
                            };
                        body_element
                    }
                )
                .spacing(theme.space.xxs),
            )
            .style(Self::item_container_style(theme))
            .padding(theme.space.sm)
            .width(Length::Fill);

            content = content.push(
                button(notification_element)
                    .on_press(Message::NotificationClicked(notification.id))
                    .style(Self::notification_button_style(theme, false))
                    .padding(0),
            );
        }
        content.into()
    }

    pub fn menu_view<'a>(&'a self, theme: &'a AshellTheme) -> Element<'a, Message> {
        let content = if self.notifications.is_empty() {
            container(text("No notifications").size(theme.font_size.md))
                .width(Length::Fill)
                .height(Length::Fixed(EMPTY_STATE_HEIGHT))
                .center_x(Length::Fill)
                .center_y(Length::Fixed(EMPTY_STATE_HEIGHT))
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
}
