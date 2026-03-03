use super::SubMenu;
use crate::{
    components::icons::{StaticIcon, icon, icon_button, icon_mono},
    config::SettingsFormat,
    services::{
        ReadOnlyService, Service, ServiceEvent,
        audio::{AudioCommand, AudioService, DevicePortType, Port},
    },
    theme::AshellTheme,
};
use iced::{
    Alignment, Element, Length, Subscription, Theme,
    widget::{
        Column, MouseArea, Row, button, column, container, horizontal_rule, row, slider, text,
    },
    window::Id,
};

#[derive(Debug, Clone)]
pub enum Message {
    Event(ServiceEvent<AudioService>),
    DefaultSinkChanged(String, Option<String>),
    DefaultSourceChanged(String, Option<String>),
    ToggleSinkMute,
    SinkVolumeChanged(i32),
    ToggleSourceMute,
    SourceVolumeChanged(i32),
    SinksMore(Id),
    SourcesMore(Id),
    OpenMore,
    OpenSourceMore,
    ToggleSinksMenu,
    ToggleSourcesMenu,
    ConfigReloaded(AudioSettingsConfig),
}

pub enum Action {
    None,
    ToggleSinksMenu,
    ToggleSourcesMenu,
    CloseMenu(Id),
    CloseSubMenu,
}

#[derive(Debug, Clone)]
pub struct AudioSettingsConfig {
    pub sinks_more_cmd: Option<String>,
    pub sources_more_cmd: Option<String>,
    pub indicator_format: SettingsFormat,
    pub microphone_indicator_format: SettingsFormat,
}

impl AudioSettingsConfig {
    pub fn new(
        sinks_more_cmd: Option<String>,
        sources_more_cmd: Option<String>,
        indicator_format: SettingsFormat,
        microphone_indicator_format: SettingsFormat,
    ) -> Self {
        Self {
            sinks_more_cmd,
            sources_more_cmd,
            indicator_format,
            microphone_indicator_format,
        }
    }
}

pub struct AudioSettings {
    config: AudioSettingsConfig,
    service: Option<AudioService>,
}

pub struct SubmenuEntry<RMessage> {
    pub name: String,
    pub icon: StaticIcon,
    pub active: bool,
    pub msg: RMessage,
}

#[derive(Debug, Clone, Copy)]
pub enum SliderType {
    Sink,
    Source,
}

impl AudioSettings {
    pub fn new(config: AudioSettingsConfig) -> Self {
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

                        if !service.has_multiple_sinks() {
                            return Action::CloseSubMenu;
                        }

                        if !service.has_multiple_sources() {
                            return Action::CloseSubMenu;
                        }
                    }
                    Action::None
                }
                ServiceEvent::Error(_) => Action::None,
            },
            Message::ToggleSinkMute => {
                if let Some(service) = self.service.as_mut() {
                    let _ = service.command(AudioCommand::ToggleSinkMute);
                }
                Action::None
            }
            Message::SinkVolumeChanged(value) => {
                if let Some(service) = self.service.as_mut() {
                    let _ = service.command(AudioCommand::SinkVolume(value));
                }
                Action::None
            }
            Message::DefaultSinkChanged(name, port) => {
                if let Some(service) = self.service.as_mut() {
                    let _ = service.command(AudioCommand::DefaultSink(name, port));
                }
                Action::None
            }
            Message::ToggleSourceMute => {
                if let Some(service) = self.service.as_mut() {
                    let _ = service.command(AudioCommand::ToggleSourceMute);
                }
                Action::None
            }
            Message::SourceVolumeChanged(value) => {
                if let Some(service) = self.service.as_mut() {
                    let _ = service.command(AudioCommand::SourceVolume(value));
                }
                Action::None
            }
            Message::DefaultSourceChanged(name, port) => {
                if let Some(service) = self.service.as_mut() {
                    let _ = service.command(AudioCommand::DefaultSource(name, port));
                }
                Action::None
            }
            Message::OpenMore => {
                if let Some(cmd) = &self.config.sinks_more_cmd {
                    crate::utils::launcher::execute_command(cmd.to_string());
                }
                Action::None
            }
            Message::OpenSourceMore => {
                if let Some(cmd) = &self.config.sources_more_cmd {
                    crate::utils::launcher::execute_command(cmd.to_string());
                }
                Action::None
            }
            Message::SinksMore(id) => {
                if let Some(cmd) = &self.config.sinks_more_cmd {
                    crate::utils::launcher::execute_command(cmd.to_string());
                    Action::CloseMenu(id)
                } else {
                    Action::None
                }
            }
            Message::SourcesMore(id) => {
                if let Some(cmd) = &self.config.sources_more_cmd {
                    crate::utils::launcher::execute_command(cmd.to_string());
                    Action::CloseMenu(id)
                } else {
                    Action::None
                }
            }
            Message::ToggleSinksMenu => Action::ToggleSinksMenu,
            Message::ToggleSourcesMenu => Action::ToggleSourcesMenu,
            Message::ConfigReloaded(config) => {
                self.config = config;
                Action::None
            }
        }
    }

    pub fn sink_indicator(&'_ self, theme: &'_ AshellTheme) -> Option<Element<'_, Message>> {
        self.service
            .as_ref()
            .and_then(|service| {
                service.active_sink().map(|sink| {
                    (
                        service,
                        if sink.is_mute {
                            StaticIcon::Speaker0
                        } else {
                            match service.cur_sink_volume {
                                0..=33 => StaticIcon::Speaker1,
                                34..=66 => StaticIcon::Speaker2,
                                _ => StaticIcon::Speaker3,
                            }
                        },
                    )
                })
            })
            .map(|(service, icon_type)| {
                let volume = service.cur_sink_volume;

                let make_scroll_handler = |cur_volume: i32| {
                    move |delta| {
                        let delta = match delta {
                            iced::mouse::ScrollDelta::Lines { y, .. } => y,
                            iced::mouse::ScrollDelta::Pixels { y, .. } => y,
                        };
                        let new_volume = if delta > 0.0 {
                            (cur_volume + 5).min(100)
                        } else {
                            (cur_volume - 5).max(0)
                        };
                        Message::SinkVolumeChanged(new_volume)
                    }
                };

                match self.config.indicator_format {
                    SettingsFormat::Icon => {
                        let icon = icon(icon_type);
                        MouseArea::new(icon)
                            .on_right_press(Message::OpenMore)
                            .on_scroll(make_scroll_handler(volume))
                            .into()
                    }
                    SettingsFormat::Percentage | SettingsFormat::Time => {
                        MouseArea::new(text(format!("{}%", volume)))
                            .on_right_press(Message::OpenMore)
                            .on_scroll(make_scroll_handler(volume))
                            .into()
                    }
                    SettingsFormat::IconAndPercentage | SettingsFormat::IconAndTime => {
                        let icon = icon(icon_type);
                        MouseArea::new(
                            row!(icon, text(format!("{}%", volume)))
                                .spacing(theme.space.xxs)
                                .align_y(Alignment::Center),
                        )
                        .on_right_press(Message::OpenMore)
                        .on_scroll(make_scroll_handler(volume))
                        .into()
                    }
                }
            })
    }

    pub fn source_indicator(&'_ self, theme: &'_ AshellTheme) -> Option<Element<'_, Message>> {
        self.service
            .as_ref()
            .and_then(|service| {
                service.active_source().map(|source| {
                    (
                        service,
                        if source.is_mute {
                            StaticIcon::Mic0
                        } else {
                            StaticIcon::Mic1
                        },
                    )
                })
            })
            .map(|(service, icon_type)| {
                let volume = service.cur_source_volume;

                let make_scroll_handler = |cur_volume: i32| {
                    move |delta| {
                        let delta = match delta {
                            iced::mouse::ScrollDelta::Lines { y, .. } => y,
                            iced::mouse::ScrollDelta::Pixels { y, .. } => y,
                        };
                        let new_volume = if delta > 0.0 {
                            (cur_volume + 5).min(100)
                        } else {
                            (cur_volume - 5).max(0)
                        };
                        Message::SourceVolumeChanged(new_volume)
                    }
                };

                match self.config.microphone_indicator_format {
                    SettingsFormat::Icon => {
                        let icon = icon(icon_type);
                        MouseArea::new(icon)
                            .on_right_press(Message::OpenSourceMore)
                            .on_scroll(make_scroll_handler(volume))
                            .into()
                    }
                    SettingsFormat::Percentage | SettingsFormat::Time => {
                        MouseArea::new(text(format!("{}%", volume)))
                            .on_right_press(Message::OpenSourceMore)
                            .on_scroll(make_scroll_handler(volume))
                            .into()
                    }
                    SettingsFormat::IconAndPercentage | SettingsFormat::IconAndTime => {
                        let icon = icon(icon_type);
                        MouseArea::new(
                            row!(icon, text(format!("{}%", volume)))
                                .spacing(theme.space.xxs)
                                .align_y(Alignment::Center),
                        )
                        .on_right_press(Message::OpenSourceMore)
                        .on_scroll(make_scroll_handler(volume))
                        .into()
                    }
                }
            })
    }

    pub fn sliders<'a>(
        &'a self,
        theme: &'a AshellTheme,
        sub_menu: Option<SubMenu>,
    ) -> (Option<Element<'a, Message>>, Option<Element<'a, Message>>) {
        if let Some(service) = &self.service {
            let sink_slider = service.active_sink().map(|s| {
                Self::slider(
                    theme,
                    SliderType::Sink,
                    s.is_mute,
                    Message::ToggleSinkMute,
                    service.cur_sink_volume,
                    &Message::SinkVolumeChanged,
                    if service.has_multiple_sinks() {
                        Some((sub_menu, Message::ToggleSinksMenu))
                    } else {
                        None
                    },
                )
            });

            let source_slider = service.active_source().map(|s| {
                Self::slider(
                    theme,
                    SliderType::Source,
                    s.is_mute,
                    Message::ToggleSourceMute,
                    service.cur_source_volume,
                    &Message::SourceVolumeChanged,
                    if service.has_multiple_sources() {
                        Some((sub_menu, Message::ToggleSourcesMenu))
                    } else {
                        None
                    },
                )
            });

            (sink_slider, source_slider)
        } else {
            (None, None)
        }
    }

    pub fn sinks_submenu<'a>(
        &'a self,
        id: Id,
        theme: &'a AshellTheme,
    ) -> Option<Element<'a, Message>> {
        self.service.as_ref().map(|service| {
            Self::submenu(
                theme,
                service
                    .sink_iter()
                    .map(|route| SubmenuEntry {
                        name: route.to_string(),
                        icon: route
                            .port
                            .and_then(Self::port_icon)
                            .unwrap_or(StaticIcon::Speaker3),
                        active: route.is_active()
                            && route.device.name == service.server_info.default_sink,
                        msg: Message::DefaultSinkChanged(
                            route.device.name.clone(),
                            route.port.map(|p| p.name.clone()),
                        ),
                    })
                    .collect(),
                if self.config.sinks_more_cmd.is_some() {
                    Some(Message::SinksMore(id))
                } else {
                    None
                },
            )
        })
    }

    pub fn sources_submenu<'a>(
        &'a self,
        id: Id,
        theme: &'a AshellTheme,
    ) -> Option<Element<'a, Message>> {
        self.service.as_ref().map(|service| {
            Self::submenu(
                theme,
                service
                    .source_iter()
                    .map(|route| SubmenuEntry {
                        name: route.to_string(),
                        icon: route
                            .port
                            .and_then(Self::port_icon)
                            .unwrap_or(StaticIcon::Mic1),
                        active: route.is_active()
                            && route.device.name == service.server_info.default_source,
                        msg: Message::DefaultSourceChanged(
                            route.device.name.clone(),
                            route.port.map(|p| p.name.clone()),
                        ),
                    })
                    .collect(),
                if self.config.sources_more_cmd.is_some() {
                    Some(Message::SourcesMore(id))
                } else {
                    None
                },
            )
        })
    }

    fn port_icon(port: &Port) -> Option<StaticIcon> {
        match port.device_type {
            DevicePortType::Unknown => None,
            DevicePortType::Aux => Some(StaticIcon::AudioJack),
            DevicePortType::Speaker => Some(StaticIcon::Speaker3),
            DevicePortType::Headphones => Some(StaticIcon::Headphones1),
            DevicePortType::Line => Some(StaticIcon::AudioJack),
            DevicePortType::Mic => Some(StaticIcon::Mic1),
            DevicePortType::Headset => Some(StaticIcon::Headset),
            DevicePortType::Handset => Some(StaticIcon::Phone),
            DevicePortType::Earpiece => Some(StaticIcon::Ear),
            DevicePortType::SPDIF => Some(StaticIcon::AudioRca),
            DevicePortType::HDMI => Some(StaticIcon::MonitorSpeaker),
            DevicePortType::TV => Some(StaticIcon::MonitorSpeaker),
            DevicePortType::Radio => Some(StaticIcon::Radio),
            DevicePortType::Video => Some(StaticIcon::MonitorSpeaker),
            DevicePortType::USB => Some(StaticIcon::Usb),
            DevicePortType::Bluetooth => Some(StaticIcon::SpeakerBluetooth),
            DevicePortType::Portable => None,
            DevicePortType::Handsfree => Some(StaticIcon::Ear),
            DevicePortType::Car => Some(StaticIcon::Car),
            DevicePortType::HiFi => Some(StaticIcon::AudioHiFi),
            DevicePortType::Phone => Some(StaticIcon::Phone),
            DevicePortType::Network => Some(StaticIcon::SpeakerWireless),
            DevicePortType::Analog => Some(StaticIcon::AudioRca),
        }
    }

    fn slider<'a>(
        theme: &'a AshellTheme,
        slider_type: SliderType,
        is_mute: bool,
        toggle_mute: Message,
        volume: i32,
        volume_changed: &'a dyn Fn(i32) -> Message,
        with_submenu: Option<(Option<SubMenu>, Message)>,
    ) -> Element<'a, Message> {
        Row::new()
            .push(
                MouseArea::new(
                    icon_button(
                        theme,
                        if is_mute {
                            match slider_type {
                                SliderType::Sink => StaticIcon::Speaker0,
                                SliderType::Source => StaticIcon::Mic0,
                            }
                        } else {
                            match slider_type {
                                SliderType::Sink => StaticIcon::Speaker3,
                                SliderType::Source => StaticIcon::Mic1,
                            }
                        },
                    )
                    .on_press(toggle_mute),
                )
                .on_right_press(Message::OpenMore),
            )
            .push(
                MouseArea::new(
                    slider(0..=100, volume, volume_changed)
                        .step(1)
                        .width(Length::Fill),
                )
                .on_scroll(move |delta| {
                    let delta = match delta {
                        iced::mouse::ScrollDelta::Lines { y, .. } => y,
                        iced::mouse::ScrollDelta::Pixels { y, .. } => y,
                    };
                    // volume is always changed by one less than expected
                    let new_volume = if delta > 0.0 {
                        (volume + 5 + 1).min(100)
                    } else {
                        (volume - 5 + 1).max(0)
                    };
                    volume_changed(new_volume)
                }),
            )
            .push_maybe(with_submenu.map(|(submenu, msg)| {
                icon_button(
                    theme,
                    match (slider_type, submenu) {
                        (SliderType::Sink, Some(SubMenu::Sinks))
                        | (SliderType::Source, Some(SubMenu::Sources)) => StaticIcon::Close,
                        _ => StaticIcon::RightArrow,
                    },
                )
                .on_press(msg)
            }))
            .align_y(Alignment::Center)
            .spacing(theme.space.xs)
            .into()
    }

    fn submenu<'a>(
        theme: &'a AshellTheme,
        entries: Vec<SubmenuEntry<Message>>,
        more_msg: Option<Message>,
    ) -> Element<'a, Message> {
        let entries = Column::with_children(
            entries
                .into_iter()
                .map(|e| {
                    if e.active {
                        container(
                            row!(icon_mono(e.icon), text(e.name))
                                .align_y(Alignment::Center)
                                .spacing(theme.space.md)
                                .padding([theme.space.xxs, theme.space.sm]),
                        )
                        .style(|theme: &Theme| container::Style {
                            text_color: Some(theme.palette().success),
                            ..Default::default()
                        })
                        .into()
                    } else {
                        button(
                            row!(icon_mono(e.icon), text(e.name))
                                .spacing(theme.space.md)
                                .align_y(Alignment::Center),
                        )
                        .on_press(e.msg)
                        .padding([theme.space.xxs, theme.space.sm])
                        .width(Length::Fill)
                        .style(theme.ghost_button_style())
                        .into()
                    }
                })
                .collect::<Vec<_>>(),
        )
        .spacing(theme.space.xxs)
        .into();

        match more_msg {
            Some(more_msg) => column!(
                entries,
                horizontal_rule(1),
                button("More")
                    .on_press(more_msg)
                    .padding([theme.space.xxs, theme.space.sm])
                    .width(Length::Fill)
                    .style(theme.ghost_button_style()),
            )
            .spacing(theme.space.sm)
            .into(),
            _ => entries,
        }
    }

    pub fn subscription(&self) -> Subscription<Message> {
        AudioService::subscribe().map(Message::Event)
    }
}
