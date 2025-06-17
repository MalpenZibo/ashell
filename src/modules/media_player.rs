use super::{Module, OnModulePress};
use crate::{
    app,
    components::icons::{Icons, icon},
    config::MediaPlayerModuleConfig,
    menu::MenuType,
    services::{
        ReadOnlyService, Service, ServiceEvent,
        mpris::{
            MprisPlayerCommand, MprisPlayerData, MprisPlayerService, PlaybackStatus, PlayerCommand,
        },
    },
    style::settings_button_style,
    utils::truncate_text,
};
use iced::{
    Background, Border, Element, Length, Subscription, Task, Theme,
    alignment::Vertical,
    widget::{Column, button, column, container, horizontal_rule, row, slider, text},
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

    pub fn menu_view(&self, config: &MediaPlayerModuleConfig, opacity: f32) -> Element<Message> {
        match &self.service {
            None => text("Not connected to MPRIS service").into(),
            Some(s) => column!(
                text("Players").size(20),
                horizontal_rule(1),
                column(s.iter().map(|d| {
                    let title = text(Self::get_title(d, config))
                        .wrapping(text::Wrapping::WordOrGlyph)
                        .width(Length::Fill);

                    let play_pause_icon = match d.state {
                        PlaybackStatus::Playing => Icons::Pause,
                        PlaybackStatus::Paused | PlaybackStatus::Stopped => Icons::Play,
                    };

                    let buttons = row![
                        button(icon(Icons::SkipPrevious))
                            .on_press(Message::Prev(d.service.clone()))
                            .padding([5, 12])
                            .style(settings_button_style(opacity)),
                        button(icon(play_pause_icon))
                            .on_press(Message::PlayPause(d.service.clone()))
                            .style(settings_button_style(opacity)),
                        button(icon(Icons::SkipNext))
                            .on_press(Message::Next(d.service.clone()))
                            .padding([5, 12])
                            .style(settings_button_style(opacity)),
                    ]
                    .spacing(8);

                    let volume_slider = d.volume.map(|v| {
                        slider(0.0..=100.0, v, move |v| {
                            Message::SetVolume(d.service.clone(), v)
                        })
                    });

                    container(
                        Column::new()
                            .push(row!(title, buttons).spacing(8).align_y(Vertical::Center))
                            .push_maybe(volume_slider)
                            .spacing(8),
                    )
                    .style(move |theme: &Theme| container::Style {
                        background: Background::Color(
                            theme
                                .extended_palette()
                                .secondary
                                .strong
                                .color
                                .scale_alpha(opacity),
                        )
                        .into(),
                        border: Border::default().rounded(16),
                        ..container::Style::default()
                    })
                    .padding(16)
                    .width(Length::Fill)
                    .into()
                }))
                .spacing(16)
            )
            .spacing(8)
            .into(),
        }
    }

    fn handle_command(
        &mut self,
        service_name: String,
        command: PlayerCommand,
    ) -> Task<crate::app::Message> {
        match self.service.as_mut() {
            Some(s) => s
                .command(MprisPlayerCommand {
                    service_name,
                    command,
                })
                .map(|event| crate::app::Message::MediaPlayer(Message::Event(event))),
            _ => Task::none(),
        }
    }

    fn get_title(d: &MprisPlayerData, config: &MediaPlayerModuleConfig) -> String {
        match &d.metadata {
            Some(m) => truncate_text(&m.to_string(), config.max_title_length),
            None => "No Title".to_string(),
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
        self.service.as_ref().and_then(|s| match s.len() {
            0 => None,
            _ => Some((
                row![
                    icon(Icons::MusicNote),
                    text(Self::get_title(&s[0], config))
                        .wrapping(text::Wrapping::WordOrGlyph)
                        .size(12)
                ]
                .align_y(Vertical::Center)
                .spacing(8)
                .into(),
                Some(OnModulePress::ToggleMenu(MenuType::MediaPlayer)),
            )),
        })
    }

    fn subscription(&self, (): Self::SubscriptionData<'_>) -> Option<Subscription<app::Message>> {
        Some(
            MprisPlayerService::subscribe()
                .map(|event| app::Message::MediaPlayer(Message::Event(event))),
        )
    }
}
