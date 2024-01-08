use iced::{
    theme::Button,
    widget::{button, column, row, slider, text, Column, Row, Text},
    Alignment, Element, Length,
};

use crate::{
    components::icons::{icon, Icons},
    style::{GhostButtonStyle, SettingsButtonStyle, YELLOW},
    utils::audio::{Sink, Sinks, Source, Sources},
};

use super::SubMenu;

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
    volume_confirm: Message,
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
                .padding([8, 9])
                .on_press(toggle_mute)
                .style(Button::custom(SettingsButtonStyle))
                .into(),
            ),
            Some(
                slider(0..=100, volume, volume_changed)
                    .step(1)
                    .on_release(volume_confirm)
                    .width(Length::Fill)
                    .into(),
            ),
            with_submenu.map(|(submenu, msg)| {
                button(icon(match (slider_type, submenu) {
                    (SliderType::Sink, Some(SubMenu::Sinks)) => Icons::Close,
                    (SliderType::Source, Some(SubMenu::Sources)) => Icons::Close,
                    _ => Icons::RightArrow,
                }))
                .padding([8, 9])
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
                    row!(icon(Icons::Point), text(e.name))
                        .align_items(Alignment::Center)
                        .spacing(8)
                        .padding([4, 12])
                        .into()
                } else {
                    button(text(e.name))
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
