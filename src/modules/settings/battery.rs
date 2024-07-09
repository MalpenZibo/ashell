use crate::{
    components::icons::icon,
    utils::{
        battery::{BatteryData, BatteryStatus},
        format_duration, IndicatorState,
    },
};
use iced::{
    widget::{container, row, text, Container}, Alignment, Background, Border, Element, Theme
};

pub fn battery_indicator<'a, Message: 'static>(data: BatteryData) -> Element<'a, Message> {
    let icon_type = data.get_icon();
    let state = data.get_indicator_state();

    container(
        row!(icon(icon_type), text(format!("{}%", data.capacity)))
            .spacing(4)
            .align_items(Alignment::Center),
    )
    .style(move |theme: &Theme| container::Appearance {
        text_color: Some(match state {
            IndicatorState::Success => theme.palette().success,
            IndicatorState::Danger => theme.palette().danger,
            _ => theme.palette().text,
        }),
        ..Default::default()
    })
    .into()
}

pub fn settings_battery_indicator<'a, Message: 'static>(
    data: BatteryData,
) -> Container<'a, Message> {
    let state = data.get_indicator_state();

    container({
        let battery_info =
            container(row!(icon(data.get_icon()), text(format!("{}%", data.capacity))).spacing(4))
                .style(move |theme: &Theme| container::Appearance {
                    text_color: Some(match state {
                        IndicatorState::Success => theme.palette().success,
                        IndicatorState::Danger => theme.palette().danger,
                        _ => theme.palette().text,
                    }),
                    ..Default::default()
                });
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
    .style(move |theme: &Theme| container::Appearance {
        background: Background::Color(theme.extended_palette().background.weak.color).into(),
        border: Border::with_radius(32),
        ..container::Appearance::default()
    })
}
