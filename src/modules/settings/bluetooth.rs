use super::{quick_setting_button, sub_menu_wrapper, Message, SubMenu};
use crate::{
    components::icons::{icon, Icons},
    style::{RED, TEXT},
    utils::{bluetooth::BluetoothCommand, Commander},
};
use iced::{
    widget::{row, text, Column, Row},
    Element,
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
        sub_menu: &mut Option<SubMenu>,
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
        }
    }

    pub fn get_bluetooth_quick_setting_button(
        &self,
        sub_menu: Option<SubMenu>,
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
                .map(|_| sub_menu_wrapper(self.bluetooth_menu())),
        ))
    }

    pub fn bluetooth_menu(&self) -> Element<Message> {
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
