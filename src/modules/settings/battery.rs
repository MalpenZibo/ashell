use crate::{
    components::icons::icon,
    style::SURFACE_0,
    utils::{
        battery::{BatteryData, BatteryStatus},
        format_duration,
    },
};
use iced::{
    widget::{container, row, text, Container, Row},
    Border, Theme,
};

pub fn battery_indicator<'a, Message>(
    data: BatteryData,
) -> Row<'a, Message, Theme, iced::Renderer> {
    let icon_type = data.get_icon();
    let color = data.get_color();

    row!(
        icon(icon_type).style(color),
        text(format!("{}%", data.capacity)).style(color)
    )
    .spacing(4)
    .align_items(iced::Alignment::Center)
}

pub fn settings_battery_indicator<'a, Message: 'static>(
    data: BatteryData,
) -> Container<'a, Message, Theme, iced::Renderer> {
    container({
        let battery_info = row!(
            icon(data.get_icon()).style(data.get_color()),
            text(format!("{}%", data.capacity)).style(data.get_color())
        )
        .spacing(4);
        match data.status {
            BatteryStatus::Charging(remaining) if data.capacity < 95 => row!(
                battery_info,
                text(format!("Full in {}", format_duration(&remaining)))
            )
            .spacing(16),
            BatteryStatus::Discharging(remaining) if data.capacity < 95 => row!(
                battery_info,
                text(format!("Empty in {}", format_duration(&remaining)))
            )
            .spacing(16),
            _ => row!(battery_info),
        }
    })
    .padding([8, 12])
    .style(move |_: &Theme| iced::widget::container::Appearance {
        background: iced::Background::Color(SURFACE_0).into(),
        border: Border::with_radius(32),
        ..iced::widget::container::Appearance::default()
    })
}
