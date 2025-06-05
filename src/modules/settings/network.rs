use super::{Message, SubMenu, quick_setting_button};
use crate::{
    components::icons::{Icons, icon},
    services::{
        ServiceEvent,
        network::{
            AccessPoint, ActiveConnectionInfo, KnownConnection, NetworkData, NetworkService, Vpn,
            dbus::ConnectivityState,
        },
    },
    style::{ghost_button_style, settings_button_style},
    utils::IndicatorState,
};
use iced::{
    Alignment, Element, Length, Theme,
    widget::{Column, button, column, container, horizontal_rule, row, scrollable, text, toggler},
    window::Id,
};

#[derive(Debug, Clone)]
pub enum NetworkMessage {
    Event(ServiceEvent<NetworkService>),
    ToggleWiFi,
    ScanNearByWiFi,
    WiFiMore(Id),
    VpnMore(Id),
    SelectAccessPoint(AccessPoint),
    RequestWiFiPassword(Id, String),
    ToggleVpn(Vpn),
    ToggleAirplaneMode,
}

static WIFI_SIGNAL_ICONS: [Icons; 6] = [
    Icons::Wifi0,
    Icons::Wifi1,
    Icons::Wifi2,
    Icons::Wifi3,
    Icons::Wifi4,
    Icons::Wifi5,
];

static WIFI_LOCK_SIGNAL_ICONS: [Icons; 5] = [
    Icons::WifiLock1,
    Icons::WifiLock2,
    Icons::WifiLock3,
    Icons::WifiLock4,
    Icons::WifiLock5,
];

impl ActiveConnectionInfo {
    pub fn get_wifi_icon(signal: u8) -> Icons {
        WIFI_SIGNAL_ICONS[1 + f32::round(signal as f32 / 100. * 4.) as usize]
    }

    pub fn get_wifi_lock_icon(signal: u8) -> Icons {
        WIFI_LOCK_SIGNAL_ICONS[f32::round(signal as f32 / 100. * 4.) as usize]
    }

    pub fn get_icon(&self) -> Icons {
        match self {
            Self::WiFi { strength, .. } => Self::get_wifi_icon(*strength),
            Self::Wired { .. } => Icons::Ethernet,
            Self::Vpn { .. } => Icons::Vpn,
        }
    }

    pub fn get_indicator_state(&self) -> IndicatorState {
        match self {
            Self::WiFi {
                strength: 0 | 1, ..
            } => IndicatorState::Warning,
            _ => IndicatorState::Normal,
        }
    }
}

impl NetworkData {
    pub fn get_connection_indicator<Message: 'static>(&self) -> Option<Element<Message>> {
        if self.airplane_mode || !self.wifi_present {
            None
        } else {
            Some(
                self.active_connections
                    .iter()
                    .find(|c| {
                        matches!(c, ActiveConnectionInfo::WiFi { .. })
                            || matches!(c, ActiveConnectionInfo::Wired { .. })
                    })
                    .map_or_else(
                        || icon(Icons::Wifi0).into(),
                        |a| {
                            let icon_type = a.get_icon();
                            let state = (self.connectivity, a.get_indicator_state());

                            container(icon(icon_type))
                                .style(move |theme: &Theme| container::Style {
                                    text_color: match state {
                                        (ConnectivityState::Full, IndicatorState::Warning) => {
                                            Some(theme.extended_palette().danger.weak.color)
                                        }
                                        (ConnectivityState::Full, _) => None,
                                        _ => Some(theme.palette().danger),
                                    },
                                    ..Default::default()
                                })
                                .into()
                        },
                    ),
            )
        }
    }

    pub fn get_vpn_indicator<Message: 'static>(&self) -> Option<Element<Message>> {
        self.active_connections
            .iter()
            .find(|c| matches!(c, ActiveConnectionInfo::Vpn { .. }))
            .map(|a| {
                let icon_type = a.get_icon();

                container(icon(icon_type))
                    .style(|theme: &Theme| container::Style {
                        text_color: Some(theme.extended_palette().danger.weak.color),
                        ..Default::default()
                    })
                    .into()
            })
    }

    pub fn get_wifi_quick_setting_button(
        &self,
        id: Id,
        sub_menu: Option<SubMenu>,
        show_more_button: bool,
        opacity: f32,
    ) -> Option<(Element<Message>, Option<Element<Message>>)> {
        if self.wifi_present {
            let active_connection = self.active_connections.iter().find_map(|c| match c {
                ActiveConnectionInfo::WiFi { name, strength, .. } => {
                    Some((name, strength, c.get_icon()))
                }
                _ => None,
            });

            Some((
                quick_setting_button(
                    active_connection.map_or_else(|| Icons::Wifi0, |(_, _, icon)| icon),
                    "Wi-Fi".to_string(),
                    active_connection.map(|(name, _, _)| name.clone()),
                    self.wifi_enabled,
                    Message::Network(NetworkMessage::ToggleWiFi),
                    Some((
                        SubMenu::Wifi,
                        sub_menu,
                        Message::ToggleSubMenu(SubMenu::Wifi),
                    ))
                    .filter(|_| self.wifi_enabled),
                    opacity,
                ),
                sub_menu
                    .filter(|menu_type| *menu_type == SubMenu::Wifi)
                    .map(|_| {
                        self.wifi_menu(
                            id,
                            active_connection.map(|(name, strengh, _)| (name.as_str(), *strengh)),
                            show_more_button,
                            opacity,
                        )
                        .map(Message::Network)
                    }),
            ))
        } else {
            None
        }
    }

    pub fn get_vpn_quick_setting_button(
        &self,
        id: Id,
        sub_menu: Option<SubMenu>,
        show_more_button: bool,
        opacity: f32,
    ) -> Option<(Element<Message>, Option<Element<Message>>)> {
        self.known_connections
            .iter()
            .any(|c| matches!(c, KnownConnection::Vpn { .. }))
            .then(|| {
                (
                    quick_setting_button(
                        Icons::Vpn,
                        "Vpn".to_string(),
                        None,
                        self.active_connections
                            .iter()
                            .any(|c| matches!(c, ActiveConnectionInfo::Vpn { .. })),
                        Message::ToggleSubMenu(SubMenu::Vpn),
                        None,
                        opacity,
                    ),
                    sub_menu
                        .filter(|menu_type| *menu_type == SubMenu::Vpn)
                        .map(|_| {
                            self.vpn_menu(id, show_more_button, opacity)
                                .map(Message::Network)
                        }),
                )
            })
    }

    pub fn wifi_menu(
        &self,
        id: Id,
        active_connection: Option<(&str, u8)>,
        show_more_button: bool,
        opacity: f32,
    ) -> Element<NetworkMessage> {
        let main = column!(
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
                    .style(settings_button_style(opacity))
                    .on_press(NetworkMessage::ScanNearByWiFi),
            )
            .spacing(8)
            .width(Length::Fill)
            .align_y(Alignment::Center),
            horizontal_rule(1),
            container(scrollable(
                Column::with_children(
                    self.wireless_access_points
                    .iter()
                    .filter_map(|ac| if active_connection.is_some_and(|(ssid, _)| ssid == ac.ssid) {Some((ac, true))} else {None })
                    .chain(self.wireless_access_points
                        .iter()
                        .filter_map(|ac| if active_connection.is_some_and(|(ssid, _)| ssid == ac.ssid) {None} else {Some((ac, false))})
                    )
                        .map(|(ac, is_active)| {
                            let is_known = self.known_connections.iter().any(|c| {
                                matches!(
                                    c,
                                    KnownConnection::AccessPoint(AccessPoint { ssid, .. }) if ssid == &ac.ssid
                                )
                            });

                            button(
                                container(
                                    row!(
                                        icon(if ac.public {
                                            ActiveConnectionInfo::get_wifi_icon(ac.strength)
                                        } else {
                                            ActiveConnectionInfo::get_wifi_lock_icon(ac.strength)
                                        })
                                        .width(Length::Shrink),
                                        text(ac.ssid.clone()).width(Length::Fill),
                                    )
                                    .align_y(Alignment::Center)
                                    .spacing(8),
                                )
                                .style(move |theme: &Theme| {
                                    container::Style {
                                        text_color: if is_active {
                                            Some(theme.palette().success)
                                        } else {
                                            None
                                        },
                                        ..Default::default()
                                    }
                                }),
                            )
                            .style(ghost_button_style(opacity))
                            .padding([8, 8])
                            .on_press_maybe(if !is_active {
                                Some(if is_known {
                                    NetworkMessage::SelectAccessPoint(ac.clone())
                                } else {
                                    NetworkMessage::RequestWiFiPassword(id, ac.ssid.clone())
                                })
                            } else {
                                None
                            })
                            .width(Length::Fill)
                            .into()
                        })
                        .collect::<Vec<Element<NetworkMessage>>>(),
                )
                .spacing(4)
            ))
            .max_height(200),
        )
        .spacing(8);

        if show_more_button {
            column!(
                main,
                horizontal_rule(1),
                button("More")
                    .on_press(NetworkMessage::WiFiMore(id))
                    .padding([4, 12])
                    .width(Length::Fill)
                    .style(ghost_button_style(opacity))
            )
            .spacing(12)
            .into()
        } else {
            main.into()
        }
    }

    pub fn vpn_menu(
        &self,
        id: Id,
        show_more_button: bool,
        opacity: f32,
    ) -> Element<NetworkMessage> {
        let main = Column::with_children(
            self.known_connections
                .iter()
                .filter_map(|c| match c {
                    KnownConnection::Vpn(vpn) => Some(vpn),
                    _ => None,
                })
                .map(|vpn| {
                    let is_active = self.active_connections.iter().any(
                        |c| matches!(c, ActiveConnectionInfo::Vpn { name, .. } if name == &vpn.name),
                    );

                    row!(
                        text(vpn.name.to_string()).width(Length::Fill),
                        toggler(is_active)
                            .on_toggle(|_| { NetworkMessage::ToggleVpn(vpn.clone()) })
                            .width(Length::Shrink),
                    )
                    .into()
                })
                .collect::<Vec<Element<NetworkMessage>>>(),
        )
        .spacing(8);

        if show_more_button {
            column!(
                main,
                horizontal_rule(1),
                button("More")
                    .on_press(NetworkMessage::VpnMore(id))
                    .padding([4, 12])
                    .width(Length::Fill)
                    .style(ghost_button_style(opacity))
            )
            .spacing(12)
            .into()
        } else {
            main.into()
        }
    }

    pub fn get_airplane_mode_quick_setting_button(
        &self,
        opacity: f32,
    ) -> (Element<Message>, Option<Element<Message>>) {
        (
            quick_setting_button(
                Icons::Airplane,
                "Airplane Mode".to_string(),
                None,
                self.airplane_mode,
                Message::Network(NetworkMessage::ToggleAirplaneMode),
                None,
                opacity,
            ),
            None,
        )
    }
}
