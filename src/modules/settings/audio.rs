use iced::{
    theme::Button,
    widget::{button, row, slider, text, Column, Row},
    Alignment, Element, Length,
};

use crate::{
    components::icons::{icon, Icons},
    style::{GhostButtonStyle, SettingsButtonStyle, GREEN, YELLOW},
    utils::audio::{AudioCommand, DeviceType, Sink, Sinks, Source, Sources, Volume},
};

use super::{Settings, SubMenu, Message};

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
}

impl AudioMessage {
    pub fn update(self, settings: &mut Settings) {
        match self {
            AudioMessage::SinkChanges(sinks) => {
                settings.sinks = sinks;
                settings.cur_sink_volume = (settings
                    .sinks
                    .iter()
                    .find_map(|sink| {
                        if sink
                            .ports
                            .iter()
                            .any(|p| p.active && sink.name == settings.default_sink)
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
            }
            AudioMessage::SourceChanges(sources) => {
                settings.sources = sources;
                settings.cur_source_volume = (settings
                    .sources
                    .iter()
                    .find_map(|source| {
                        if source
                            .ports
                            .iter()
                            .any(|p| p.active && source.name == settings.default_source)
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
            }
            AudioMessage::DefaultSinkSourceChanged(sink, source) => {
                settings.default_sink = sink;
                settings.default_source = source;

                settings.cur_sink_volume = (settings
                    .sinks
                    .iter()
                    .find_map(|sink| {
                        if sink
                            .ports
                            .iter()
                            .any(|p| p.active && sink.name == settings.default_sink)
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
                settings.cur_source_volume = (settings
                    .sources
                    .iter()
                    .find_map(|source| {
                        if source
                            .ports
                            .iter()
                            .any(|p| p.active && source.name == settings.default_source)
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
            }
            AudioMessage::SinkToggleMute => {
                if let Some(sink) = settings
                    .sinks
                    .iter()
                    .find(|sink| sink.name == settings.default_sink)
                {
                    let _ = settings.audio_commander.send(AudioCommand::SinkMute(
                        settings.default_sink.clone(),
                        !sink.is_mute,
                    ));
                }
            }
            AudioMessage::SinkVolumeChanged(volume) => {
                settings.cur_sink_volume = volume;
                if let Some(sink) = settings
                    .sinks
                    .iter_mut()
                    .find(|sink| sink.name == settings.default_sink)
                {
                    if let Some(new_volume) = sink
                        .volume
                        .scale_volume(settings.cur_sink_volume as f64 / 100.)
                    {
                        let _ = settings
                            .audio_commander
                            .send(AudioCommand::SinkVolume(sink.name.clone(), *new_volume));
                    }
                }
            }
            AudioMessage::DefaultSinkChanged(name, port) => {
                settings.default_sink = name.clone();
                for sink in settings.sinks.iter_mut() {
                    for cur_port in sink.ports.iter_mut() {
                        cur_port.active = sink.name == name && cur_port.name == port;
                    }
                }

                let _ = settings
                    .audio_commander
                    .send(AudioCommand::DefaultSink(name, port));
            }
            AudioMessage::SourceToggleMute => {
                if let Some(source) = settings
                    .sources
                    .iter()
                    .find(|source| source.name == settings.default_source)
                {
                    let _ = settings.audio_commander.send(AudioCommand::SourceMute(
                        settings.default_source.clone(),
                        !source.is_mute,
                    ));
                }
            }
            AudioMessage::SourceVolumeChanged(volume) => {
                settings.cur_source_volume = volume;
                if let Some(source) = settings
                    .sources
                    .iter_mut()
                    .find(|source| source.name == settings.default_source)
                {
                    if let Some(new_volume) = source
                        .volume
                        .scale_volume(settings.cur_source_volume as f64 / 100.)
                    {
                        let _ = settings
                            .audio_commander
                            .send(AudioCommand::SourceVolume(source.name.clone(), *new_volume));
                    }
                }
            }
            AudioMessage::DefaultSourceChanged(name, port) => {
                settings.default_source = name.clone();
                for source in settings.sources.iter_mut() {
                    for cur_port in source.ports.iter_mut() {
                        cur_port.active = source.name == name && cur_port.name == port;
                    }
                }

                let _ = settings
                    .audio_commander
                    .send(AudioCommand::DefaultSource(name, port));
            }
        }
    }
}

pub fn sink_indicator<'a, Message>(data: &Vec<Sink>) -> Option<Element<'a, Message>> {
    if !data.is_empty() {
        let icon_type = data.get_icon();

        Some(icon(icon_type).into())
    } else {
        None
    }
}

pub fn source_indicator<'a, Message>(data: &Vec<Source>) -> Option<Element<'a, Message>> {
    if !data.is_empty() {
        let icon_type = data.get_icon();

        Some(icon(icon_type).style(YELLOW).into())
    } else {
        None
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
                .padding([8, 10])
                .on_press(toggle_mute)
                .style(Button::custom(SettingsButtonStyle))
                .into(),
            ),
            Some(
                slider(0..=100, volume, volume_changed)
                    .step(1)
                    .width(Length::Fill)
                    .into(),
            ),
            with_submenu.map(|(submenu, msg)| {
                button(icon(match (slider_type, submenu) {
                    (SliderType::Sink, Some(SubMenu::Sinks)) => Icons::Close,
                    (SliderType::Source, Some(SubMenu::Sources)) => Icons::Close,
                    _ => Icons::RightArrow,
                }))
                .padding([8, 10])
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

pub fn get_audio_sliders<'a>(
    sinks: &[Sink],
    cur_sink_volume: i32,
    sources: &[Source],
    cur_source_volume: i32,
    sub_menu: Option<SubMenu>,
) -> (Option<Element<'a, Message>>, Option<Element<'a, Message>>) {
    let active_sink = sinks
        .iter()
        .find(|sink| sink.ports.iter().any(|p| p.active));

    let sink_slider = active_sink.map(|s| {
        audio_slider(
            SliderType::Sink,
            s.is_mute,
            Message::Audio(AudioMessage::SinkToggleMute),
            cur_sink_volume,
            |v| Message::Audio(AudioMessage::SinkVolumeChanged(v)),
            if sinks.iter().map(|s| s.ports.len()).sum::<usize>() > 1 {
                Some((sub_menu, Message::ToggleSubMenu(SubMenu::Sinks)))
            } else {
                None
            },
        )
    });

    let active_source = sources
        .iter()
        .find(|source| source.ports.iter().any(|p| p.active));

    let source_slider = active_source.map(|s| {
        audio_slider(
            SliderType::Source,
            s.is_mute,
            Message::Audio(AudioMessage::SourceToggleMute),
            cur_source_volume,
            |v| Message::Audio(AudioMessage::SourceVolumeChanged(v)),
            if sources.iter().map(|s| s.ports.len()).sum::<usize>() > 1 {
                Some((sub_menu, Message::ToggleSubMenu(SubMenu::Sources)))
            } else {
                None
            },
        )
    });

    (sink_slider, source_slider)
}

pub struct SubmenuEntry<Message> {
    pub name: String,
    pub device: DeviceType,
    pub active: bool,
    pub msg: Message,
}

pub fn audio_submenu<'a, Message: 'a + Clone>(
    entries: Vec<SubmenuEntry<Message>>,
) -> Element<'a, Message> {
    Column::with_children(
        entries
            .into_iter()
            .map(|e| {
                if e.active {
                    row!(
                        icon(e.device.get_icon()).style(GREEN),
                        text(e.name).style(GREEN)
                    )
                    .align_items(Alignment::Center)
                    .spacing(16)
                    .padding([4, 12])
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
    .into()
}
