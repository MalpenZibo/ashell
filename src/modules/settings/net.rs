use crate::{
    components::icons::{icon, Icons},
    style::YELLOW,
    utils::net::Wifi,
};
use iced::widget::Text;

use super::Settings;

#[derive(Debug, Clone)]
pub enum NetMessage {
    Wifi(Option<Wifi>),
    VpnActive(bool),
}

impl NetMessage {
    pub fn update(self, settings: &mut Settings) {
        match self {
            NetMessage::Wifi(wifi) => {
                settings.wifi = wifi;
            }
            NetMessage::VpnActive(active) => {
                settings.vpn_active = active;
            }
        };
    }
}

pub fn wifi_indicator<'a>(data: &Wifi) -> Text<'a, iced::Renderer> {
    let icon_type = data.get_icon();
    let color = data.get_color();

    icon(icon_type).style(color)
}

pub fn vpn_indicator<'a>() -> Text<'a, iced::Renderer> {
    icon(Icons::Vpn).style(YELLOW)
}
