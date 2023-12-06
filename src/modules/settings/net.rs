use crate::{components::icons::{icon, Icons}, utils::net::Wifi, style::YELLOW};
use iced::widget::Text;

pub fn wifi_indicator<'a>(data: &Wifi) -> Text<'a, iced::Renderer> {
    let icon_type = data.get_icon();
    let color = data.get_color();

    icon(icon_type).style(color)
}

pub fn vpn_indicator<'a>() -> Text<'a, iced::Renderer> {
    icon(Icons::Vpn).style(YELLOW)
}
