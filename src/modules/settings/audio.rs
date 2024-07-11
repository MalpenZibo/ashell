use iced::{
    theme::{self, Button},
    widget::{button, column, container, horizontal_rule, row, slider, text, Column, Row},
    Alignment, Command, Element, Length, Subscription, Theme,
};

use crate::{
    components::icons::{icon, Icons},
    config::SettingsModuleConfig,
    menu::Menu,
    style::{GhostButtonStyle, SettingsButtonStyle, SliderStyle},
    utils::{
        audio::{AudioCommand, DeviceType, Sink, Sinks, Source, Volume},
        Commander,
    },
};

use super::{Message, SubMenu};

#[derive(Debug, Clone)]
pub enum AudioMessage {
    DefaultSinkSourceChanged(String, String),
    SinkChanges(Vec<Sink>),
    SourceChanges(Vec<Source>),
    SinkToggleMute,
    SinkVolumeChanged(i32),
    DefaultSinkChanged(String, String),
    SourceToggleMute,
    SourceVolumeChanged(i32),
    DefaultSourceChanged(String, String),
    SinksMore,
    SourcesMore,
}

pub struct Audio {
    audio_commander: Commander<AudioCommand>,
    sinks: Vec<Sink>,
    sources: Vec<Source>,
    default_sink: String,
    default_source: String,
    cur_sink_volume: i32,
    cur_source_volume: i32,
}

impl Audio {
    pub fn new() -> Self {
        Self {
            audio_commander: Commander::new(),
            sinks: Vec::new(),
            sources: Vec::new(),
            default_sink: String::new(),
            default_source: String::new(),
            cur_sink_volume: 0,
            cur_source_volume: 0,
        }
    }

    pub fn update(
        &mut self,
        message: AudioMessage,
        menu: &mut Menu<crate::app::Message>,
        config: &SettingsModuleConfig,
    ) -> Command<crate::app::Message> {
        match message {
            AudioMessage::SinkChanges(sinks) => {
                self.sinks = sinks;
                self.cur_sink_volume = (self
                    .sinks
                    .iter()
                    .find_map(|sink| {
                        if sink
                            .ports
                            .iter()
                            .any(|p| p.active && sink.name == self.default_sink)
                        {
                            Some(if sink.is_mute {
                                0.
                            } else {
                                sink.volume.get_volume()
                            })
                        } else {
                            None
                        }
                    })
                    .unwrap_or_default()
                    * 100.) as i32;

                Command::none()
            }
            AudioMessage::SourceChanges(sources) => {
                self.sources = sources;
                self.cur_source_volume = (self
                    .sources
                    .iter()
                    .find_map(|source| {
                        if source
                            .ports
                            .iter()
                            .any(|p| p.active && source.name == self.default_source)
                        {
                            Some(if source.is_mute {
                                0.
                            } else {
                                source.volume.get_volume()
                            })
                        } else {
                            None
                        }
                    })
                    .unwrap_or_default()
                    * 100.) as i32;

                Command::none()
            }
            AudioMessage::DefaultSinkSourceChanged(sink, source) => {
                self.default_sink = sink;
                self.default_source = source;

                self.cur_sink_volume = (self
                    .sinks
                    .iter()
                    .find_map(|sink| {
                        if sink
                            .ports
                            .iter()
                            .any(|p| p.active && sink.name == self.default_sink)
                        {
                            Some(if sink.is_mute {
                                0.
                            } else {
                                sink.volume.get_volume()
                            })
                        } else {
                            None
                        }
                    })
                    .unwrap_or_default()
                    * 100.) as i32;
                self.cur_source_volume = (self
                    .sources
                    .iter()
                    .find_map(|source| {
                        if source
                            .ports
                            .iter()
                            .any(|p| p.active && source.name == self.default_source)
                        {
                            Some(if source.is_mute {
                                0.
                            } else {
                                source.volume.get_volume()
                            })
                        } else {
                            None
                        }
                    })
                    .unwrap_or_default()
                    * 100.) as i32;

                Command::none()
            }
            AudioMessage::SinkToggleMute => {
                if let Some(sink) = self
                    .sinks
                    .iter()
                    .find(|sink| sink.name == self.default_sink)
                {
                    let _ = self.audio_commander.send(AudioCommand::SinkMute(
                        self.default_sink.clone(),
                        !sink.is_mute,
                    ));
                }

                Command::none()
            }
            AudioMessage::SinkVolumeChanged(volume) => {
                self.cur_sink_volume = volume;
                if let Some(sink) = self
                    .sinks
                    .iter_mut()
                    .find(|sink| sink.name == self.default_sink)
                {
                    if let Some(new_volume) =
                        sink.volume.scale_volume(self.cur_sink_volume as f64 / 100.)
                    {
                        let _ = self
                            .audio_commander
                            .send(AudioCommand::SinkVolume(sink.name.clone(), *new_volume));
                    }
                }

                Command::none()
            }
            AudioMessage::DefaultSinkChanged(name, port) => {
                self.default_sink.clone_from(&name);
                for sink in self.sinks.iter_mut() {
                    for cur_port in sink.ports.iter_mut() {
                        cur_port.active = sink.name == name && cur_port.name == port;
                    }
                }

                let _ = self
                    .audio_commander
                    .send(AudioCommand::DefaultSink(name, port));

                Command::none()
            }
            AudioMessage::SourceToggleMute => {
                if let Some(source) = self
                    .sources
                    .iter()
                    .find(|source| source.name == self.default_source)
                {
                    let _ = self.audio_commander.send(AudioCommand::SourceMute(
                        self.default_source.clone(),
                        !source.is_mute,
                    ));
                }

                Command::none()
            }
            AudioMessage::SourceVolumeChanged(volume) => {
                self.cur_source_volume = volume;
                if let Some(source) = self
                    .sources
                    .iter_mut()
                    .find(|source| source.name == self.default_source)
                {
                    if let Some(new_volume) = source
                        .volume
                        .scale_volume(self.cur_source_volume as f64 / 100.)
                    {
                        let _ = self
                            .audio_commander
                            .send(AudioCommand::SourceVolume(source.name.clone(), *new_volume));
                    }
                }

                Command::none()
            }
            AudioMessage::DefaultSourceChanged(name, port) => {
                self.default_source.clone_from(&name);
                for source in self.sources.iter_mut() {
                    for cur_port in source.ports.iter_mut() {
                        cur_port.active = source.name == name && cur_port.name == port;
                    }
                }

                let _ = self
                    .audio_commander
                    .send(AudioCommand::DefaultSource(name, port));

                Command::none()
            }
            AudioMessage::SinksMore => {
                if let Some(more_cmd) = &config.audio_sinks_more_cmd {
                    crate::utils::launcher::execute_command(more_cmd.to_string());
                    menu.close()
                } else {
                    Command::none()
                }
            }
            AudioMessage::SourcesMore => {
                if let Some(more_cmd) = &config.audio_sources_more_cmd {
                    crate::utils::launcher::execute_command(more_cmd.to_string());
                    menu.close()
                } else {
                    Command::none()
                }
            }
        }
    }

    pub fn sink_indicator<'a, Message>(&self) -> Option<Element<'a, Message>> {
        if !self.sinks.is_empty() {
            let icon_type = self.sinks.get_icon();

            Some(icon(icon_type).into())
        } else {
            None
        }
    }

    pub fn audio_sliders<'a>(
        &self,
        sub_menu: Option<SubMenu>,
    ) -> (Option<Element<'a, Message>>, Option<Element<'a, Message>>) {
        let active_sink = self
            .sinks
            .iter()
            .find(|sink| sink.ports.iter().any(|p| p.active));

        let sink_slider = active_sink.map(|s| {
            audio_slider(
                SliderType::Sink,
                s.is_mute,
                Message::Audio(AudioMessage::SinkToggleMute),
                self.cur_sink_volume,
                |v| Message::Audio(AudioMessage::SinkVolumeChanged(v)),
                if self.sinks.iter().map(|s| s.ports.len()).sum::<usize>() > 1 {
                    Some((sub_menu, Message::ToggleSubMenu(SubMenu::Sinks)))
                } else {
                    None
                },
            )
        });

        let active_source = self
            .sources
            .iter()
            .find(|source| source.ports.iter().any(|p| p.active));

        let source_slider = active_source.map(|s| {
            audio_slider(
                SliderType::Source,
                s.is_mute,
                Message::Audio(AudioMessage::SourceToggleMute),
                self.cur_source_volume,
                |v| Message::Audio(AudioMessage::SourceVolumeChanged(v)),
                if self.sources.iter().map(|s| s.ports.len()).sum::<usize>() > 1 {
                    Some((sub_menu, Message::ToggleSubMenu(SubMenu::Sources)))
                } else {
                    None
                },
            )
        });

        (sink_slider, source_slider)
    }

    pub fn sinks_submenu<'a>(&self, show_more: bool) -> Element<'a, Message> {
        audio_submenu(
            self.sinks
                .iter()
                .flat_map(|s| {
                    s.ports.iter().map(|p| SubmenuEntry {
                        name: format!("{}: {}", p.description, s.description),
                        device: p.device_type,
                        active: p.active && s.name == self.default_sink,
                        msg: Message::Audio(AudioMessage::DefaultSinkChanged(
                            s.name.clone(),
                            p.name.clone(),
                        )),
                    })
                })
                .collect(),
            if show_more {
                Some(Message::Audio(AudioMessage::SinksMore))
            } else {
                None
            },
        )
    }

    pub fn sources_submenu<'a>(&self, show_more: bool) -> Element<'a, Message> {
        audio_submenu(
            self.sources
                .iter()
                .flat_map(|s| {
                    s.ports.iter().map(|p| SubmenuEntry {
                        name: format!("{}: {}", p.description, s.description),
                        device: p.device_type,
                        active: p.active && s.name == self.default_source,
                        msg: Message::Audio(AudioMessage::DefaultSourceChanged(
                            s.name.clone(),
                            p.name.clone(),
                        )),
                    })
                })
                .collect(),
            if show_more {
                Some(Message::Audio(AudioMessage::SourcesMore))
            } else {
                None
            },
        )
    }

    pub fn subscription(&self) -> Subscription<AudioMessage> {
        crate::utils::audio::subscription(self.audio_commander.give_receiver())
    }
}

pub enum SliderType {
    Sink,
    Source,
}

pub fn audio_slider<'a, Message: 'a + Clone>(
    slider_type: SliderType,
    is_mute: bool,
    toggle_mute: Message,
    volume: i32,
    volume_changed: impl Fn(i32) -> Message + 'a,
    with_submenu: Option<(Option<SubMenu>, Message)>,
) -> Element<'a, Message> {
    Row::with_children(
        vec![
            Some(
                button(icon(if is_mute {
                    match slider_type {
                        SliderType::Sink => Icons::Speaker0,
                        SliderType::Source => Icons::Mic0,
                    }
                } else {
                    match slider_type {
                        SliderType::Sink => Icons::Speaker3,
                        SliderType::Source => Icons::Mic1,
                    }
                }))
                .padding([
                    8,
                    match slider_type {
                        SliderType::Sink => 13,
                        SliderType::Source => 14,
                    },
                ])
                .on_press(toggle_mute)
                .style(Button::custom(SettingsButtonStyle))
                .into(),
            ),
            Some(
                slider(0..=100, volume, volume_changed)
                    .step(1)
                    .width(Length::Fill)
                    .style(theme::Slider::Custom(Box::new(SliderStyle)))
                    .into(),
            ),
            with_submenu.map(|(submenu, msg)| {
                button(icon(match (slider_type, submenu) {
                    (SliderType::Sink, Some(SubMenu::Sinks)) => Icons::Close,
                    (SliderType::Source, Some(SubMenu::Sources)) => Icons::Close,
                    _ => Icons::RightArrow,
                }))
                .padding([8, 13])
                .on_press(msg)
                .style(Button::custom(SettingsButtonStyle))
                .into()
            }),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>(),
    )
    .align_items(Alignment::Center)
    .spacing(8)
    .into()
}

pub struct SubmenuEntry<Message> {
    pub name: String,
    pub device: DeviceType,
    pub active: bool,
    pub msg: Message,
}

pub fn audio_submenu<'a, Message: 'a + Clone>(
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
                            .align_items(Alignment::Center)
                            .spacing(16)
                            .padding([4, 12]),
                    )
                    .style(|theme: &Theme| container::Appearance {
                        text_color: Some(theme.palette().success),
                        ..Default::default()
                    })
                    .into()
                } else {
                    button(
                        row!(icon(e.device.get_icon()), text(e.name))
                            .spacing(16)
                            .align_items(Alignment::Center),
                    )
                    .on_press(e.msg)
                    .padding([4, 12])
                    .width(Length::Fill)
                    .style(Button::custom(GhostButtonStyle))
                    .into()
                }
            })
            .collect::<Vec<_>>(),
    )
    .spacing(4)
    .into();

    if let Some(more_msg) = more_msg {
        column!(
            entries,
            horizontal_rule(1),
            button("More")
                .on_press(more_msg)
                .padding([4, 12])
                .width(Length::Fill)
                .style(Button::custom(GhostButtonStyle)),
        )
        .spacing(12)
        .into()
    } else {
        entries
    }
}
