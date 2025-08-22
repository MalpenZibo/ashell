use super::{SubMenu, quick_setting_button};
use crate::{
    components::icons::{Icons, icon},
    services::{
        ReadOnlyService, Service, ServiceEvent,
        bluetooth::{BluetoothCommand, BluetoothService, BluetoothState},
    },
    theme::AshellTheme,
};
use iced::{
    Element, Length, Subscription, Task, Theme,
    widget::{Column, Row, button, column, container, horizontal_rule, row, text},
    window::Id,
};

#[derive(Debug, Clone)]
pub enum Message {
    Event(ServiceEvent<BluetoothService>),
    Toggle,
    ToggleSubMenu,
    More(Id),
    ConfigReloaded(BluetoothSettingsConfig),
}

pub enum Action {
    None,
    ToggleBluetoothMenu,
    CloseMenu(Id),
    CloseSubMenu(Task<Message>),
}

#[derive(Debug, Clone)]
pub struct BluetoothSettingsConfig {
    pub more_cmd: Option<String>,
}

impl BluetoothSettingsConfig {
    pub fn new(more_cmd: Option<String>) -> Self {
        Self { more_cmd }
    }
}

pub struct BluetoothSettings {
    config: BluetoothSettingsConfig,
    service: Option<BluetoothService>,
}

impl BluetoothSettings {
    pub fn new(config: BluetoothSettingsConfig) -> Self {
        Self {
            config,
            service: None,
        }
    }

    pub fn update(&mut self, message: Message) -> Action {
        match message {
            Message::Event(event) => match event {
                ServiceEvent::Init(service) => {
                    self.service = Some(service);
                    Action::None
                }
                ServiceEvent::Update(data) => {
                    if let Some(service) = self.service.as_mut() {
                        service.update(data);
                    }
                    Action::None
                }
                _ => Action::None,
            },
            Message::Toggle => match self.service.as_mut() {
                Some(service) => Action::CloseSubMenu(
                    service
                        .command(BluetoothCommand::Toggle)
                        .map(Message::Event),
                ),
                _ => Action::None,
            },
            Message::ToggleSubMenu => Action::ToggleBluetoothMenu,
            Message::More(id) => {
                if let Some(cmd) = &self.config.more_cmd {
                    crate::utils::launcher::execute_command(cmd.to_string());

                    Action::CloseMenu(id)
                } else {
                    Action::None
                }
            }
            Message::ConfigReloaded(config) => {
                self.config = config;
                Action::None
            }
        }
    }

    pub fn quick_setting_button<'a>(
        &'a self,
        id: Id,
        theme: &'a AshellTheme,
        sub_menu: Option<SubMenu>,
    ) -> Option<(Element<'a, Message>, Option<Element<'a, Message>>)> {
        if let Some(service) = &self.service
            && service.state != BluetoothState::Unavailable
        {
            Some((
                quick_setting_button(
                    theme,
                    Icons::Bluetooth,
                    "Bluetooth".to_owned(),
                    None,
                    service.state == BluetoothState::Active,
                    Message::Toggle,
                    Some((SubMenu::Bluetooth, sub_menu, Message::ToggleSubMenu))
                        .filter(|_| service.state == BluetoothState::Active),
                ),
                sub_menu
                    .filter(|menu_type| *menu_type == SubMenu::Bluetooth)
                    .and_then(|_| self.bluetooth_menu(id, theme)),
            ))
        } else {
            None
        }
    }

    fn bluetooth_menu<'a>(
        &'a self,
        id: Id,
        theme: &'a AshellTheme,
    ) -> Option<Element<'a, Message>> {
        if let Some(service) = &self.service {
            let main = if service.devices.is_empty() {
                text("No devices connected").into()
            } else {
                Column::with_children(
                    service
                        .devices
                        .iter()
                        .map(|d| {
                            Row::new()
                                .push(text(d.name.to_string()).width(Length::Fill))
                                .push_maybe(d.battery.map(|v| Self::battery_level(theme, v)))
                                .into()
                        })
                        .collect::<Vec<Element<Message>>>(),
                )
                .spacing(theme.space.xs)
                .into()
            };

            if self.config.more_cmd.is_some() {
                Some(
                    column!(
                        main,
                        horizontal_rule(1),
                        button("More")
                            .on_press(Message::More(id))
                            .padding([theme.space.xxs, theme.space.sm])
                            .width(Length::Fill)
                            .style(theme.ghost_button_style())
                    )
                    .spacing(theme.space.sm)
                    .into(),
                )
            } else {
                Some(main)
            }
        } else {
            None
        }
    }

    fn battery_level<'a>(theme: &AshellTheme, battery: u8) -> Element<'a, Message> {
        container(
            row!(
                icon(match battery {
                    0..=20 => Icons::Battery0,
                    21..=40 => Icons::Battery1,
                    41..=60 => Icons::Battery2,
                    61..=80 => Icons::Battery3,
                    _ => Icons::Battery4,
                }),
                text(format!("{battery}%"))
            )
            .spacing(theme.space.xs)
            .width(Length::Shrink),
        )
        .style(move |theme: &Theme| container::Style {
            text_color: Some(if battery <= 20 {
                theme.palette().danger
            } else {
                theme.palette().text
            }),
            ..container::Style::default()
        })
        .into()
    }

    pub fn subscription(&self) -> Subscription<Message> {
        BluetoothService::subscribe().map(Message::Event)
    }
}
