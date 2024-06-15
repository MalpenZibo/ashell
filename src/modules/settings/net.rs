use crate::{
    components::icons::{icon, Icons},
    menu::Menu,
    style::{GhostButtonStyle, SettingsButtonStyle, GREEN, TEXT, YELLOW},
    utils::{
        net::{
            get_wifi_icon, get_wifi_lock_icon, ActiveConnection, NetCommand, Vpn, Wifi,
            WifiConnection, WifiDeviceState,
        },
        Commander,
    },
};
use iced::{
    theme::Button,
    widget::{button, column, container, horizontal_rule, row, scrollable, text, toggler, Column},
    Element, Length,
};

use super::{quick_setting_button, sub_menu_wrapper, Message, SubMenu};

#[derive(Debug, Clone)]
pub enum NetMessage {
    WifiDeviceState(WifiDeviceState),
    ActiveConnection(Option<ActiveConnection>),
    ToggleWifi,
    ActivateWifi(String, Option<String>),
    RequestWifiPassword(String),
    ScanNearByWifi,
    VpnActive(bool),
    VpnConnections(Vec<Vpn>),
    VpnToggle(String),
    NearByWifi(Vec<WifiConnection>),
}

pub struct Net {
    commander: Commander<NetCommand>,
    wifi_device_state: WifiDeviceState,
    scanning_nearby_wifi: bool,
    active_connection: Option<ActiveConnection>,
    vpn_active: bool,
    vpn_connections: Vec<Vpn>,
    nearby_wifi: Vec<WifiConnection>,
}

impl Net {
    pub fn new() -> Self {
        Self {
            commander: Commander::new(),
            wifi_device_state: WifiDeviceState::Unavailable,
            scanning_nearby_wifi: false,
            active_connection: None,
            vpn_active: false,
            vpn_connections: Vec::new(),
            nearby_wifi: Vec::new(),
        }
    }

    pub fn update<Message>(
        &mut self,
        msg: NetMessage,
        menu: &mut Menu,
        password_dialog: &mut Option<(String, String)>,
    ) -> iced::Command<Message> {
        match msg {
            NetMessage::WifiDeviceState(state) => {
                self.wifi_device_state = state;

                iced::Command::none()
            }
            NetMessage::ActiveConnection(connection) => {
                self.active_connection = connection;

                iced::Command::none()
            }
            NetMessage::ToggleWifi => {
                let _ = self.commander.send(NetCommand::ToggleWifi);

                iced::Command::none()
            }
            NetMessage::ActivateWifi(ssid, password) => {
                let _ = self
                    .commander
                    .send(NetCommand::ActivateWifiConnection(ssid, password));

                iced::Command::none()
            }
            NetMessage::RequestWifiPassword(ssid) => {
                *password_dialog = Some((ssid, "".to_string()));

                menu.set_keyboard_interactivity()
            }
            NetMessage::ScanNearByWifi => {
                self.scanning_nearby_wifi = true;
                let _ = self.commander.send(NetCommand::ScanNearByWifi);

                iced::Command::none()
            }
            NetMessage::VpnActive(active) => {
                self.vpn_active = active;

                iced::Command::none()
            }
            NetMessage::VpnConnections(connections) => {
                self.vpn_connections = connections;

                iced::Command::none()
            }
            NetMessage::VpnToggle(name) => {
                if let Some(vpn) = self.vpn_connections.iter_mut().find(|vpn| vpn.name == name) {
                    vpn.is_active = !vpn.is_active;
                    if vpn.is_active {
                        let _ = self.commander.send(NetCommand::ActivateVpn(name));
                    } else {
                        let _ = self.commander.send(NetCommand::DeactivateVpn(name));
                    }
                }

                iced::Command::none()
            }
            NetMessage::NearByWifi(connections) => {
                self.scanning_nearby_wifi = false;
                self.nearby_wifi = connections;

                iced::Command::none()
            }
        }
    }

    pub fn get_vpn_connections(&self) {
        self.commander.send(NetCommand::GetVpnConnections).unwrap();
    }

    pub fn get_nearby_wifi(&mut self) {
        self.scanning_nearby_wifi = true;
        self.commander.send(NetCommand::ScanNearByWifi).unwrap();
    }

    pub fn activate_wifi(&self, ssid: String, password: String) {
        let _ = self
            .commander
            .send(NetCommand::ActivateWifiConnection(ssid, Some(password)));
    }

    pub fn active_connection_indicator(&self) -> Option<Element<Message>> {
        self.active_connection.as_ref().map(|a| {
            let icon_type = a.get_icon();
            let color = a.get_color();

            icon(icon_type).style(color).into()
        })
    }

    pub fn vpn_indicator(&self) -> Option<Element< Message>> {
        if self.vpn_active {
            Some(icon(Icons::Vpn).style(YELLOW).into())
        } else {
            None
        }
    }

    pub fn get_wifi_quick_setting_button(
        &self,
        sub_menu: Option<SubMenu>,
    ) -> Option<(Element<Message>, Option<Element<Message>>)> {
        self.active_connection.as_ref().map_or_else(
            || {
                if self.wifi_device_state != WifiDeviceState::Unavailable {
                    Some((
                        quick_setting_button(
                            Icons::Wifi0,
                            "Wi-Fi".to_string(),
                            None,
                            self.wifi_device_state == WifiDeviceState::Active,
                            Message::Net(NetMessage::ToggleWifi),
                            Some((
                                SubMenu::Wifi,
                                sub_menu,
                                Message::ToggleSubMenu(SubMenu::Wifi),
                            ))
                            .filter(|_| self.wifi_device_state == WifiDeviceState::Active),
                        ),
                        sub_menu
                            .filter(|menu_type| *menu_type == SubMenu::Wifi)
                            .map(|_| sub_menu_wrapper(self.wifi_menu(None)).map(Message::Net)),
                    ))
                } else {
                    None
                }
            },
            |a| match a {
                ActiveConnection::Wifi(wifi) => Some((
                    quick_setting_button(
                        a.get_icon(),
                        "Wi-Fi".to_string(),
                        Some(wifi.ssid.clone()),
                        true,
                        Message::Net(NetMessage::ToggleWifi),
                        Some((
                            SubMenu::Wifi,
                            sub_menu,
                            Message::ToggleSubMenu(SubMenu::Wifi),
                        )),
                    ),
                    sub_menu
                        .filter(|menu_type| *menu_type == SubMenu::Wifi)
                        .map(|_| sub_menu_wrapper(self.wifi_menu(Some(wifi))).map(Message::Net)),
                )),
                _ => None,
            },
        )
    }

    pub fn get_vpn_quick_setting_button(
        &self,
        sub_menu: Option<SubMenu>,
    ) -> Option<(Element<Message>, Option<Element<Message>>)> {
        Some((
            quick_setting_button(
                Icons::Vpn,
                "Vpn".to_string(),
                None,
                self.vpn_active,
                Message::ToggleSubMenu(SubMenu::Vpn),
                None,
            ),
            sub_menu
                .filter(|menu_type| *menu_type == SubMenu::Vpn)
                .map(|_| sub_menu_wrapper(self.vpn_menu()).map(Message::Net)),
        ))
    }

    pub fn wifi_menu(&self, active_connection: Option<&Wifi>) -> Element<NetMessage> {
        column!(
            row!(
                text("Nearby Wifi").width(Length::Fill),
                text(if self.scanning_nearby_wifi {
                    "Scanning..."
                } else {
                    ""
                })
                .size(12),
                button(icon(Icons::Refresh))
                    .padding([4, 10])
                    .style(Button::custom(SettingsButtonStyle))
                    .on_press(NetMessage::ScanNearByWifi),
            )
            .spacing(8)
            .width(iced::Length::Fill)
            .align_items(iced::Alignment::Center),
            horizontal_rule(1),
            container(scrollable(
                Column::with_children(
                    self.nearby_wifi
                        .iter()
                        .map(|wifi| {
                            let is_active = active_connection.is_some_and(|c| c.ssid == wifi.ssid);
                            let color = if is_active { GREEN } else { TEXT };
                            button(
                                row!(
                                    icon(if wifi.public {
                                        get_wifi_icon(wifi.strength)
                                    } else {
                                        get_wifi_lock_icon(wifi.strength)
                                    })
                                    .style(color)
                                    .width(iced::Length::Shrink),
                                    text(wifi.ssid.to_string())
                                        .style(color)
                                        .width(iced::Length::Fill),
                                )
                                .align_items(iced::Alignment::Center)
                                .spacing(8),
                            )
                            .style(iced::theme::Button::custom(GhostButtonStyle))
                            .padding([8, 8])
                            .on_press_maybe(if !is_active {
                                Some(if wifi.known {
                                    NetMessage::ActivateWifi(wifi.ssid.clone(), None)
                                } else {
                                    NetMessage::RequestWifiPassword(wifi.ssid.clone())
                                })
                            } else {
                                None
                            })
                            .width(Length::Fill)
                            .into()
                        })
                        .collect::<Vec<Element<NetMessage>>>(),
                )
                .spacing(4)
            ))
            .max_height(200)
        )
        .spacing(8)
        .into()
    }

    pub fn vpn_menu(&self) -> Element<NetMessage> {
        Column::with_children(
            self.vpn_connections
                .iter()
                .map(|vpn| {
                    row!(
                        text(vpn.name.to_string()).width(iced::Length::Fill),
                        toggler(None, vpn.is_active, |_| {
                            NetMessage::VpnToggle(vpn.name.clone())
                        })
                        .width(iced::Length::Shrink)
                    )
                    .into()
                })
                .collect::<Vec<Element<NetMessage>>>(),
        )
        .spacing(8)
        .into()
    }

    pub fn subscription(&self) -> iced::Subscription<NetMessage> {
        crate::utils::net::subscription(self.commander.give_receiver())
    }
}
