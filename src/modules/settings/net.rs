use crate::{
    components::icons::{icon, Icons},
    style::{SettingsButtonStyle, GREEN, TEXT, YELLOW},
    utils::net::{
        get_wifi_icon, ActiveConnection, NetCommand, Vpn, Wifi, WifiConnection, WifiDeviceState,
    },
};
use iced::{
    alignment::Horizontal,
    theme::Button,
    widget::{button, column, container, horizontal_rule, row, text, toggler, Column, Text},
    Element,
};

use super::Settings;

#[derive(Debug, Clone)]
pub enum NetMessage {
    WifiDeviceState(WifiDeviceState),
    ActiveConnection(Option<ActiveConnection>),
    ToggleWifi,
    ScanNearByWifi,
    VpnActive(bool),
    VpnConnections(Vec<Vpn>),
    VpnToggle(String),
    DeactivateVpns,
    NearByWifi(Vec<WifiConnection>),
}

impl NetMessage {
    pub fn update(self, settings: &mut Settings) {
        println!("{:?}", self);
        match self {
            NetMessage::WifiDeviceState(state) => {
                settings.wifi_device_state = state;
            }
            NetMessage::ActiveConnection(connection) => {
                settings.active_connection = connection;
            }
            NetMessage::ToggleWifi => {
                // let _ = settings.net_commander.send(NetCommand::ToggleWifi);
            }
            NetMessage::ScanNearByWifi => {
                settings.scanning_nearby_wifi = true;
                let _ = settings.net_commander.send(NetCommand::ScanNearByWifi);
            }
            NetMessage::VpnActive(active) => {
                settings.vpn_active = active;
            }
            NetMessage::VpnConnections(connections) => {
                settings.vpn_connections = connections;
            }
            NetMessage::VpnToggle(name) => {
                if let Some(vpn) = settings
                    .vpn_connections
                    .iter_mut()
                    .find(|vpn| vpn.name == name)
                {
                    vpn.is_active = !vpn.is_active;
                    if vpn.is_active {
                        let _ = settings.net_commander.send(NetCommand::ActivateVpn(name));
                    } else {
                        let _ = settings.net_commander.send(NetCommand::DeactivateVpn(name));
                    }
                }
            }
            NetMessage::DeactivateVpns => {
                for vpn in settings.vpn_connections.iter_mut() {
                    vpn.is_active = false;
                    let _ = settings
                        .net_commander
                        .send(NetCommand::DeactivateVpn(vpn.name.clone()));
                }
            }
            NetMessage::NearByWifi(connections) => {
                settings.scanning_nearby_wifi = false;
                settings.nearby_wifi = connections;
            }
        };
    }
}

pub fn active_connection_indicator<'a>(data: &ActiveConnection) -> Text<'a, iced::Renderer> {
    let icon_type = data.get_icon();
    let color = data.get_color();

    icon(icon_type).style(color)
}

pub fn vpn_indicator<'a>() -> Text<'a, iced::Renderer> {
    icon(Icons::Vpn).style(YELLOW)
}

pub fn wifi_menu<'a>(
    scanning_nearby_wifi: bool,
    active_connection: Option<&Wifi>,
    nearby_wifi: &Vec<WifiConnection>,
) -> Element<'a, NetMessage> {
    column!(
        container(
            row!(
                text(if scanning_nearby_wifi {
                    "Scanning..."
                } else {
                    ""
                })
                .size(12),
                button(icon(Icons::Refresh))
                    .padding([4, 8])
                    .style(Button::custom(SettingsButtonStyle))
                    .on_press(NetMessage::ScanNearByWifi),
            )
            .spacing(8)
            .align_items(iced::Alignment::Center),
        )
        .width(iced::Length::Fill)
        .align_x(Horizontal::Right),
        horizontal_rule(1),
        Column::with_children(
            nearby_wifi
                .iter()
                .map(|wifi| {
                    let color = if active_connection.is_some_and(|c| c.ssid == wifi.ssid) {
                        GREEN
                    } else {
                        TEXT
                    };
                    row!(
                        icon(get_wifi_icon(wifi.strength))
                            .style(color)
                            .width(iced::Length::Shrink),
                        text(wifi.ssid.to_string())
                            .style(color)
                            .width(iced::Length::Fill),
                    )
                    .spacing(8)
                    .into()
                })
                .collect::<Vec<Element<'a, NetMessage>>>(),
        )
        .spacing(8)
    )
    .spacing(4)
    .into()
}

pub fn vpn_menu<'a>(vpn_connections: &'a Vec<Vpn>) -> Element<'a, NetMessage> {
    Column::with_children(
        vpn_connections
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
            .collect::<Vec<Element<'a, NetMessage>>>(),
    )
    .spacing(8)
    .into()
}
