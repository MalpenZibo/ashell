use super::{SubMenu, quick_setting_button};
use crate::{
    components::icons::{StaticIcon, icon},
    services::{
        ReadOnlyService, Service, ServiceEvent,
        bluetooth::{BluetoothCommand, BluetoothService, BluetoothState},
    },
    theme::AshellTheme,
};
use iced::{
    Element, Length, Subscription, Task, Theme,
    alignment::Vertical,
    widget::{Column, button, column, container, horizontal_rule, row, scrollable, text},
    window::Id,
};
use zbus::zvariant::OwnedObjectPath;

#[derive(Debug, Clone)]
pub enum Message {
    Event(ServiceEvent<BluetoothService>),
    Toggle,
    ToggleSubMenu,
    StartDiscovery,
    StopDiscovery,
    PairDevice(OwnedObjectPath),
    ConnectDevice(OwnedObjectPath),
    DisconnectDevice(OwnedObjectPath),
    RemoveDevice(OwnedObjectPath),
    More(Id),
    ConfigReloaded(BluetoothSettingsConfig),
}

pub enum Action {
    None,
    ToggleBluetoothMenu,
    CloseMenu(Id),
    CloseSubMenu(Task<Message>),
    Command(Task<Message>),
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
            Message::StartDiscovery => match self.service.as_mut() {
                Some(service) => Action::Command(
                    service
                        .command(BluetoothCommand::StartDiscovery)
                        .map(Message::Event),
                ),
                _ => Action::None,
            },
            Message::StopDiscovery => match self.service.as_mut() {
                Some(service) => Action::Command(
                    service
                        .command(BluetoothCommand::StopDiscovery)
                        .map(Message::Event),
                ),
                _ => Action::None,
            },
            Message::PairDevice(device_path) => match self.service.as_mut() {
                Some(service) => Action::Command(
                    service
                        .command(BluetoothCommand::PairDevice(device_path))
                        .map(Message::Event),
                ),
                _ => Action::None,
            },
            Message::ConnectDevice(device_path) => match self.service.as_mut() {
                Some(service) => Action::Command(
                    service
                        .command(BluetoothCommand::ConnectDevice(device_path))
                        .map(Message::Event),
                ),
                _ => Action::None,
            },
            Message::DisconnectDevice(device_path) => match self.service.as_mut() {
                Some(service) => Action::Command(
                    service
                        .command(BluetoothCommand::DisconnectDevice(device_path))
                        .map(Message::Event),
                ),
                _ => Action::None,
            },
            Message::RemoveDevice(device_path) => match self.service.as_mut() {
                Some(service) => Action::Command(
                    service
                        .command(BluetoothCommand::RemoveDevice(device_path))
                        .map(Message::Event),
                ),
                _ => Action::None,
            },
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
            // Get connected devices names
            let connected_devices: Vec<_> = service
                .devices
                .iter()
                .filter(|d| d.connected)
                .map(|d| d.name.clone())
                .collect();

            let device_name = match connected_devices.len() {
                0 => None,
                1 => Some(connected_devices[0].clone()),
                n => Some(format!("{} devices", n)),
            };

            Some((
                quick_setting_button(
                    theme,
                    StaticIcon::Bluetooth,
                    "Bluetooth".to_owned(),
                    device_name,
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
            // Separate devices by their status
            let connected_devices: Vec<_> =
                service.devices.iter().filter(|d| d.connected).collect();

            let paired_devices: Vec<_> = service
                .devices
                .iter()
                .filter(|d| d.paired && !d.connected)
                .collect();

            let available_devices: Vec<_> = service
                .devices
                .iter()
                .filter(|d| !d.paired && !d.connected)
                .collect();

            let mut main_column = column![
                row![
                    text("Bluetooth Devices").width(Length::Fill),
                    text(if service.discovering {
                        "Scanning..."
                    } else {
                        ""
                    })
                    .size(theme.font_size.xs),
                    button(icon(if service.discovering {
                        StaticIcon::Close
                    } else {
                        StaticIcon::Refresh
                    }))
                    .padding([theme.space.xxs, theme.space.sm])
                    .style(theme.settings_button_style())
                    .on_press(if service.discovering {
                        Message::StopDiscovery
                    } else {
                        Message::StartDiscovery
                    }),
                ]
                .align_y(Vertical::Center)
                .spacing(theme.space.xs)
                .width(Length::Fill),
                horizontal_rule(1),
            ]
            .spacing(theme.space.xs);

            // Connected devices section
            if !connected_devices.is_empty() {
                main_column = main_column.push(text("Connected").size(theme.font_size.xs));

                let connected_list = Column::with_children(
                    connected_devices
                        .iter()
                        .map(|d| {
                            let mut device_row = row![text(d.name.clone()).width(Length::Fill)]
                                .spacing(theme.space.xs);

                            if let Some(battery) = d.battery {
                                device_row = device_row.push(Self::battery_level(theme, battery));
                            }

                            button(container(device_row).style(move |theme: &Theme| {
                                container::Style {
                                    text_color: Some(theme.palette().success),
                                    ..Default::default()
                                }
                            }))
                            .style(theme.ghost_button_style())
                            .padding([theme.space.xs, theme.space.xs])
                            .on_press(Message::DisconnectDevice(d.path.clone()))
                            .width(Length::Fill)
                            .into()
                        })
                        .collect::<Vec<Element<'a, Message>>>(),
                )
                .spacing(theme.space.xxs);

                main_column = main_column
                    .push(container(scrollable(connected_list)).max_height(150))
                    .push(horizontal_rule(1));
            }

            // Paired devices section
            if !paired_devices.is_empty() {
                main_column = main_column.push(text("Paired").size(theme.font_size.xs));

                let paired_list = Column::with_children(
                    paired_devices
                        .iter()
                        .map(|d| {
                            let avail = available_devices.iter().any(|dev| dev.path == d.path);

                            let mut device_row = row![text(d.name.clone()).width(Length::Fill)]
                                .spacing(theme.space.xs)
                                .align_y(Vertical::Center)
                                .padding([theme.space.xs, theme.space.xs]);

                            if avail {
                                device_row = device_row.push(
                                    button(text("Connect").size(theme.font_size.xs))
                                        .style(theme.settings_button_style())
                                        .padding([theme.space.xxs, theme.space.sm])
                                        .on_press(Message::ConnectDevice(d.path.clone())),
                                );
                            }

                            device_row = device_row.push(
                                button(text("Remove").size(theme.font_size.xs))
                                    .style(theme.settings_button_style())
                                    .padding([theme.space.xxs, theme.space.sm])
                                    .on_press(Message::RemoveDevice(d.path.clone())),
                            );

                            device_row.into()
                        })
                        .collect::<Vec<Element<'a, Message>>>(),
                )
                .spacing(theme.space.xxs);

                main_column = main_column
                    .push(container(scrollable(paired_list)).max_height(150))
                    .push(horizontal_rule(1));
            }

            // Available devices section
            if !available_devices.is_empty() {
                main_column = main_column.push(text("Available").size(theme.font_size.xs));

                let available_list = Column::with_children(
                    available_devices
                        .iter()
                        .map(|d| {
                            button(
                                row![
                                    text(d.name.clone()).width(Length::Fill),
                                    text("Pair").size(theme.font_size.xs),
                                ]
                                .align_y(Vertical::Center)
                                .spacing(theme.space.xs),
                            )
                            .style(theme.ghost_button_style())
                            .padding([theme.space.xs, theme.space.xs])
                            .on_press(Message::PairDevice(d.path.clone()))
                            .width(Length::Fill)
                            .into()
                        })
                        .collect::<Vec<Element<'a, Message>>>(),
                )
                .spacing(theme.space.xxs);

                main_column = main_column
                    .push(container(scrollable(available_list)).max_height(150))
                    .push(horizontal_rule(1));
            }

            // No devices message
            if connected_devices.is_empty()
                && paired_devices.is_empty()
                && available_devices.is_empty()
            {
                main_column = main_column.push(text("No devices found"));
            }

            if self.config.more_cmd.is_some() {
                Some(
                    main_column
                        .push(
                            button("More")
                                .on_press(Message::More(id))
                                .padding([theme.space.xxs, theme.space.sm])
                                .width(Length::Fill)
                                .style(theme.ghost_button_style()),
                        )
                        .into(),
                )
            } else {
                Some(main_column.into())
            }
        } else {
            None
        }
    }

    fn battery_level<'a>(theme: &AshellTheme, battery: u8) -> Element<'a, Message> {
        container(
            row!(
                icon(match battery {
                    0..=20 => StaticIcon::Battery0,
                    21..=40 => StaticIcon::Battery1,
                    41..=60 => StaticIcon::Battery2,
                    61..=80 => StaticIcon::Battery3,
                    _ => StaticIcon::Battery4,
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

    pub fn bluetooth_indicator<'a>(
        &'a self,
        _theme: &'a AshellTheme,
    ) -> Option<Element<'a, Message>> {
        if let Some(service) = &self.service {
            if service.state == BluetoothState::Active {
                let connected_count = service.devices.iter().filter(|d| d.connected).count();
                if connected_count > 0 {
                    return Some(icon(StaticIcon::BluetoothConnected).into());
                } else {
                    return Some(icon(StaticIcon::Bluetooth).into());
                }
            }
        }
        None
    }

    pub fn subscription(&self) -> Subscription<Message> {
        BluetoothService::subscribe().map(Message::Event)
    }
}
