use std::convert;

use crate::{
    components::icons::{StaticIcon, icon},
    config::{PeripheralIndicators, SettingsFormat},
    modules::settings::quick_setting_button,
    services::{
        ReadOnlyService, Service, ServiceEvent,
        upower::{
            BatteryData, BatteryStatus, PeripheralDeviceKind, PowerProfile, PowerProfileCommand,
            UPowerService,
        },
    },
    theme::AshellTheme,
    utils::{self, IndicatorState, format_duration},
};
use iced::{
    Alignment, Element, Length, Subscription, Task, Theme,
    alignment::Vertical,
    widget::{Column, Row, button, column, container, horizontal_rule, row, text},
};

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
    pub peripheral_indicators: PeripheralIndicators,
    pub peripheral_battery_format: SettingsFormat,
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
        peripheral_indicators: PeripheralIndicators,
        peripheral_battery_format: SettingsFormat,
    ) -> Self {
        Self {
            suspend_cmd,
            hibernate_cmd,
            reboot_cmd,
            shutdown_cmd,
            logout_cmd,
            battery_format,
            peripheral_indicators,
            peripheral_battery_format,
        }
    }
}

pub struct PowerSettings {
    config: PowerSettingsConfig,
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

    pub fn menu<'a>(&'a self, theme: &'a AshellTheme) -> Element<'a, Message> {
        column!(
            button(row!(icon(StaticIcon::Suspend), text("Suspend")).spacing(theme.space.md))
                .padding([theme.space.xxs, theme.space.sm])
                .on_press(Message::Suspend)
                .width(Length::Fill)
                .style(theme.ghost_button_style()),
            button(row!(icon(StaticIcon::Hibernate), text("Hibernate")).spacing(theme.space.md))
                .padding([theme.space.xxs, theme.space.sm])
                .on_press(Message::Hibernate)
                .width(Length::Fill)
                .style(theme.ghost_button_style()),
            button(row!(icon(StaticIcon::Reboot), text("Reboot")).spacing(theme.space.md))
                .padding([theme.space.xxs, theme.space.sm])
                .on_press(Message::Reboot)
                .width(Length::Fill)
                .style(theme.ghost_button_style()),
            button(row!(icon(StaticIcon::Power), text("Shutdown")).spacing(theme.space.md))
                .padding([theme.space.xxs, theme.space.sm])
                .on_press(Message::Shutdown)
                .width(Length::Fill)
                .style(theme.ghost_button_style()),
            horizontal_rule(1),
            button(row!(icon(StaticIcon::Logout), text("Logout")).spacing(theme.space.md))
                .padding([theme.space.xxs, theme.space.sm])
                .on_press(Message::Logout)
                .width(Length::Fill)
                .style(theme.ghost_button_style()),
        )
        .padding(theme.space.xs)
        .width(Length::Fill)
        .spacing(theme.space.xs)
        .into()
    }

    pub fn peripheral_menu<'a>(&'a self, theme: &'a AshellTheme) -> Option<Element<'a, Message>> {
        self.service
            .as_ref()
            .filter(|s| !s.peripherals.is_empty())
            .map(|service| {
                Column::with_children(
                    service
                        .peripherals
                        .iter()
                        .map(|p| {
                            Row::new()
                                .push(icon(p.kind.get_icon()))
                                .push(text(p.name.to_string()).width(Length::Fill))
                                .push(self.menu_indicator(theme, p.data, None))
                                .align_y(Vertical::Center)
                                .spacing(theme.space.sm)
                                .into()
                        })
                        .collect::<Vec<Element<Message>>>(),
                )
                .spacing(theme.space.xs)
                .into()
            })
    }

    pub fn peripheral_indicators<'a>(
        &self,
        ashell_theme: &AshellTheme,
    ) -> Option<Element<'a, Message>> {
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
                    let mut row = Row::new()
                        .spacing(ashell_theme.space.xxs)
                        .align_y(Alignment::Center);

                    for p in service.peripherals.iter() {
                        row = row.push_maybe({
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
                                        .spacing(ashell_theme.space.xxs)
                                        .align_y(Alignment::Center)
                                        .into(),
                                        SettingsFormat::IconAndPercentage => row!(
                                            icon(p.get_icon_state()),
                                            text(format!("{}%", p.data.capacity))
                                        )
                                        .spacing(ashell_theme.space.xxs)
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

    pub fn battery_indicator<'a>(
        &self,
        ashell_theme: &AshellTheme,
    ) -> Option<Element<'a, Message>> {
        self.service.as_ref().and_then(|service| {
            service.system_battery.map(|battery| {
                let state = battery.get_indicator_state();

                container(match self.config.battery_format {
                    SettingsFormat::Icon => icon(battery.get_icon()).into(),
                    SettingsFormat::Percentage => convert::Into::<Element<'a, Message>>::into(
                        text(format!("{}%", battery.capacity)),
                    ),
                    SettingsFormat::IconAndPercentage => row!(
                        icon(battery.get_icon()),
                        text(format!("{}%", battery.capacity))
                    )
                    .spacing(ashell_theme.space.xxs)
                    .align_y(Alignment::Center)
                    .into(),
                })
                .style(move |theme: &Theme| container::Style {
                    text_color: Some(match state {
                        IndicatorState::Success => theme.palette().success,
                        IndicatorState::Danger => theme.palette().danger,
                        _ => theme.palette().text,
                    }),
                    ..Default::default()
                })
                .into()
            })
        })
    }

    fn menu_indicator<'a>(
        &self,
        ashell_theme: &'a AshellTheme,
        battery: BatteryData,
        peripheral_icon: Option<StaticIcon>,
    ) -> Element<'a, Message> {
        let state = battery.get_indicator_state();

        container({
            let battery_info = container(
                Row::new()
                    .push_maybe(peripheral_icon.map(icon))
                    .push(icon(battery.get_icon()))
                    .push(text(format!("{}%", battery.capacity)))
                    .spacing(ashell_theme.space.xxs),
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
                .spacing(ashell_theme.space.md),
                BatteryStatus::Discharging(remaining)
                    if battery.capacity < 95 && !remaining.is_zero() =>
                {
                    row!(
                        battery_info,
                        text(format!("Empty in {}", format_duration(&remaining)))
                    )
                    .spacing(ashell_theme.space.md)
                }
                _ => row!(battery_info),
            }
        })
        .padding([ashell_theme.space.xs, ashell_theme.space.xxs])
        .into()
    }

    pub fn battery_menu_indicator<'a>(
        &self,
        ashell_theme: &'a AshellTheme,
    ) -> Option<Element<'a, Message>> {
        self.service.as_ref().and_then(|service| {
            service
                .system_battery
                .map(|battery| {
                    let indicator = self.menu_indicator(ashell_theme, battery, None);

                    if !service.peripherals.is_empty() {
                        button(indicator)
                            .padding([0, ashell_theme.space.sm])
                            .on_press(Message::TogglePeripheralMenu)
                            .style(ashell_theme.settings_button_style())
                            .into()
                    } else {
                        indicator
                    }
                })
                .or_else(|| {
                    if let Some(peripheral) = service.peripherals.first() {
                        let indicator = self.menu_indicator(
                            ashell_theme,
                            peripheral.data,
                            Some(peripheral.kind.get_icon()),
                        );

                        Some(if service.peripherals.len() > 1 {
                            button(indicator)
                                .padding([0, ashell_theme.space.sm])
                                .on_press(Message::TogglePeripheralMenu)
                                .style(ashell_theme.settings_button_style())
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
        theme: &'a AshellTheme,
    ) -> Option<(Element<'a, Message>, Option<Element<'a, Message>>)> {
        self.service.as_ref().and_then(|service| {
            if !matches!(service.power_profile, PowerProfile::Unknown) {
                Some((
                    quick_setting_button(
                        theme,
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
