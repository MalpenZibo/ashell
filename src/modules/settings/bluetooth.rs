use super::{quick_setting_button, sub_menu_wrapper, Message, SubMenu};
use crate::{
    components::icons::{icon, Icons},
    config::SettingsModuleConfig,
    menu::Menu,
    style::{GhostButtonStyle, RED, TEXT},
    utils::{bluetooth::BluetoothCommand, Commander},
};
use iced::{
    theme::Button,
    widget::{button, column, horizontal_rule, row, text, Column, Row},
    Element, Length,
};
use log::debug;

#[derive(PartialEq, Eq, Debug, Clone)]
pub enum BluetoothState {
    Unavailable,
    Active,
    Inactive,
}

#[derive(Debug, Clone)]
pub struct Device {
    pub name: String,
    pub battery: Option<u8>,
}

#[derive(Debug, Clone)]
pub enum BluetoothMessage {
    Status(BluetoothState),
    DeviceList(Vec<Device>),
    Toggle,
    More,
}

pub struct Bluetooth {
    commander: Commander<BluetoothCommand>,
    state: BluetoothState,
    devices: Vec<Device>,
}

impl Bluetooth {
    pub fn new() -> Self {
        Self {
            commander: Commander::new(),
            state: BluetoothState::Unavailable,
            devices: Vec::new(),
        }
    }

    pub fn update(
        &mut self,
        msg: BluetoothMessage,
        menu: &mut Menu,
        sub_menu: &mut Option<SubMenu>,
        config: &SettingsModuleConfig,
    ) -> iced::Command<Message> {
        match msg {
            BluetoothMessage::Status(state) => {
                debug!("Bluetooth state: {:?}", state);
                self.state = state;

                if self.state != BluetoothState::Active && *sub_menu == Some(SubMenu::Bluetooth) {
                    *sub_menu = None;
                }

                iced::Command::none()
            }
            BluetoothMessage::DeviceList(devices) => {
                self.devices = devices;

                iced::Command::none()
            }
            BluetoothMessage::Toggle => {
                let _ = self.commander.send(BluetoothCommand::TogglePower);

                iced::Command::none()
            }
            BluetoothMessage::More => {
                if let Some(cmd) = &config.bluetooth_more_cmd {
                    crate::utils::launcher::execute_command(cmd.to_string());
                    menu.close()
                } else {
                    iced::Command::none()
                }
            }
        }
    }

    pub fn get_quick_setting_button(
        &self,
        sub_menu: Option<SubMenu>,
        show_more_button: bool,
    ) -> Option<(Element<Message>, Option<Element<Message>>)> {
        Some((
            quick_setting_button(
                Icons::Bluetooth,
                "Bluetooth".to_owned(),
                None,
                self.state == BluetoothState::Active,
                Message::Bluetooth(BluetoothMessage::Toggle),
                Some((
                    SubMenu::Bluetooth,
                    sub_menu,
                    Message::ToggleSubMenu(SubMenu::Bluetooth),
                ))
                .filter(|_| self.state == BluetoothState::Active),
            ),
            sub_menu
                .filter(|menu_type| *menu_type == SubMenu::Bluetooth)
                .map(|_| sub_menu_wrapper(self.bluetooth_menu(show_more_button))),
        ))
    }

    pub fn bluetooth_menu(&self, show_more_button: bool) -> Element<Message> {
        let main = if self.devices.is_empty() {
            text("No devices connected").into()
        } else {
            Column::with_children(
                self.devices
                    .iter()
                    .map(|d| {
                        Row::with_children(
                            vec![
                                Some(text(d.name.to_string()).width(iced::Length::Fill).into()),
                                d.battery.map(Self::battery_level),
                            ]
                            .into_iter()
                            .flatten()
                            .collect::<Vec<_>>(),
                        )
                        .into()
                    })
                    .collect::<Vec<Element<Message>>>(),
            )
            .spacing(8)
            .into()
        };

        if show_more_button {
            column!(
                main,
                horizontal_rule(1),
                button("More")
                    .on_press(Message::Bluetooth(BluetoothMessage::More))
                    .padding([4, 12])
                    .width(Length::Fill)
                    .style(Button::custom(GhostButtonStyle))
            )
            .spacing(12)
            .into()
        } else {
            main
        }
    }

    fn battery_level<'a>(battery: u8) -> Element<'a, Message> {
        let color = if battery <= 20 { RED } else { TEXT };
        row!(
            icon(match battery {
                0..=20 => Icons::Battery0,
                21..=40 => Icons::Battery1,
                41..=60 => Icons::Battery2,
                61..=80 => Icons::Battery3,
                _ => Icons::Battery4,
            })
            .style(color),
            text(format!("{}%", battery)).style(color)
        )
        .spacing(8)
        .width(iced::Length::Shrink)
        .into()
    }

    pub fn subscription(&self) -> iced::Subscription<BluetoothMessage> {
        crate::utils::bluetooth::subscription(self.commander.give_receiver())
    }
}
