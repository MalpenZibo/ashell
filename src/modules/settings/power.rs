use std::convert;

use crate::{
    components::{
        ButtonKind, IconPosition, divider, format_indicator,
        icons::{StaticIcon, icon},
        quick_setting_button, styled_button,
    },
    config::{PeripheralIndicators, SettingsFormat},
    services::{
        ReadOnlyService, Service, ServiceEvent,
        upower::{
            BatteryData, BatteryStatus, PeripheralDeviceKind, PowerProfile, PowerProfileCommand,
            UPowerService,
        },
    },
    theme::use_theme,
    utils::{self, IndicatorState, format_duration},
};
use iced::{
    Alignment, Element, Length, Subscription, Task, Theme,
    alignment::Vertical,
    widget::{Column, Row, column, container, row, text},
};

fn format_time_for_battery(battery: &BatteryData) -> String {
    match battery.status {
        BatteryStatus::Charging(duration) => {
            if battery.capacity >= 100 || duration.is_zero() {
                "100%".to_string()
            } else {
                format_duration(&duration)
            }
        }
        BatteryStatus::Discharging(duration) => {
            if battery.capacity >= 100 {
                "100%".to_string()
            } else if duration.is_zero() {
                "Calculating...".to_string()
            } else {
                format_duration(&duration)
            }
        }
        BatteryStatus::Full => "100%".to_string(),
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    Event(ServiceEvent<UPowerService>),
    TogglePeripheralMenu,
    TogglePowerProfile,
    Suspend,
    Hibernate,
    Reboot,
    Shutdown,
    Logout,
    ConfigReloaded(PowerSettingsConfig),
}

pub enum Action {
    None,
    TogglePeripheralMenu,
    Command(Task<Message>),
}

#[derive(Debug, Clone)]
pub struct PowerSettingsConfig {
    pub suspend_cmd: String,
    pub hibernate_cmd: String,
    pub reboot_cmd: String,
    pub shutdown_cmd: String,
    pub logout_cmd: String,
    pub battery_format: SettingsFormat,
    pub battery_hide_when_full: bool,
    pub peripheral_indicators: PeripheralIndicators,
    pub peripheral_battery_format: SettingsFormat,
    pub peripheral_expanded_by_default: bool,
}

impl PowerSettingsConfig {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        suspend_cmd: String,
        hibernate_cmd: String,
        reboot_cmd: String,
        shutdown_cmd: String,
        logout_cmd: String,
        battery_format: SettingsFormat,
        battery_hide_when_full: bool,
        peripheral_indicators: PeripheralIndicators,
        peripheral_battery_format: SettingsFormat,
        peripheral_expanded_by_default: bool,
    ) -> Self {
        Self {
            suspend_cmd,
            hibernate_cmd,
            reboot_cmd,
            shutdown_cmd,
            logout_cmd,
            battery_format,
            battery_hide_when_full,
            peripheral_indicators,
            peripheral_battery_format,
            peripheral_expanded_by_default,
        }
    }
}

pub struct PowerSettings {
    pub config: PowerSettingsConfig,
    service: Option<UPowerService>,
}

impl PowerSettings {
    pub fn new(config: PowerSettingsConfig) -> Self {
        Self {
            config,
            service: None,
        }
    }

    pub fn update(&mut self, message: Message) -> Action {
        match message {
            Message::Event(event) => match event {
                ServiceEvent::Init(service) => {
                    self.service = Some(service);
                    Action::None
                }
                ServiceEvent::Update(data) => {
                    if let Some(service) = self.service.as_mut() {
                        service.update(data);
                    }
                    Action::None
                }
                ServiceEvent::Error(_) => Action::None,
            },
            Message::TogglePeripheralMenu => Action::TogglePeripheralMenu,
            Message::TogglePowerProfile => match self.service.as_mut() {
                Some(service) => Action::Command(
                    service
                        .command(PowerProfileCommand::Toggle)
                        .map(Message::Event),
                ),
                _ => Action::None,
            },
            Message::Suspend => {
                utils::launcher::suspend(self.config.suspend_cmd.clone());
                Action::None
            }
            Message::Hibernate => {
                utils::launcher::hibernate(self.config.hibernate_cmd.clone());
                Action::None
            }
            Message::Reboot => {
                utils::launcher::reboot(self.config.reboot_cmd.clone());
                Action::None
            }
            Message::Shutdown => {
                utils::launcher::shutdown(self.config.shutdown_cmd.clone());
                Action::None
            }
            Message::Logout => {
                utils::launcher::logout(self.config.logout_cmd.clone());
                Action::None
            }
            Message::ConfigReloaded(config) => {
                self.config = config;
                Action::None
            }
        }
    }

    pub fn menu<'a>(&'a self) -> Element<'a, Message> {
        let space = use_theme(|t| t.space);
        column!(
            styled_button("Suspend")
                .icon(StaticIcon::Suspend, IconPosition::Before)
                .on_press(Message::Suspend)
                .width(Length::Fill),
            styled_button("Hibernate")
                .icon(StaticIcon::Hibernate, IconPosition::Before)
                .on_press(Message::Hibernate)
                .width(Length::Fill),
            styled_button("Reboot")
                .icon(StaticIcon::Reboot, IconPosition::Before)
                .on_press(Message::Reboot)
                .width(Length::Fill),
            styled_button("Shutdown")
                .icon(StaticIcon::Power, IconPosition::Before)
                .on_press(Message::Shutdown)
                .width(Length::Fill),
            divider(),
            styled_button("Logout")
                .icon(StaticIcon::Logout, IconPosition::Before)
                .on_press(Message::Logout)
                .width(Length::Fill),
        )
        .padding(space.xs)
        .width(Length::Fill)
        .spacing(space.xs)
        .into()
    }

    pub fn peripheral_menu<'a>(&'a self) -> Option<Element<'a, Message>> {
        let space = use_theme(|t| t.space);
        self.service
            .as_ref()
            .filter(|s| !s.peripherals.is_empty())
            .map(|service| {
                Column::with_children(
                    service
                        .peripherals
                        .iter()
                        .map(|p| {
                            row![
                                icon(p.kind.get_icon()),
                                text(p.name.to_string()).width(Length::Fill),
                                self.menu_indicator(p.data, None),
                            ]
                            .align_y(Vertical::Center)
                            .spacing(space.sm)
                            .into()
                        })
                        .collect::<Vec<Element<Message>>>(),
                )
                .spacing(space.xs)
                .into()
            })
    }

    pub fn peripheral_indicators<'a>(&self) -> Option<Element<'a, Message>> {
        let space = use_theme(|t| t.space);
        let get_indicators = |kinds: Option<&[PeripheralDeviceKind]>| {
            self.service
                .as_ref()
                .filter(|p| {
                    !p.peripherals.is_empty()
                        && kinds.is_none_or(|kinds| {
                            p.peripherals.iter().any(|p| kinds.contains(&p.kind))
                        })
                })
                .map(|service| {
                    let mut row = Row::with_capacity(service.peripherals.len())
                        .spacing(space.xxs)
                        .align_y(Alignment::Center);

                    for p in service.peripherals.iter() {
                        row = row.push({
                            if kinds.as_ref().is_none_or(|kinds| kinds.contains(&p.kind)) {
                                let state = p.data.get_indicator_state();

                                Some(
                                    container(match self.config.peripheral_battery_format {
                                        SettingsFormat::Icon => {
                                            convert::Into::<Element<'a, Message>>::into(icon(
                                                p.get_icon_state(),
                                            ))
                                        }
                                        SettingsFormat::Percentage => row!(
                                            icon(p.kind.get_icon()),
                                            text(format!("{}%", p.data.capacity))
                                        )
                                        .spacing(space.xxs)
                                        .align_y(Alignment::Center)
                                        .into(),
                                        SettingsFormat::IconAndPercentage => row!(
                                            icon(p.get_icon_state()),
                                            text(format!("{}%", p.data.capacity))
                                        )
                                        .spacing(space.xxs)
                                        .align_y(Alignment::Center)
                                        .into(),
                                        SettingsFormat::Time => {
                                            text(format_time_for_battery(&p.data)).into()
                                        }
                                        SettingsFormat::IconAndTime => row!(
                                            icon(p.get_icon_state()),
                                            text(format_time_for_battery(&p.data))
                                        )
                                        .spacing(space.xxs)
                                        .align_y(Alignment::Center)
                                        .into(),
                                    })
                                    .style(
                                        move |theme: &Theme| container::Style {
                                            text_color: Some(match state {
                                                IndicatorState::Success => theme.palette().success,
                                                IndicatorState::Danger => theme.palette().danger,
                                                _ => theme.palette().text,
                                            }),
                                            ..Default::default()
                                        },
                                    ),
                                )
                            } else {
                                None
                            }
                        });
                    }

                    row
                })
        };

        match &self.config.peripheral_indicators {
            PeripheralIndicators::All => get_indicators(None),
            PeripheralIndicators::Specific(kinds) => get_indicators(Some(kinds)),
        }
        .map(|r| r.into())
    }

    pub fn battery_indicator<'a>(&self) -> Option<Element<'a, Message>> {
        self.service.as_ref().and_then(|service| {
            service.system_battery.and_then(|battery| {
                if self.config.battery_hide_when_full
                    && matches!(battery.status, BatteryStatus::Full)
                {
                    return None;
                }
                let state = battery.get_indicator_state();
                let label: String = match self.config.battery_format {
                    SettingsFormat::Time | SettingsFormat::IconAndTime => {
                        format_time_for_battery(&battery)
                    }
                    _ => format!("{}%", battery.capacity),
                };

                Some(
                    format_indicator(
                        self.config.battery_format,
                        battery.get_icon(),
                        text(label).into(),
                        state,
                    )
                    .into(),
                )
            })
        })
    }

    fn menu_indicator<'a>(
        &self,
        battery: BatteryData,
        peripheral_icon: Option<StaticIcon>,
    ) -> Element<'a, Message> {
        let space = use_theme(|t| t.space);
        let state = battery.get_indicator_state();

        container({
            let battery_info = container(
                Row::with_capacity(3)
                    .push(peripheral_icon.map(icon))
                    .push(icon(battery.get_icon()))
                    .push(text(format!("{}%", battery.capacity)))
                    .spacing(space.xxs),
            )
            .style(move |theme: &Theme| container::Style {
                text_color: Some(match state {
                    IndicatorState::Success => theme.palette().success,
                    IndicatorState::Danger => theme.palette().danger,
                    _ => theme.palette().text,
                }),
                ..Default::default()
            });

            match battery.status {
                BatteryStatus::Charging(remaining) if battery.capacity < 95 => row!(
                    battery_info,
                    text(format!("Full in {}", format_duration(&remaining)))
                )
                .spacing(space.md),
                BatteryStatus::Discharging(remaining)
                    if battery.capacity < 95 && !remaining.is_zero() =>
                {
                    row!(
                        battery_info,
                        text(format!("Empty in {}", format_duration(&remaining)))
                    )
                    .spacing(space.md)
                }
                _ => row!(battery_info),
            }
        })
        .padding([space.xs, space.xxs])
        .into()
    }

    pub fn battery_menu_indicator<'a>(&self) -> Option<Element<'a, Message>> {
        self.service.as_ref().and_then(|service| {
            service
                .system_battery
                .map(|battery| {
                    let indicator = self.menu_indicator(battery, None);

                    if !service.peripherals.is_empty() {
                        styled_button(indicator)
                            .kind(ButtonKind::Solid)
                            .on_press(Message::TogglePeripheralMenu)
                            .into()
                    } else {
                        indicator
                    }
                })
                .or_else(|| {
                    if let Some(peripheral) = service.peripherals.first() {
                        let indicator =
                            self.menu_indicator(peripheral.data, Some(peripheral.kind.get_icon()));

                        Some(if service.peripherals.len() > 1 {
                            styled_button(indicator)
                                .kind(ButtonKind::Solid)
                                .on_press(Message::TogglePeripheralMenu)
                                .into()
                        } else {
                            indicator
                        })
                    } else {
                        None
                    }
                })
        })
    }

    pub fn power_profile_indicator<'a>(&'a self) -> Option<Element<'a, Message>> {
        self.service
            .as_ref()
            .and_then(|service| match service.power_profile {
                PowerProfile::Balanced => None,
                PowerProfile::Performance => Some(
                    container(icon(StaticIcon::Performance))
                        .style(|theme: &Theme| container::Style {
                            text_color: Some(theme.palette().danger),
                            ..Default::default()
                        })
                        .into(),
                ),
                PowerProfile::PowerSaver => Some(
                    container(icon(StaticIcon::PowerSaver))
                        .style(|theme: &Theme| container::Style {
                            text_color: Some(theme.palette().success),
                            ..Default::default()
                        })
                        .into(),
                ),
                PowerProfile::Unknown => None,
            })
    }

    pub fn quick_setting_button<'a>(
        &'a self,
    ) -> Option<(Element<'a, Message>, Option<Element<'a, Message>>)> {
        self.service.as_ref().and_then(|service| {
            if !matches!(service.power_profile, PowerProfile::Unknown) {
                Some((
                    quick_setting_button(
                        convert::Into::<StaticIcon>::into(service.power_profile),
                        match service.power_profile {
                            PowerProfile::Balanced => "Balanced",
                            PowerProfile::Performance => "Performance",
                            PowerProfile::PowerSaver => "Power Saver",
                            PowerProfile::Unknown => "",
                        }
                        .to_string(),
                        None,
                        true,
                        Message::TogglePowerProfile,
                        None,
                        None,
                    ),
                    None,
                ))
            } else {
                None
            }
        })
    }

    pub fn subscription(&self) -> Subscription<Message> {
        UPowerService::subscribe().map(Message::Event)
    }
}
