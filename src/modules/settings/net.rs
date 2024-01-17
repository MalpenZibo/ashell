use crate::{
    components::icons::{icon, Icons},
    style::YELLOW,
    utils::net::{ActiveConnection, NetCommand, Vpn},
};
use iced::{
    widget::{row, text, toggler, Column, Text},
    Element,
};
use zbus::zvariant::OwnedObjectPath;

use super::Settings;

#[derive(Debug, Clone)]
pub enum NetMessage {
    ActiveConnection(Option<ActiveConnection>),
    VpnActive(bool),
    VpnConnections(Vec<Vpn>),
    VpnToggle(OwnedObjectPath),
    DeactivateVpns,
}

impl NetMessage {
    pub fn update(self, settings: &mut Settings) {
        println!("{:?}", self);
        match self {
            NetMessage::ActiveConnection(connection) => {
                settings.active_connection = connection;
            }
            NetMessage::VpnActive(active) => {
                settings.vpn_active = active;
            }
            NetMessage::VpnConnections(connections) => {
                settings.vpn_connections = connections;
            }
            NetMessage::VpnToggle(object_path) => {
                if let Some(vpn) = settings
                    .vpn_connections
                    .iter_mut()
                    .find(|vpn| vpn.object_path == object_path)
                {
                    let active_connection = vpn.active_object_path.take();
                    let _ =
                        settings
                            .net_commander
                            .send(if let Some(object_path) = active_connection {
                                NetCommand::DeactivateVpn(object_path)
                            } else {
                                NetCommand::ActivateVpn(object_path)
                            });
                }
            }
            NetMessage::DeactivateVpns => {
                for vpn in settings.vpn_connections.iter_mut() {
                    if let Some(active_connection) = vpn.active_object_path.take() {
                        let _ = settings
                            .net_commander
                            .send(NetCommand::DeactivateVpn(active_connection));
                    }
                }
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

pub fn vpn_menu<'a>(vpn_connections: &'a Vec<Vpn>) -> Element<'a, NetMessage> {
    Column::with_children(
        vpn_connections
            .iter()
            .map(|vpn| {
                row!(
                    text(vpn.name.to_string()).width(iced::Length::Fill),
                    toggler(None, vpn.active_object_path.is_some(), |_| {
                        NetMessage::VpnToggle(vpn.object_path.clone())
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
