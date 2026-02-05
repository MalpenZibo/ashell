use crate::{
    components::icons::{DynamicIcon, Icon, StaticIcon, icon}, config::{self, NotificationsModuleConfig}, menu::MenuSize, services::{
        ReadOnlyService, ServiceEvent,
        notifications::{Notification, NotificationsService, dbus::NotificationDaemon},
    }, theme::AshellTheme
};
use chrono::{DateTime, Local};
use iced::{
    Alignment, Background, Border, Color, Element, Length, Subscription, Theme,
    widget::{button, column, container, row, scrollable, text},
    window::Id,
};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub enum Message {
    ConfigReloaded(config::NotificationsModuleConfig),
    NotificationClicked(u32),
    ClearNotifications,
    Event(ServiceEvent<NotificationsService>),
}

pub struct Notifications {
    config: NotificationsModuleConfig,
    notifications: HashMap<u32, Notification>,
    service: Option<NotificationsService>,
}

impl Notifications {
    pub fn new(config: NotificationsModuleConfig) -> Self {
        Self {
            config,
            notifications: HashMap::new(),
            service: None,
        }
    }

    pub fn update(&mut self, message: Message) {
        match message {
            Message::ConfigReloaded(notifications_module_config) => {
                self.config = notifications_module_config;
            }
            Message::Event(event) => match event {
                ServiceEvent::Init(service) => {
                    self.service = Some(service);
                }
                ServiceEvent::Update(update_event) => {
                    if let Some(service) = self.service.as_mut() {
                        service.update(update_event);
                        self.notifications = service.notifications.clone();
                    }
                }
                ServiceEvent::Error(_) => {}
            },
            Message::NotificationClicked(id) => {
                // Get the notification to check for actions
                if let Some(notification) = self.notifications.get(&id)
                    && !notification.actions.is_empty() {
                        // Invoke the default action (first action)
                        let action_key = notification.actions[0].clone();
                        tokio::spawn(async move {
                            NotificationDaemon::invoke_action(id, action_key).await.ok();
                        });
                    }
                // Remove the notification from local state
                self.notifications.remove(&id);
            }
            Message::ClearNotifications => {
                self.notifications.clear();
            }
        }
    }

    fn format_timestamp(&self, timestamp: std::time::SystemTime) -> String {
        let datetime: DateTime<Local> = timestamp.into();
        datetime.format(&self.config.format).to_string()
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

    pub fn menu_view<'a>(&'a self, _id: Id, theme: &'a AshellTheme) -> Element<'a, Message> {
        let mut notifications_data: Vec<_> = self.notifications.values().cloned().collect();

        // Sort by timestamp (newest first) and limit
        notifications_data.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        if let Some(max) = self.config.max_notifications {
            notifications_data.truncate(max);
        }

        let mut content = column!().spacing(theme.space.sm);
        if notifications_data.is_empty() {
            content = content.push(
                container(text("No notifications").size(theme.font_size.md))
                    .width(Length::Fill)
                    .height(Length::Fixed(300.0))
                    .center_x(Length::Fill)
                    .center_y(Length::Fixed(300.0)),
            );
        } else {
            for notification in notifications_data {
                let notification_element = container(
                    column!(
                        row!(
                            if notification.app_icon.is_empty() {
                                icon(StaticIcon::Bell).size(20.0).style(|theme: &Theme| {
                                    text::Style {
                                        color: Some(theme.palette().text),
                                    }
                                })
                            } else {
                                DynamicIcon(notification.app_icon.clone())
                                    .to_text()
                                    .size(20.0)
                                    .style(|theme: &Theme| text::Style {
                                        color: Some(theme.palette().text),
                                    })
                            },
                            text(notification.app_name).size(theme.font_size.md).style(
                                |theme: &Theme| text::Style {
                                    color: Some(theme.palette().text),
                                }
                            ),
                            if self.config.show_timestamps {
                                text(self.format_timestamp(notification.timestamp))
                                    .size(theme.font_size.sm)
                                    .style(|theme: &Theme| text::Style {
                                        color: Some(theme.extended_palette().secondary.weak.text),
                                    })
                            } else {
                                text("")
                            }
                        )
                        .spacing(theme.space.xs)
                        .align_y(Alignment::Center),
                        text(notification.summary).size(theme.font_size.sm).style(
                            |theme: &Theme| text::Style {
                                color: Some(theme.extended_palette().secondary.strong.text),
                            }
                        ),
                        if self.config.show_bodies && !notification.body.is_empty() {
                            text(notification.body)
                                .size(theme.font_size.sm)
                                .wrapping(text::Wrapping::WordOrGlyph)
                                .style(|theme: &Theme| text::Style {
                                    color: Some(theme.extended_palette().secondary.strong.text),
                                })
                        } else {
                            text("")
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
                                    style.border = Border::default().rounded(8.0);
                                }
                                _ => {
                                    style.background =
                                        Some(Background::Color(iced::Color::TRANSPARENT));
                                    style.border = Border::default().rounded(8.0);
                                }
                            }
                            style
                        })
                        .padding(0),
                );
            }
        }
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
                    container(text(""))
                        .width(Length::Fill)
                        .align_x(Alignment::End)
                }
            ),
            scrollable(content).scrollbar_width(0.0).scroller_width(0.0),
        ).width(MenuSize::Medium)
        .spacing(theme.space.sm)
        .into()
    }
}
