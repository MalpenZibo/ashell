use crate::{
    components::icons::{Icons, icon},
    services::{
        ServiceEvent,
        upower::{BatteryData, BatteryStatus, PowerProfile, UPowerService},
    },
    utils::{IndicatorState, format_duration},
};
use iced::{
    Alignment, Element, Theme,
    widget::{Container, container, row, text},
};

use super::{Message, quick_setting_button};

#[derive(Clone, Debug)]
pub enum UPowerMessage {
    Event(ServiceEvent<UPowerService>),
    TogglePowerProfile,
}

impl BatteryData {
    pub fn indicator<'a, Message: 'static>(&self) -> Element<'a, Message> {
        let icon_type = self.get_icon();
        let state = self.get_indicator_state();

        container(
            row!(icon(icon_type), text(format!("{}%", self.capacity)))
                .spacing(4)
                .align_y(Alignment::Center),
        )
        .style(move |theme: &Theme| container::Style {
            text_color: Some(match state {
                IndicatorState::Success => theme.palette().success,
                IndicatorState::Danger => theme.palette().danger,
                _ => theme.palette().text,
            }),
            ..Default::default()
        })
        .into()
    }

    pub fn settings_indicator<'a, Message: 'static>(&self) -> Container<'a, Message> {
        let state = self.get_indicator_state();

        container({
            let battery_info = container(
                row!(icon(self.get_icon()), text(format!("{}%", self.capacity))).spacing(4),
            )
            .style(move |theme: &Theme| container::Style {
                text_color: Some(match state {
                    IndicatorState::Success => theme.palette().success,
                    IndicatorState::Danger => theme.palette().danger,
                    _ => theme.palette().text,
                }),
                ..Default::default()
            });

            match self.status {
                BatteryStatus::Charging(remaining) if self.capacity < 95 => row!(
                    battery_info,
                    text(format!("Full in {}", format_duration(&remaining)))
                )
                .spacing(16),
                BatteryStatus::Discharging(remaining) if self.capacity < 95 => row!(
                    battery_info,
                    text(format!("Empty in {}", format_duration(&remaining)))
                )
                .spacing(16),
                _ => row!(battery_info),
            }
        })
        .padding([8, 4])
    }
}

impl PowerProfile {
    pub fn indicator<Message: 'static>(&self) -> Option<Element<Message>> {
        match self {
            PowerProfile::Balanced => None,
            PowerProfile::Performance => Some(
                container(icon(Icons::Performance))
                    .style(|theme: &Theme| container::Style {
                        text_color: Some(theme.palette().danger),
                        ..Default::default()
                    })
                    .into(),
            ),
            PowerProfile::PowerSaver => Some(
                container(icon(Icons::PowerSaver))
                    .style(|theme: &Theme| container::Style {
                        text_color: Some(theme.palette().success),
                        ..Default::default()
                    })
                    .into(),
            ),
            PowerProfile::Unknown => None,
        }
    }

    pub fn get_quick_setting_button(
        &self,
        opacity: f32,
    ) -> Option<(Element<Message>, Option<Element<Message>>)> {
        if !matches!(self, PowerProfile::Unknown) {
            Some((
                quick_setting_button(
                    (*self).into(),
                    match self {
                        PowerProfile::Balanced => "Balanced",
                        PowerProfile::Performance => "Performance",
                        PowerProfile::PowerSaver => "Power Saver",
                        PowerProfile::Unknown => "",
                    }
                    .to_string(),
                    None,
                    true,
                    Message::UPower(UPowerMessage::TogglePowerProfile),
                    None,
                    opacity,
                ),
                None,
            ))
        } else {
            None
        }
    }
}
