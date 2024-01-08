use crate::{
    components::icons::icon,
    style::SURFACE_0,
    utils::battery::{BatteryData, BatteryStatus},
};
use iced::{
    widget::{container, row, text, Container, Row},
    Theme,
};

pub fn battery_indicator<'a, Message>(data: BatteryData) -> Row<'a, Message, iced::Renderer> {
    let icon_type = data.get_icon();
    let color = data.get_color();

    row!(
        icon(icon_type).style(color),
        text(format!("{}%", data.capacity)).style(color)
    )
}

pub fn settings_battery_indicator<'a, Message: 'static>(
    data: BatteryData,
) -> Container<'a, Message, iced::Renderer> {
    container({
        let battery_info = row!(
            icon(data.get_icon()).style(data.get_color()),
            text(format!("{}%", data.capacity)).style(data.get_color())
        )
        .spacing(4);
        match data.status {
            BatteryStatus::Charging(remaining) => {
                let minutes = (remaining.as_secs() / 60) % 60;
                let hours = (remaining.as_secs() / 60) / 60;
                row!(
                    battery_info,
                    text(format!("Full in {}:{:0>2}", hours, minutes))
                )
                .spacing(16)
            }
            BatteryStatus::Discharging(remaining) => {
                let minutes = (remaining.as_secs() / 60) % 60;
                let hours = (remaining.as_secs() / 60) / 60;
                row!(
                    battery_info,
                    text(format!("Empty in {}:{:0>2}", hours, minutes))
                )
                .spacing(16)
            }
            BatteryStatus::Full => row!(battery_info),
        }
    })
    .padding(8)
    .style(move |_: &Theme| iced::widget::container::Appearance {
        background: iced::Background::Color(SURFACE_0).into(),
        border_radius: 32.0.into(),
        ..iced::widget::container::Appearance::default()
    })
}
