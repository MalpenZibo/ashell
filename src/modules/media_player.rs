use std::ops::Deref;

use super::{Module, OnModulePress};
use crate::{
    app,
    components::icons::{icon, Icons},
    config::MediaPlayerModuleConfig,
    menu::MenuType,
    services::{
        mpris::{MprisPlayerCommand, MprisPlayerService, PlayerCommand},
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
    service: Option<MprisPlayerService>,
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
    pub fn update(&mut self, message: Message) -> Task<crate::app::Message> {
        match message {
            Message::Prev(s) => self.handle_command(s, PlayerCommand::Prev),
            Message::PlayPause(s) => self.handle_command(s, PlayerCommand::PlayPause),
            Message::Next(s) => self.handle_command(s, PlayerCommand::Next),
            Message::SetVolume(s, v) => self.handle_command(s, PlayerCommand::Volume(v)),
            Message::Event(event) => match event {
                ServiceEvent::Init(s) => {
                    self.service = Some(s);
                    Task::none()
                }
                ServiceEvent::Update(d) => {
                    if let Some(service) = self.service.as_mut() {
                        service.update(d);
                    }
                    Task::none()
                }
                ServiceEvent::Error(_) => Task::none(),
            },
        }
    }

    pub fn menu_view(&self, config: &MediaPlayerModuleConfig) -> Element<Message> {
        match &self.service {
            Some(s) => column(
                s.deref()
                    .iter()
                    .flat_map(|d| {
                        let d = d.clone();
                        let buttons = row![
                            button(icon(Icons::SkipPrevious))
                                .on_press(Message::Prev(d.service.clone()))
                                .padding([5, 12])
                                .style(SettingsButtonStyle.into_style()),
                            button(icon(Icons::PlayPause))
                                .on_press(Message::PlayPause(d.service.clone()))
                                .style(SettingsButtonStyle.into_style()),
                            button(icon(Icons::SkipNext))
                                .on_press(Message::Next(d.service.clone()))
                                .padding([5, 12])
                                .style(SettingsButtonStyle.into_style())
                        ]
                        .spacing(8);

                        [
                            iced::widget::horizontal_rule(2).into(),
                            container(
                                column![]
                                    .push(text(match d.metadata {
                                        Some(m) => {
                                            truncate_text(&m.to_string(), config.max_title_length)
                                        }
                                        None => "No Title".to_string(),
                                    }))
                                    .push_maybe(d.volume.map(|v| {
                                        slider(0.0..=100.0, v, move |v| {
                                            Message::SetVolume(d.service.clone(), v)
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
            .into(),
            None => text("Not connected to MPRIS service").into(),
        }
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
}

impl Module for MediaPlayer {
    type ViewData<'a> = &'a MediaPlayerModuleConfig;
    type SubscriptionData<'a> = ();

    fn view(
        &self,
        config: Self::ViewData<'_>,
    ) -> Option<(Element<app::Message>, Option<OnModulePress>)> {
        match &self.service {
            Some(s) => {
                let deref = s.deref();
                match deref.len() {
                    0 => None,
                    1 => match &deref[0].metadata {
                        Some(m) => Some((
                            text(truncate_text(&m.to_string(), config.max_title_length)).into(),
                            Some(OnModulePress::ToggleMenu(MenuType::MediaPlayer)),
                        )),
                        None => Some((
                            icon(Icons::MusicNote).into(),
                            Some(OnModulePress::ToggleMenu(MenuType::MediaPlayer)),
                        )),
                    },
                    _ => Some((
                        icon(Icons::MusicNote).into(),
                        Some(OnModulePress::ToggleMenu(MenuType::MediaPlayer)),
                    )),
                }
            }
            None => None,
        }
    }

    fn subscription(&self, (): Self::SubscriptionData<'_>) -> Option<Subscription<app::Message>> {
        Some(
            MprisPlayerService::subscribe()
                .map(|event| app::Message::MediaPlayer(Message::Event(event))),
        )
    }
}
