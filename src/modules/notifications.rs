use crate::{
    components::icons::{DynamicIcon, Icon, StaticIcon, icon},
    config::{self, NotificationsModuleConfig},
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
    window::Id,
};
use log::error;
use std::collections::HashMap;

// Constants for UI dimensions and styling
const EMPTY_STATE_HEIGHT: f32 = 300.0;
const ICON_SIZE: f32 = 20.0;
const NOTIFICATION_ICON_SPACING: f32 = 10.0;
const HORIZONTAL_RULE_HEIGHT: f32 = 0.2;
const MAX_PREVIEW_ITEMS: usize = 3;

#[derive(Debug, Clone)]
pub enum Message {
    ConfigReloaded(config::NotificationsModuleConfig),
    NotificationClicked(u32),
    NotificationClosed(u32),
    ClearNotifications,
    NotificationsCleared,
    Event(ServiceEvent<NotificationsService>),
    Expand,
}

pub struct Notifications {
    config: NotificationsModuleConfig,
    notifications: HashMap<u32, Notification>,
    service: Option<NotificationsService>,
    collapse: bool,
}

impl Notifications {
    pub fn new(config: NotificationsModuleConfig) -> Self {
        Self {
            config,
            notifications: HashMap::new(),
            service: None,
            collapse: true,
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
                        self.notifications = service.notifications.clone();
                    }
                    Task::none()
                }
                ServiceEvent::Error(_) => Task::none(),
            },
            Message::NotificationClicked(id) => {
                // Get the notification action before async operation
                let action_key = self.notifications.get(&id)
                    .filter(|n| !n.actions.is_empty())
                    .and_then(|n| n.actions.first())
                    .cloned();

                // Perform async operations, then remove from state
                Task::perform(
                    async move {
                        // Invoke action if present
                        if let Some(action_key) = action_key {
                            if let Err(e) = NotificationDaemon::invoke_action(id, action_key).await {
                                error!("Failed to invoke notification action for id {}: {}", id, e);
                            }
                        }
                        // Close notification
                        if let Err(e) = NotificationDaemon::close_notification_by_id(id).await {
                            error!("Failed to close notification id {}: {}", id, e);
                        }
                        id
                    },
                    Message::NotificationClosed,
                )
            }
            Message::NotificationClosed(id) => {
                // Remove from local state after async operation completes
                self.notifications.remove(&id);
                Task::none()
            }
            Message::ClearNotifications => {
                // Get notification IDs for async operation
                let notification_ids: Vec<u32> = self.notifications.keys().copied().collect();

                // Close each notification through the daemon
                Task::perform(
                    async move {
                        for id in notification_ids {
                            if let Err(e) = NotificationDaemon::close_notification_by_id(id).await {
                                error!("Failed to close notification id {}: {}", id, e);
                            }
                        }
                    },
                    |_| Message::NotificationsCleared,
                )
            }
            Message::NotificationsCleared => {
                // Clear local state after async operations complete
                self.notifications.clear();
                Task::none()
            }
            Message::Expand => {
                self.collapse = !self.collapse;
                Task::none()
            }
        }
    }

    fn format_timestamp(&self, timestamp: std::time::SystemTime) -> String {
        let datetime: DateTime<Local> = timestamp.into();
        datetime.format(&self.config.format).to_string()
    }

    // Helper method for common notification item button styling
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

    // Helper method for group header button styling
    fn group_header_button_style(
        theme: &AshellTheme,
    ) -> impl Fn(&Theme, iced::widget::button::Status) -> iced::widget::button::Style {
        let theme = theme.clone();
        move |iced_theme: &Theme, status| {
            let mut style = iced::widget::button::Style::default();
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
                    style.border = Border::default().rounded(
                        Radius::default()
                            .top_left(theme.radius.md)
                            .top_right(theme.radius.md),
                    );
                }
                _ => {
                    style.background = Some(Background::Color(iced::Color::TRANSPARENT));
                    style.border = Border::default().rounded(
                        Radius::default()
                            .top_left(theme.radius.md)
                            .top_right(theme.radius.md),
                    );
                }
            }
            style
        }
    }

    fn build_preview_item<'a>(
        &'a self,
        notification: &'a Notification,
        is_last: bool,
        theme: &'a AshellTheme,
    ) -> Element<'a, Message> {
        button(row!(
            text(&notification.summary)
                .size(theme.font_size.sm)
                .style(|theme: &Theme| text::Style {
                    color: Some(theme.extended_palette().secondary.strong.text),
                },),
            Space::with_width(Length::Fixed(NOTIFICATION_ICON_SPACING)),
            text(&notification.body)
                .size(theme.font_size.sm)
                .wrapping(text::Wrapping::WordOrGlyph)
                .style(|theme: &Theme| text::Style {
                    color: Some(theme.extended_palette().secondary.weak.text),
                },)
        ))
        .width(Length::Fill)
        .style(Self::notification_button_style(theme, is_last))
        .on_press(Message::NotificationClicked(notification.id))
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
                        .size(theme.font_size.sm)
                        .style(|theme: &Theme| text::Style {
                            color: Some(theme.extended_palette().secondary.strong.text),
                        },),
                    Space::with_width(Length::Fill),
                    text(self.format_timestamp(notification.timestamp))
                        .size(theme.font_size.sm)
                        .style(|theme: &Theme| text::Style {
                            color: Some(theme.extended_palette().secondary.weak.text,),
                        })
                ),
                text(&notification.body)
                    .size(theme.font_size.sm)
                    .wrapping(text::Wrapping::WordOrGlyph)
                    .style(|theme: &Theme| text::Style {
                        color: Some(theme.extended_palette().secondary.weak.text),
                    },)
            )
            .spacing(theme.space.xs),
        )
        .width(Length::Fill)
        .style(Self::notification_button_style(theme, is_last))
        .on_press(Message::NotificationClicked(notification.id))
        .into()
    }

    pub fn view(&'_ self, _: &AshellTheme) -> Element<'_, Message> {
        let count = self.notifications.len();

        if count > 0 {
            icon(StaticIcon::BellBadge).into()
        } else {
            icon(StaticIcon::Bell).into()
        }
    }

    pub fn subscription(&self) -> Subscription<Message> {
        use crate::services::ReadOnlyService;
        NotificationsService::subscribe().map(Message::Event)
    }
    fn grouped_notifications<'a>(
        &'a self,
        _id: Id,
        theme: &'a AshellTheme,
    ) -> Element<'a, Message> {
        let mut grouped: HashMap<String, Vec<&Notification>> = HashMap::new();
        for notification in self.notifications.values() {
            grouped
                .entry(notification.app_name.clone())
                .or_default()
                .push(notification);
        }

        let mut content = column!().spacing(theme.space.sm);
        for (app_name, notifications) in grouped {
            let app_icon = notifications.first().and_then(|n| {
                if n.app_icon.is_empty() {
                    None
                } else {
                    Some(
                        DynamicIcon(n.app_icon.clone())
                            .to_text()
                            .size(ICON_SIZE)
                            .style(|theme: &Theme| text::Style {
                                color: Some(theme.palette().text),
                            }),
                    )
                }
            });
            let header = row!(
                button(
                    app_icon.unwrap_or_else(|| icon(StaticIcon::Bell).size(ICON_SIZE).style(
                        |theme: &Theme| text::Style {
                            color: Some(theme.palette().text),
                        }
                    ))
                )
                .on_press(Message::ClearNotifications),
                text(app_name)
                    .size(theme.font_size.md)
                    .style(|theme: &Theme| text::Style {
                        color: Some(theme.palette().text),
                    }),
                Space::with_width(Length::Fill),
                text(format!(
                    "{} new {}",
                    notifications.len(),
                    notifications
                        .first()
                        .map(|a| { self.format_timestamp(a.timestamp) })
                        .unwrap_or_default()
                ))
                .size(theme.font_size.sm)
                .style(|theme: &Theme| text::Style {
                    color: Some(theme.extended_palette().secondary.weak.text),
                }),
                icon(if self.collapse {
                    StaticIcon::RightChevron
                } else {
                    StaticIcon::UpChevron
                })
            )
            .spacing(theme.space.xs)
            .align_y(Alignment::Center);

            let mut preview = column!();
            if self.collapse {
                // Show preview of up to 3 notifications
                let preview_count = std::cmp::min(notifications.len(), MAX_PREVIEW_ITEMS);
                for (i, notification) in notifications.iter().take(MAX_PREVIEW_ITEMS).enumerate() {
                    let is_last = i == preview_count - 1;
                    preview = preview.push(self.build_preview_item(notification, is_last, theme));
                }
            } else {
                // Show all notifications with full details
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
                    .on_press(Message::Expand),
                preview
            );
            content = content.push(
                container(item)
                    .style(move |app_theme: &Theme| container::Style {
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
                    })
                    .width(Length::Fill),
            );
        }
        content.into()
    }
    fn list_notifications<'a>(&'a self, _id: Id, theme: &'a AshellTheme) -> Element<'a, Message> {
        let mut notifications_data: Vec<_> = self.notifications.values().cloned().collect();

        // Sort by timestamp (newest first) and limit
        notifications_data.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        if let Some(max) = self.config.max_notifications {
            notifications_data.truncate(max);
        }

        let mut content = column!().spacing(theme.space.sm);
        {
            for notification in notifications_data {
                let notification_element = container(
                    column!(
                        row!(
                            if notification.app_icon.is_empty() {
                                icon(StaticIcon::Bell)
                                    .size(ICON_SIZE)
                                    .style(|theme: &Theme| text::Style {
                                        color: Some(theme.palette().text),
                                    })
                            } else {
                                DynamicIcon(notification.app_icon.clone())
                                    .to_text()
                                    .size(ICON_SIZE)
                                    .style(|theme: &Theme| text::Style {
                                        color: Some(theme.palette().text),
                                    })
                            },
                            text(notification.app_name).size(theme.font_size.md).style(
                                |theme: &Theme| text::Style {
                                    color: Some(theme.palette().text),
                                }
                            ),
                            Space::with_width(Length::Fill),
                            {
                                let timestamp_element: Element<'_, Message> =
                                    if self.config.show_timestamps {
                                        text(self.format_timestamp(notification.timestamp))
                                            .size(theme.font_size.sm)
                                            .style(|theme: &Theme| text::Style {
                                                color: Some(
                                                    theme.extended_palette().secondary.weak.text,
                                                ),
                                            })
                                            .into()
                                    } else {
                                        Space::with_width(Length::Shrink).into()
                                    };
                                timestamp_element
                            }
                        )
                        .spacing(theme.space.xs)
                        .align_y(Alignment::Center),
                        text(notification.summary).size(theme.font_size.sm).style(
                            |theme: &Theme| text::Style {
                                color: Some(theme.extended_palette().secondary.strong.text),
                            }
                        ),
                        {
                            let body_element: Element<'_, Message> = if self.config.show_bodies
                                && !notification.body.is_empty()
                            {
                                text(notification.body)
                                    .size(theme.font_size.sm)
                                    .wrapping(text::Wrapping::WordOrGlyph)
                                    .style(|theme: &Theme| text::Style {
                                        color: Some(theme.extended_palette().secondary.strong.text),
                                    })
                                    .into()
                            } else {
                                Space::with_height(Length::Shrink).into()
                            };
                            body_element
                        }
                    )
                    .spacing(theme.space.xxs),
                )
                .style(move |app_theme: &Theme| container::Style {
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
                })
                .padding(theme.space.sm)
                .width(Length::Fill);

                content = content.push(
                    button(notification_element)
                        .on_press(Message::NotificationClicked(notification.id))
                        .style(move |iced_theme: &Theme, status| {
                            let mut style = iced::widget::button::Style::default();
                            match status {
                                iced::widget::button::Status::Hovered => {
                                    style.background = Some(Background::Color(
                                        iced_theme
                                            .extended_palette()
                                            .secondary
                                            .strong
                                            .color
                                            .scale_alpha(0.2),
                                    ));
                                    style.border = Border::default().rounded(theme.radius.md);
                                }
                                _ => {
                                    style.background =
                                        Some(Background::Color(iced::Color::TRANSPARENT));
                                    style.border = Border::default().rounded(theme.radius.md);
                                }
                            }
                            style
                        })
                        .padding(0),
                );
            }
        }
        content.into()
    }
    pub fn menu_view<'a>(&'a self, _id: Id, theme: &'a AshellTheme) -> Element<'a, Message> {
        let content = if self.notifications.is_empty() {
            container(text("No notifications").size(theme.font_size.md))
                .width(Length::Fill)
                .height(Length::Fixed(EMPTY_STATE_HEIGHT))
                .center_x(Length::Fill)
                .center_y(Length::Fixed(EMPTY_STATE_HEIGHT))
                .into()
        } else if self.config.grouped {
            self.grouped_notifications(_id, theme)
        } else {
            self.list_notifications(_id, theme)
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
