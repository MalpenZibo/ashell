use super::SubMenu;
use crate::{
    components::icons::{StaticIcon, icon, icon_button},
    config::SettingsFormat,
    services::{
        ReadOnlyService, Service, ServiceEvent,
        audio::{AudioCommand, AudioService, DeviceType, Sinks},
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
    DefaultSinkChanged(String, String),
    DefaultSourceChanged(String, String),
    ToggleSinkMute,
    SinkVolumeChanged(i32),
    ToggleSourceMute,
    SourceVolumeChanged(i32),
    SinksMore(Id),
    SourcesMore(Id),
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
}

impl AudioSettingsConfig {
    pub fn new(
        sinks_more_cmd: Option<String>,
        sources_more_cmd: Option<String>,
        indicator_format: SettingsFormat,
    ) -> Self {
        Self {
            sinks_more_cmd,
            sources_more_cmd,
            indicator_format,
        }
    }
}

pub struct AudioSettings {
    config: AudioSettingsConfig,
    service: Option<AudioService>,
}

pub struct SubmenuEntry<RMessage> {
    pub name: String,
    pub device: DeviceType,
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

                        if service.sinks.len() < 2 {
                            return Action::CloseSubMenu;
                        }

                        if service.sources.len() < 2 {
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

    pub fn sink_indicator(&'_ self) -> Option<Element<'_, Message>> {
        self.service
            .as_ref()
            .filter(|service| !service.sinks.is_empty())
            .map(|service| {
                let icon_type = service.sinks.get_icon(&service.server_info.default_sink);
                let volume = service.cur_sink_volume;

                let scroll_handler = move |delta| {
                    let cur_vol = volume;
                    let delta = match delta {
                        iced::mouse::ScrollDelta::Lines { y, .. } => y,
                        iced::mouse::ScrollDelta::Pixels { y, .. } => y,
                    };
                    let new_volume = if delta > 0.0 {
                        (cur_vol + 5).min(100)
                    } else {
                        (cur_vol - 5).max(0)
                    };
                    Message::SinkVolumeChanged(new_volume)
                };

                match self.config.indicator_format {
                    SettingsFormat::Icon => {
                        let icon = icon(icon_type);
                        MouseArea::new(icon).on_scroll(scroll_handler).into()
                    }
                    SettingsFormat::Percentage => MouseArea::new(text(format!("{}%", volume)))
                        .on_scroll(scroll_handler)
                        .into(),
                    SettingsFormat::IconAndPercentage => {
                        let icon = icon(icon_type);
                        MouseArea::new(
                            row!(icon, text(format!("{}%", volume)))
                                .spacing(4)
                                .align_y(Alignment::Center),
                        )
                        .on_scroll(scroll_handler)
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
            let active_sink = service
                .sinks
                .iter()
                .find(|sink| sink.name == service.server_info.default_sink);

            let sink_slider = active_sink.map(|s| {
                Self::slider(
                    theme,
                    SliderType::Sink,
                    s.is_mute,
                    Message::ToggleSinkMute,
                    service.cur_sink_volume,
                    &Message::SinkVolumeChanged,
                    if service.sinks.iter().map(|s| s.ports.len()).sum::<usize>() > 1 {
                        Some((sub_menu, Message::ToggleSinksMenu))
                    } else {
                        None
                    },
                )
            });

            if service.sources.iter().any(|source| source.in_use) {
                let active_source = service
                    .sources
                    .iter()
                    .find(|source| source.name == service.server_info.default_source);

                let source_slider = active_source.map(|s| {
                    Self::slider(
                        theme,
                        SliderType::Source,
                        s.is_mute,
                        Message::ToggleSourceMute,
                        service.cur_source_volume,
                        &Message::SourceVolumeChanged,
                        if service.sources.iter().map(|s| s.ports.len()).sum::<usize>() > 1 {
                            Some((sub_menu, Message::ToggleSourcesMenu))
                        } else {
                            None
                        },
                    )
                });

                (sink_slider, source_slider)
            } else {
                (sink_slider, None)
            }
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
                    .sinks
                    .iter()
                    .flat_map(|s| {
                        s.ports.iter().map(|p| SubmenuEntry {
                            name: format!("{}: {}", p.description, s.description),
                            device: p.device_type,
                            active: p.active && s.name == service.server_info.default_sink,
                            msg: Message::DefaultSinkChanged(s.name.clone(), p.name.clone()),
                        })
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
                    .sources
                    .iter()
                    .flat_map(|s| {
                        s.ports.iter().map(|p| SubmenuEntry {
                            name: format!("{}: {}", p.description, s.description),
                            device: p.device_type,
                            active: p.active && s.name == service.server_info.default_source,
                            msg: Message::DefaultSourceChanged(s.name.clone(), p.name.clone()),
                        })
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
                            row!(icon(e.device.get_icon()), text(e.name))
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
                            row!(icon(e.device.get_icon()), text(e.name))
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
