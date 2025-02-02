use std::ops::Deref;

use super::{Module, OnModulePress};
use crate::{
    app,
    components::icons::{icon, Icons},
    config::MediaPlayerModuleConfig,
    menu::MenuType,
    services::{
        mpris::{MprisPlayerCommand, MprisPlayerData, MprisPlayerService, PlayerCommand},
        ReadOnlyService, Service, ServiceEvent,
    },
    style::SettingsButtonStyle,
    utils::truncate_text,
};
use iced::{
    widget::{button, column, container, row, slider, text},
    Alignment::Center,
    Element, Subscription, Task,
};

#[derive(Default)]
pub struct MediaPlayer {
    data: Vec<PlayerData>,
    service: Option<MprisPlayerService>,
}

struct PlayerData {
    name: String,
    song: Option<String>,
    volume: Option<f64>,
}

#[derive(Debug, Clone)]
pub enum Message {
    Prev(String),
    PlayPause(String),
    Next(String),
    SetVolume(String, f64),
    Event(ServiceEvent<MprisPlayerService>),
}

impl MediaPlayer {
    pub fn update(
        &mut self,
        message: Message,
        config: &MediaPlayerModuleConfig,
    ) -> Task<crate::app::Message> {
        match message {
            Message::Prev(s) => self.handle_command(s, PlayerCommand::Prev),
            Message::PlayPause(s) => self.handle_command(s, PlayerCommand::PlayPause),
            Message::Next(s) => self.handle_command(s, PlayerCommand::Next),
            Message::SetVolume(s, v) => self.handle_command(s, PlayerCommand::Volume(v)),
            Message::Event(event) => match event {
                ServiceEvent::Init(s) => {
                    self.data = Self::map_service_to_module_data(s.deref(), config);
                    self.service = Some(s);
                    Task::none()
                }
                ServiceEvent::Update(d) => {
                    self.data = Self::map_service_to_module_data(&d, config);

                    if let Some(service) = self.service.as_mut() {
                        service.update(d);
                    }
                    Task::none()
                }
                ServiceEvent::Error(_) => Task::none(),
            },
        }
    }

    pub fn menu_view(&self) -> Element<Message> {
        column(
            self.data
                .iter()
                .flat_map(|d| {
                    let buttons = row![
                        button(icon(Icons::SkipPrevious))
                            .on_press(Message::Prev(d.name.clone()))
                            .padding([5, 12])
                            .style(SettingsButtonStyle.into_style()),
                        button(icon(Icons::PlayPause))
                            .on_press(Message::PlayPause(d.name.clone()))
                            .style(SettingsButtonStyle.into_style()),
                        button(icon(Icons::SkipNext))
                            .on_press(Message::Next(d.name.clone()))
                            .padding([5, 12])
                            .style(SettingsButtonStyle.into_style())
                    ]
                    .spacing(8);

                    [
                        iced::widget::horizontal_rule(2).into(),
                        container(
                            column![]
                                .push_maybe(d.song.clone().map(text))
                                .push_maybe(d.volume.map(|v| {
                                    slider(0.0..=100.0, v, |v| {
                                        Message::SetVolume(d.name.clone(), v)
                                    })
                                }))
                                .push(buttons)
                                .width(iced::Length::Fill)
                                .spacing(12)
                                .align_x(Center),
                        )
                        .padding(16)
                        .into(),
                    ]
                })
                .skip(1),
        )
        .spacing(16)
        .into()
    }

    fn handle_command(
        &mut self,
        service_name: String,
        command: PlayerCommand,
    ) -> Task<crate::app::Message> {
        if let Some(s) = self.service.as_mut() {
            s.command(MprisPlayerCommand {
                service_name,
                command,
            })
            .map(|event| crate::app::Message::MediaPlayer(Message::Event(event)))
        } else {
            Task::none()
        }
    }

    fn map_service_to_module_data(
        data: &[MprisPlayerData],
        config: &MediaPlayerModuleConfig,
    ) -> Vec<PlayerData> {
        data.iter()
            .map(|d| PlayerData {
                name: d.service.clone(),
                song: d
                    .metadata
                    .clone()
                    .map(|d| truncate_text(&d.to_string(), config.max_title_length)),
                volume: d.volume,
            })
            .collect()
    }
}

impl Module for MediaPlayer {
    type ViewData<'a> = ();
    type SubscriptionData<'a> = ();

    fn view(
        &self,
        (): Self::ViewData<'_>,
    ) -> Option<(Element<app::Message>, Option<OnModulePress>)> {
        match self.data.len() {
            0 => None,
            1 => self.data[0].song.clone().map(|s| {
                (
                    text(s).into(),
                    Some(OnModulePress::ToggleMenu(MenuType::MediaPlayer)),
                )
            }),
            _ => Some((
                icon(Icons::MusicNote).into(),
                Some(OnModulePress::ToggleMenu(MenuType::MediaPlayer)),
            )),
        }
    }

    fn subscription(&self, (): Self::SubscriptionData<'_>) -> Option<Subscription<app::Message>> {
        Some(
            MprisPlayerService::subscribe()
                .map(|event| app::Message::MediaPlayer(Message::Event(event))),
        )
    }
}
