use crate::{components::icons::icon, utils::battery::BatteryData};
use iced::widget::{row, text, Row};

pub fn battery_indicator<'a, Message>(data: BatteryData) -> Row<'a, Message, iced::Renderer> {
    let icon_type = data.get_icon();
    let color = data.get_color();

    row!(
        icon(icon_type).style(color),
        text(format!("{}%", data.capacity)).style(color)
    )
}
