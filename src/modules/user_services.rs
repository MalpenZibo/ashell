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
    widget::{Column, column, container, row, rule, scrollable, text, toggler},
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

        row![icon(StaticIcon::Server), text(format!("{active}/{total}")),]
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
                            .map(|unit| {
                                let name = unit.display_name();
                                let unit_name = unit.name.clone();
                                let is_active = unit.is_active();
                                let can_toggle = unit.can_toggle();

                                row!(
                                    text(name).width(Length::Fill),
                                    toggler(is_active)
                                        .on_toggle_maybe(can_toggle.then_some(move |_| {
                                            Message::ToggleUnit(unit_name.clone())
                                        },))
                                        .width(Length::Shrink),
                                )
                                .padding([theme.space.xxs, theme.space.xs])
                                .into()
                            })
                            .collect::<Vec<_>>(),
                    )
                    .spacing(theme.space.xs)
                    .padding(
                        Padding::default()
                            .right(theme.space.md)
                            .left(theme.space.xs),
                    ),
                ))
                .max_height(400)
                .into(),
            },
        ]
        .width(MenuSize::Medium)
        .spacing(theme.space.xs)
        .into()
    }

    pub fn subscription(&self) -> Subscription<Message> {
        UserServicesService::subscribe().map(Message::Event)
    }
}
