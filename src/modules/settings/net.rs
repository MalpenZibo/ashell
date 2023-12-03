use iced::widget::{Text, row, Row};

use crate::{
    components::icons::{icon, Icons},
    utils::net::{ActiveConnection, Vpn}, style::YELLOW,
};

pub fn net_indicator<'a, Message>(
    active_connection: &Option<ActiveConnection>,
    active_vpn: &Vec<Vpn>,
) -> Row<'a, Message, iced::Renderer> {
    let mut row = row!().spacing(4);

    if let Some(connection) = active_connection {
        row = row.push(connection_indicator(connection));
    }

    if !active_vpn.is_empty() {
        row = row.push(vpn_indicator());
    }

    row
}

pub fn connection_indicator<'a>(data: &ActiveConnection) -> Text<'a, iced::Renderer> {
    let icon_type = data.get_icon();
    let color = data.get_color();

    icon(icon_type).style(color)
}

pub fn vpn_indicator<'a>() -> Text<'a, iced::Renderer> {
    icon(Icons::Vpn).style(YELLOW)
}
