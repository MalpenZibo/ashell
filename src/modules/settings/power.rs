use std::convert;

use crate::{
    components::icons::{StaticIcon, icon},
    config::{BatteryFormat, PeripheralIndicators},
    modules::settings::quick_setting_button,
    services::{
        ReadOnlyService, Service, ServiceEvent,
        upower::{BatteryStatus, PowerProfile, PowerProfileCommand, UPowerService},
    },
    theme::AshellTheme,
    utils::{self, IndicatorState, format_duration},
};
use iced::{
    Alignment, Element, Length, Subscription, Task, Theme,
    widget::{Row, button, column, container, horizontal_rule, row, text},
};

#[derive(Debug, Clone)]
pub enum Message {
    Event(ServiceEvent<UPowerService>),
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
    Command(Task<Message>),
}

#[derive(Debug, Clone)]
pub struct PowerSettingsConfig {
    pub suspend_cmd: String,
    pub hibernate_cmd: String,
    pub reboot_cmd: String,
    pub shutdown_cmd: String,
    pub logout_cmd: String,
    pub battery_format: BatteryFormat,
    pub peripheral_indicators: PeripheralIndicators,
    pub peripheral_battery_format: BatteryFormat,
}

impl PowerSettingsConfig {
    pub fn new(
        suspend_cmd: String,
        hibernate_cmd: String,
        reboot_cmd: String,
        shutdown_cmd: String,
        logout_cmd: String,
        battery_format: BatteryFormat,
        peripheral_indicators: PeripheralIndicators,
        peripheral_battery_format: BatteryFormat,
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

    pub fn peripheral_indicators<'a>(
        &self,
        ashell_theme: &AshellTheme,
    ) -> Option<Element<'a, Message>> {
        // if self.config.peripheral_indicators {
        self.service.as_ref().map(|s| {
            Row::with_children(s.peripherals.iter().map(|p| {
                let state = p.data.get_indicator_state();

                container(match self.config.peripheral_battery_format {
                    BatteryFormat::Icon => {
                        convert::Into::<Element<'a, Message>>::into(icon(p.get_icon_state()))
                    }
                    BatteryFormat::Percentage => row!(
                        icon(p.kind.get_icon()),
                        text(format!("{}%", p.data.capacity))
                    )
                    .spacing(ashell_theme.space.xxs)
                    .align_y(Alignment::Center)
                    .into(),
                    BatteryFormat::IconAndPercentage => row!(
                        icon(p.get_icon_state()),
                        text(format!("{}%", p.data.capacity))
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
            }))
            .into()
        })
        // } else {
        //     None
        // }
    }

    pub fn battery_indicator<'a>(
        &self,
        ashell_theme: &AshellTheme,
    ) -> Option<Element<'a, Message>> {
        self.service.as_ref().and_then(|service| {
            service.system_battery.map(|battery| {
                let state = battery.get_indicator_state();

                container(match self.config.battery_format {
                    BatteryFormat::Icon => icon(battery.get_icon()).into(),
                    BatteryFormat::Percentage => convert::Into::<Element<'_, _, _>>::into(text(
                        format!("{}%", battery.capacity),
                    )),
                    BatteryFormat::IconAndPercentage => row!(
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

    pub fn battery_menu_indicator<'a>(
        &self,
        ashell_theme: &AshellTheme,
    ) -> Option<Element<'a, Message>> {
        self.service.as_ref().and_then(|service| {
            service.system_battery.map(|battery| {
                let state = battery.get_indicator_state();

                container({
                    let battery_info = container(
                        row!(
                            icon(battery.get_icon()),
                            text(format!("{}%", battery.capacity))
                        )
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
                        BatteryStatus::Discharging(remaining) if battery.capacity < 95 => row!(
                            battery_info,
                            text(format!("Empty in {}", format_duration(&remaining)))
                        )
                        .spacing(ashell_theme.space.md),
                        _ => row!(battery_info),
                    }
                })
                .padding([ashell_theme.space.xs, ashell_theme.space.xxs])
                .into()
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
