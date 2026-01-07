use super::{SubMenu, quick_setting_button};
use crate::{
    components::icons::{IconButtonSize, StaticIcon, icon, icon_button},
    config::SettingsFormat,
    services::{
        ReadOnlyService, Service, ServiceEvent,
        bluetooth::{BluetoothCommand, BluetoothDevice, BluetoothService, BluetoothState},
    },
    theme::AshellTheme,
};
use iced::{
    Element, Length, Subscription, Task, Theme,
    alignment::{Alignment, Horizontal, Vertical},
    widget::{Column, Row, button, column, container, horizontal_rule, row, scrollable, text},
    window::Id,
};
use itertools::Itertools;
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
    pub indicator_format: SettingsFormat,
}

impl BluetoothSettingsConfig {
    pub fn new(more_cmd: Option<String>, indicator_format: SettingsFormat) -> Self {
        Self {
            more_cmd,
            indicator_format,
        }
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
        self.service.as_ref().map(|service| {
            let connected_devices = service
                .devices
                .iter()
                .filter(|d| d.connected)
                .sorted_by_key(|d| &d.name);
            let paired_devices = service
                .devices
                .iter()
                .filter(|d| d.paired && !d.connected)
                .sorted_by_key(|d| &d.name);

            let mut known_devices = connected_devices.chain(paired_devices).peekable();
            let mut available_devices = service
                .devices
                .iter()
                .filter(|d| !d.paired && !d.connected)
                .peekable();

            let some_known = known_devices.peek().is_some();
            let some_available = available_devices.peek().is_some();

            Column::new()
                .push(
                    row![
                        text("Bluetooth Devices").width(Length::Fill),
                        text(if service.discovering {
                            "Scanning..."
                        } else {
                            ""
                        })
                        .size(theme.font_size.xs),
                        icon_button(
                            theme,
                            if service.discovering {
                                StaticIcon::Close
                            } else {
                                StaticIcon::Refresh
                            }
                        )
                        .on_press(if service.discovering {
                            Message::StopDiscovery
                        } else {
                            Message::StartDiscovery
                        })
                    ]
                    .align_y(Vertical::Center)
                    .spacing(theme.space.xs)
                    .width(Length::Fill),
                )
                .push_maybe(if some_known {
                    let known_device_entry = |d: &BluetoothDevice| {
                        button(
                            Row::new()
                                .push(
                                    text(d.name.clone())
                                        .color_maybe(if d.connected {
                                            Some(theme.get_theme().palette().success)
                                        } else {
                                            None
                                        })
                                        .width(Length::Fill),
                                )
                                .push_maybe(
                                    d.battery.map(|battery| Self::battery_level(theme, battery)),
                                )
                                .push(
                                    icon_button(theme, StaticIcon::Remove)
                                        .on_press(Message::RemoveDevice(d.path.clone()))
                                        .color(theme.get_theme().palette().danger)
                                        .size(IconButtonSize::Small),
                                )
                                .align_y(Vertical::Center)
                                .spacing(theme.space.xs)
                                .width(Length::Fill),
                        )
                        .style(theme.ghost_button_style())
                        .padding([theme.space.xs, theme.space.xs])
                        .on_press(if d.connected {
                            Message::DisconnectDevice(d.path.clone())
                        } else {
                            Message::ConnectDevice(d.path.clone())
                        })
                        .into()
                    };

                    Some(
                        column!(
                            column!(
                                container(
                                    text("Known devices")
                                        .size(theme.font_size.xs)
                                        .width(Length::Fill)
                                        .align_x(Horizontal::Right)
                                )
                                .padding([0, theme.space.sm]),
                                horizontal_rule(1),
                            ),
                            container(scrollable(
                                Column::with_children(known_devices.map(known_device_entry),)
                                    .padding([0, theme.space.xs, 0, 0])
                            ))
                            .max_height(150),
                        )
                        .spacing(theme.space.xs),
                    )
                } else {
                    None
                })
                .push_maybe(if some_available {
                    Some(
                        column!(
                            column!(
                                container(
                                    text("Available")
                                        .width(Length::Fill)
                                        .align_x(Horizontal::Right)
                                        .size(theme.font_size.xs),
                                )
                                .padding([0, theme.space.sm]),
                                horizontal_rule(1),
                            ),
                            container(scrollable(
                                Column::with_children(available_devices.map(|d| {
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
                                }))
                                .padding([
                                    0,
                                    theme.space.xs,
                                    0,
                                    0
                                ])
                            ))
                            .max_height(150),
                        )
                        .spacing(theme.space.xs),
                    )
                } else {
                    None
                })
                .push_maybe(if !some_known && !some_available {
                    Some(text("No devices found"))
                } else {
                    None
                })
                .push_maybe(self.config.more_cmd.as_ref().map(|_| horizontal_rule(1)))
                .push_maybe(self.config.more_cmd.as_ref().map(|_| {
                    button("More")
                        .on_press(Message::More(id))
                        .padding([theme.space.xxs, theme.space.sm])
                        .width(Length::Fill)
                        .style(theme.ghost_button_style())
                }))
                .spacing(theme.space.sm)
                .into()
        })
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
        if let Some(service) = &self.service
            && service.state == BluetoothState::Active
        {
            let connected_count = service.devices.iter().filter(|d| d.connected).count();
            match self.config.indicator_format {
                SettingsFormat::Icon => {
                    if connected_count > 0 {
                        Some(icon(StaticIcon::BluetoothConnected).into())
                    } else {
                        Some(icon(StaticIcon::Bluetooth).into())
                    }
                }
                SettingsFormat::Percentage => {
                    if connected_count > 0 {
                        Some(text(format!("{}", connected_count)).into())
                    } else {
                        Some(icon(StaticIcon::Bluetooth).into())
                    }
                }
                SettingsFormat::IconAndPercentage => {
                    if connected_count > 0 {
                        Some(
                            row!(
                                icon(StaticIcon::BluetoothConnected),
                                text(format!("{}", connected_count))
                            )
                            .spacing(4)
                            .align_y(Alignment::Center)
                            .into(),
                        )
                    } else {
                        Some(icon(StaticIcon::Bluetooth).into())
                    }
                }
            }
        } else {
            None
        }
    }

    pub fn subscription(&self) -> Subscription<Message> {
        BluetoothService::subscribe().map(Message::Event)
    }
}
