use super::SubMenu;
use crate::{
    components::{
        divider, format_indicator,
        icons::{StaticIcon, icon, icon_mono},
        selectable_list_item, slider_control,
    },
    config::SettingsFormat,
    services::{
        ReadOnlyService, Service, ServiceEvent,
        audio::{AudioCommand, AudioService, DevicePortType, Port},
    },
    theme::AshellTheme,
    utils::remote_value::{self, Remote},
};
use iced::{
    Element, Length, Subscription, SurfaceId, Task,
    mouse::ScrollDelta,
    widget::{Column, MouseArea, Text, button, column, text},
};
use libpulse_binding::volume::Volume;

const VOL_PERCENT: u32 = Volume::NORMAL.0 / 100;

#[derive(Debug, Clone)]
pub enum Message {
    Event(ServiceEvent<AudioService>),
    DefaultSinkChanged(String, Option<String>),
    DefaultSourceChanged(String, Option<String>),
    ToggleSinkMute,
    SinkVolumeChanged(remote_value::Message<u32>),
    ToggleSourceMute,
    SourceVolumeChanged(remote_value::Message<u32>),
    SinksMore(SurfaceId),
    SourcesMore(SurfaceId),
    OpenMore,
    OpenSourceMore,
    ToggleSinksMenu,
    ToggleSourcesMenu,
    ConfigReloaded(AudioSettingsConfig),
}

pub enum Action {
    None,
    Task(Task<Message>),
    ToggleSinksMenu,
    ToggleSourcesMenu,
    CloseMenu(SurfaceId),
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
            Message::SinkVolumeChanged(message) => {
                if let Some(service) = self.service.as_mut() {
                    if let Some(value) = message.value() {
                        let _ = service.command(AudioCommand::SinkVolume(value));
                    }
                    return Action::Task(
                        service
                            .sink_slider
                            .update(message)
                            .map(Message::SinkVolumeChanged),
                    );
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
            Message::SourceVolumeChanged(message) => {
                if let Some(service) = self.service.as_mut() {
                    if let Some(value) = message.value() {
                        let _ = service.command(AudioCommand::SourceVolume(value));
                    }
                    return Action::Task(
                        service
                            .source_slider
                            .update(message)
                            .map(Message::SourceVolumeChanged),
                    );
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
                            match service.sink_slider.value() {
                                v if v <= 33 * VOL_PERCENT => StaticIcon::Speaker1,
                                v if v <= 66 * VOL_PERCENT => StaticIcon::Speaker2,
                                _ => StaticIcon::Speaker3,
                            }
                        },
                    )
                })
            })
            .map(|(service, icon_type)| {
                let volume = service.sink_slider.value();
                MouseArea::new(format_indicator(
                    theme,
                    self.config.indicator_format,
                    icon(icon_type).into(),
                    Self::vol_text(volume).into(),
                ))
                .on_right_press(Message::OpenMore)
                .on_scroll(Self::on_scroll(volume, Message::SinkVolumeChanged))
                .into()
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
                let volume = service.source_slider.value();
                MouseArea::new(format_indicator(
                    theme,
                    self.config.microphone_indicator_format,
                    icon(icon_type).into(),
                    Self::vol_text(volume).into(),
                ))
                .on_right_press(Message::OpenSourceMore)
                .on_scroll(Self::on_scroll(volume, Message::SourceVolumeChanged))
                .into()
            })
    }

    pub fn sliders<'a>(
        &'a self,
        theme: &'a AshellTheme,
        sub_menu: Option<SubMenu>,
    ) -> (Option<Element<'a, Message>>, Option<Element<'a, Message>>) {
        if let Some(service) = &self.service {
            let sink_slider = service.active_sink().map(|s| {
                Self::audio_slider(
                    theme,
                    SliderType::Sink,
                    s.is_mute,
                    Message::ToggleSinkMute,
                    &service.sink_slider,
                    &Message::SinkVolumeChanged,
                    if service.has_multiple_sinks() {
                        Some((sub_menu, Message::ToggleSinksMenu))
                    } else {
                        None
                    },
                )
            });

            let source_slider = service.active_source().map(|s| {
                Self::audio_slider(
                    theme,
                    SliderType::Source,
                    s.is_mute,
                    Message::ToggleSourceMute,
                    &service.source_slider,
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
        id: SurfaceId,
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
                        active: route.device.name == service.server_info.default_sink,
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
        id: SurfaceId,
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
                        active: route.device.name == service.server_info.default_source,
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

    fn audio_slider<'a>(
        theme: &'a AshellTheme,
        slider_type: SliderType,
        is_mute: bool,
        toggle_mute: Message,
        volume: &'a Remote<u32>,
        volume_changed: &'a dyn Fn(remote_value::Message<u32>) -> Message,
        with_submenu: Option<(Option<SubMenu>, Message)>,
    ) -> Element<'a, Message> {
        let mute_icon = if is_mute {
            match slider_type {
                SliderType::Sink => StaticIcon::Speaker0,
                SliderType::Source => StaticIcon::Mic0,
            }
        } else {
            match slider_type {
                SliderType::Sink => StaticIcon::Speaker3,
                SliderType::Source => StaticIcon::Mic1,
            }
        };

        let mut ctrl = slider_control(
            theme,
            mute_icon,
            Volume::MUTED.0..=Volume::NORMAL.0,
            volume.value(),
            volume_changed,
            Self::on_scroll(volume.value(), volume_changed),
        )
        .on_icon_press(toggle_mute)
        .on_icon_right_press(match slider_type {
            SliderType::Sink => Message::OpenMore,
            SliderType::Source => Message::OpenSourceMore,
        });

        if let Some((submenu, msg)) = with_submenu {
            let expanded = match slider_type {
                SliderType::Sink => submenu == Some(SubMenu::Sinks),
                SliderType::Source => submenu == Some(SubMenu::Sources),
            };
            ctrl = ctrl.trailing_toggle(expanded, msg);
        }

        ctrl.into()
    }

    fn on_scroll<F>(cur_volume: u32, make_msg: F) -> impl Fn(ScrollDelta) -> Message
    where
        F: Fn(remote_value::Message<u32>) -> Message,
    {
        move |delta| {
            let y = match delta {
                ScrollDelta::Lines { y, .. } => y,
                ScrollDelta::Pixels { y, .. } => y,
            };
            let step = 5 * VOL_PERCENT;
            let new_volume = if y > 0.0 {
                (cur_volume + step).min(Volume::NORMAL.0)
            } else {
                cur_volume.saturating_sub(step)
            };
            make_msg(remote_value::Message::RequestAndTimeout(new_volume))
        }
    }

    fn vol_text<'a>(volume: u32) -> Text<'a> {
        text(format!("{}%", volume / VOL_PERCENT))
    }

    fn submenu<'a>(
        theme: &'a AshellTheme,
        entries: Vec<SubmenuEntry<Message>>,
        more_msg: Option<Message>,
    ) -> Element<'a, Message> {
        let entries: Element<'a, Message> = Column::with_children(
            entries
                .into_iter()
                .map(|e| {
                    selectable_list_item(theme, icon_mono(e.icon).into(), e.name, e.active, e.msg)
                })
                .collect::<Vec<_>>(),
        )
        .spacing(theme.space.xxs)
        .into();

        match more_msg {
            Some(more_msg) => column!(
                entries,
                divider(),
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
