use super::{quick_setting_button, sub_menu_wrapper, Message, Settings, SubMenu};
use crate::{
    components::icons::{icon, Icons},
    style::{RED, TEXT},
    utils::bluetooth::BluetoothCommand,
};
use iced::{
    widget::{row, text, Column, Row},
    Element,
};

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

impl BluetoothMessage {
    pub fn update(self, settings: &mut Settings) -> iced::Command<Message> {
        match self {
            BluetoothMessage::Status(state) => {
                settings.bluetooth_state = state;

                iced::Command::none()
            }
            BluetoothMessage::DeviceList(devices) => {
                settings.bluetooth_devices = devices;

                iced::Command::none()
            }
            BluetoothMessage::Toggle => {
                let _ = settings
                    .bluetooth_commander
                    .send(BluetoothCommand::TogglePower);

                iced::Command::none()
            }
        }
    }
}

pub fn get_bluetooth_quick_setting_button<'a>(
    settings: &Settings,
) -> Option<(Element<'a, Message>, Option<Element<'a, Message>>)> {
    Some((
        quick_setting_button(
            Icons::Bluetooth,
            "Bluetooth".to_owned(),
            None,
            settings.bluetooth_state == BluetoothState::Active,
            Message::Bluetooth(BluetoothMessage::Toggle),
            Some((
                SubMenu::Bluetooth,
                settings.sub_menu,
                Message::ToggleSubMenu(SubMenu::Bluetooth),
            ))
            .filter(|_| settings.bluetooth_state == BluetoothState::Active),
        ),
        settings
            .sub_menu
            .filter(|menu_type| *menu_type == SubMenu::Bluetooth)
            .map(|_| {
                sub_menu_wrapper(
                    bluetooth_menu(&settings.bluetooth_devices).map(Message::Bluetooth),
                )
            }),
    ))
}

pub fn bluetooth_menu<'a>(devices: &[Device]) -> Element<'a, BluetoothMessage> {
    Column::with_children(
        devices
            .iter()
            .map(|d| {
                Row::with_children(
                    vec![
                        Some(text(d.name.to_string()).width(iced::Length::Fill).into()),
                        d.battery.map(battery_level),
                    ]
                    .into_iter()
                    .flatten()
                    .collect::<Vec<_>>(),
                )
                .into()
            })
            .collect::<Vec<Element<'a, BluetoothMessage>>>(),
    )
    .spacing(8)
    .into()
}

fn battery_level<'a>(battery: u8) -> Element<'a, BluetoothMessage> {
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
