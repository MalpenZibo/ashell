use super::{Message, SubMenu};
use crate::{
    components::icons::{Icons, icon},
    services::{
        ServiceEvent,
        audio::{AudioData, AudioService, DeviceType, Sinks},
    },
    style::{ghost_button_style, settings_button_style},
};
use iced::{
    Alignment, Element, Length, Theme,
    widget::{Column, Row, button, column, container, horizontal_rule, row, slider, text},
    window::Id,
};

#[derive(Debug, Clone)]
pub enum AudioMessage {
    Event(ServiceEvent<AudioService>),
    DefaultSinkChanged(String, String),
    DefaultSourceChanged(String, String),
    ToggleSinkMute,
    SinkVolumeChanged(i32),
    ToggleSourceMute,
    SourceVolumeChanged(i32),
    SinksMore(Id),
    SourcesMore(Id),
}

impl AudioData {
    pub fn sink_indicator<Message>(&self) -> Option<Element<Message>> {
        if !self.sinks.is_empty() {
            let icon_type = self.sinks.get_icon(&self.server_info.default_sink);

            Some(icon(icon_type).into())
        } else {
            None
        }
    }

    pub fn audio_sliders(
        &self,
        sub_menu: Option<SubMenu>,
        opacity: f32,
    ) -> (Option<Element<Message>>, Option<Element<Message>>) {
        let active_sink = self
            .sinks
            .iter()
            .find(|sink| sink.name == self.server_info.default_sink);

        let sink_slider = active_sink.map(|s| {
            audio_slider(
                SliderType::Sink,
                s.is_mute,
                Message::Audio(AudioMessage::ToggleSinkMute),
                self.cur_sink_volume,
                |v| Message::Audio(AudioMessage::SinkVolumeChanged(v)),
                if self.sinks.iter().map(|s| s.ports.len()).sum::<usize>() > 1 {
                    Some((sub_menu, Message::ToggleSubMenu(SubMenu::Sinks)))
                } else {
                    None
                },
                opacity,
            )
        });

        if self.sources.iter().any(|source| source.in_use) {
            let active_source = self
                .sources
                .iter()
                .find(|source| source.name == self.server_info.default_source);

            let source_slider = active_source.map(|s| {
                audio_slider(
                    SliderType::Source,
                    s.is_mute,
                    Message::Audio(AudioMessage::ToggleSourceMute),
                    self.cur_source_volume,
                    |v| Message::Audio(AudioMessage::SourceVolumeChanged(v)),
                    if self.sources.iter().map(|s| s.ports.len()).sum::<usize>() > 1 {
                        Some((sub_menu, Message::ToggleSubMenu(SubMenu::Sources)))
                    } else {
                        None
                    },
                    opacity,
                )
            });

            (sink_slider, source_slider)
        } else {
            (sink_slider, None)
        }
    }

    pub fn sinks_submenu(&self, id: Id, show_more: bool, opacity: f32) -> Element<Message> {
        audio_submenu(
            self.sinks
                .iter()
                .flat_map(|s| {
                    s.ports.iter().map(|p| SubmenuEntry {
                        name: format!("{}: {}", p.description, s.description),
                        device: p.device_type,
                        active: p.active && s.name == self.server_info.default_sink,
                        msg: Message::Audio(AudioMessage::DefaultSinkChanged(
                            s.name.clone(),
                            p.name.clone(),
                        )),
                    })
                })
                .collect(),
            if show_more {
                Some(Message::Audio(AudioMessage::SinksMore(id)))
            } else {
                None
            },
            opacity,
        )
    }

    pub fn sources_submenu(&self, id: Id, show_more: bool, opacity: f32) -> Element<Message> {
        audio_submenu(
            self.sources
                .iter()
                .flat_map(|s| {
                    s.ports.iter().map(|p| SubmenuEntry {
                        name: format!("{}: {}", p.description, s.description),
                        device: p.device_type,
                        active: p.active && s.name == self.server_info.default_source,
                        msg: Message::Audio(AudioMessage::DefaultSourceChanged(
                            s.name.clone(),
                            p.name.clone(),
                        )),
                    })
                })
                .collect(),
            if show_more {
                Some(Message::Audio(AudioMessage::SourcesMore(id)))
            } else {
                None
            },
            opacity,
        )
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
    opacity: f32,
) -> Element<'a, Message> {
    Row::new()
        .push(
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
            .style(settings_button_style(opacity)),
        )
        .push(
            slider(0..=100, volume, volume_changed)
                .step(1)
                .width(Length::Fill),
        )
        .push_maybe(with_submenu.map(|(submenu, msg)| {
            button(icon(match (slider_type, submenu) {
                (SliderType::Sink, Some(SubMenu::Sinks)) => Icons::Close,
                (SliderType::Source, Some(SubMenu::Sources)) => Icons::Close,
                _ => Icons::RightArrow,
            }))
            .padding([8, 13])
            .on_press(msg)
            .style(settings_button_style(opacity))
        }))
        .align_y(Alignment::Center)
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
    opacity: f32,
) -> Element<'a, Message> {
    let entries = Column::with_children(
        entries
            .into_iter()
            .map(|e| {
                if e.active {
                    container(
                        row!(icon(e.device.get_icon()), text(e.name))
                            .align_y(Alignment::Center)
                            .spacing(16)
                            .padding([4, 12]),
                    )
                    .style(|theme: &Theme| container::Style {
                        text_color: Some(theme.palette().success),
                        ..Default::default()
                    })
                    .into()
                } else {
                    button(
                        row!(icon(e.device.get_icon()), text(e.name))
                            .spacing(16)
                            .align_y(Alignment::Center),
                    )
                    .on_press(e.msg)
                    .padding([4, 12])
                    .width(Length::Fill)
                    .style(ghost_button_style(opacity))
                    .into()
                }
            })
            .collect::<Vec<_>>(),
    )
    .spacing(4)
    .into();

    match more_msg {
        Some(more_msg) => column!(
            entries,
            horizontal_rule(1),
            button("More")
                .on_press(more_msg)
                .padding([4, 12])
                .width(Length::Fill)
                .style(ghost_button_style(opacity)),
        )
        .spacing(12)
        .into(),
        _ => entries,
    }
}
