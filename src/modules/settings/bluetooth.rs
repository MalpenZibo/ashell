use super::{quick_setting_button, sub_menu_wrapper, Message, SubMenu};
use crate::{
    components::icons::{icon, Icons},
    services::{
        bluetooth::{BluetoothData, BluetoothService, BluetoothState},
        ServiceEvent,
    },
    style::GhostButtonStyle,
};
use iced::{
    theme::Button,
    widget::{button, column, container, horizontal_rule, row, text, Column, Row},
    Element, Length, Theme,
};

#[derive(Debug, Clone)]
pub enum BluetoothMessage {
    Event(ServiceEvent<BluetoothService>),
    Toggle,
    More,
}

impl BluetoothData {
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
                                Some(text(d.name.to_string()).width(Length::Fill).into()),
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
        container(
            row!(
                icon(match battery {
                    0..=20 => Icons::Battery0,
                    21..=40 => Icons::Battery1,
                    41..=60 => Icons::Battery2,
                    61..=80 => Icons::Battery3,
                    _ => Icons::Battery4,
                }),
                text(format!("{}%", battery))
            )
            .spacing(8)
            .width(Length::Shrink),
        )
        .style(move |theme: &Theme| container::Appearance {
            text_color: Some(if battery <= 20 {
                theme.palette().danger
            } else {
                theme.palette().text
            }),
            ..container::Appearance::default()
        })
        .into()
    }
}
