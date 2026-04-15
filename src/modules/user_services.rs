use crate::{
    components::icons::{StaticIcon, icon, icon_button},
    menu::MenuSize,
    services::{
        ReadOnlyService, Service, ServiceEvent,
        user_services::{UserServicesCommand, UserServicesService},
    },
    theme::AshellTheme,
};
use iced::{
    Alignment, Element, Length, Padding, Subscription, Task,
    alignment::Vertical,
    widget::{Column, button, column, container, row, rule, scrollable, text},
};
use std::convert;

#[derive(Debug, Clone)]
pub enum Message {
    ToggleUnit(String),
    Refresh,
    Event(ServiceEvent<UserServicesService>),
}

pub enum Action {
    None,
    Command(Task<Message>),
}

pub struct UserServices {
    service: Option<UserServicesService>,
}

impl UserServices {
    pub fn new() -> Self {
        Self { service: None }
    }

    pub fn update(&mut self, message: Message) -> Action {
        match message {
            Message::ToggleUnit(name) => match self.service.as_mut() {
                Some(service) => Action::Command(
                    service
                        .command(UserServicesCommand::ToggleUnit(name))
                        .map(Message::Event),
                ),
                None => Action::None,
            },
            Message::Refresh => match self.service.as_mut() {
                Some(service) => Action::Command(
                    service
                        .command(UserServicesCommand::Refresh)
                        .map(Message::Event),
                ),
                None => Action::None,
            },
            Message::Event(event) => match event {
                ServiceEvent::Init(service) => {
                    self.service = Some(service);
                    Action::None
                }
                ServiceEvent::Update(update) => {
                    if let Some(service) = self.service.as_mut() {
                        service.update(update);
                    }
                    Action::None
                }
                ServiceEvent::Error(()) => Action::None,
            },
        }
    }

    pub fn view(&self, theme: &AshellTheme) -> Element<'_, Message> {
        let (active, total) = match &self.service {
            Some(service) => (service.active_count(), service.units.len()),
            None => (0, 0),
        };

        row![
            icon(StaticIcon::Server),
            text(format!("{active}/{total}")).size(theme.font_size.sm),
        ]
        .align_y(Alignment::Center)
        .spacing(theme.space.xxs)
        .into()
    }

    pub fn menu_view<'a>(&'a self, theme: &'a AshellTheme) -> Element<'a, Message> {
        column![
            row![
                text("User Services")
                    .size(theme.font_size.lg)
                    .width(Length::Fill),
                icon_button::<Message>(theme, StaticIcon::Refresh).on_press(Message::Refresh),
            ]
            .align_y(Vertical::Center),
            rule::horizontal(1),
            match &self.service {
                None => convert::Into::<Element<'_, _>>::into(
                    container(text("Connecting...")).padding(theme.space.xs),
                ),
                Some(service) if service.units.is_empty() => {
                    convert::Into::<Element<'_, _>>::into(
                        container(text("No user services found")).padding(theme.space.xs),
                    )
                }
                Some(service) => container(scrollable(
                    Column::with_children(
                        service
                            .units
                            .iter()
                            .map(|unit| self.unit_row(theme, unit))
                            .collect::<Vec<_>>(),
                    )
                    .spacing(theme.space.xxs)
                    .padding(Padding::default().right(theme.space.md)),
                ))
                .max_height(400)
                .into(),
            },
        ]
        .width(MenuSize::Medium)
        .spacing(theme.space.xs)
        .into()
    }

    fn unit_row<'a>(
        &'a self,
        theme: &'a AshellTheme,
        unit: &'a crate::services::user_services::UnitInfo,
    ) -> Element<'a, Message> {
        let status_dot = icon(StaticIcon::Point)
            .size(theme.font_size.xs)
            .color(unit.status_color());

        let name = text(unit.display_name()).size(theme.font_size.sm);

        let content = row![status_dot, name]
            .align_y(Vertical::Center)
            .spacing(theme.space.xs)
            .width(Length::Fill);

        if unit.can_toggle() {
            button(content)
                .style(theme.ghost_button_style())
                .padding(theme.space.xs)
                .on_press(Message::ToggleUnit(unit.name.clone()))
                .width(Length::Fill)
                .into()
        } else {
            container(content)
                .padding(theme.space.xs)
                .width(Length::Fill)
                .into()
        }
    }

    pub fn subscription(&self) -> Subscription<Message> {
        UserServicesService::subscribe().map(Message::Event)
    }
}
