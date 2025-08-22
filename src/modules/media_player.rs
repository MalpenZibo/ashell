use crate::{
    components::icons::{Icons, icon},
    config::MediaPlayerModuleConfig,
    services::{
        ReadOnlyService, Service, ServiceEvent,
        mpris::{
            MprisPlayerCommand, MprisPlayerData, MprisPlayerService, PlaybackStatus, PlayerCommand,
        },
    },
    theme::AshellTheme,
    utils::truncate_text,
};
use iced::{
    Background, Border, Element, Length, Subscription, Task, Theme,
    alignment::Vertical,
    widget::{Column, button, column, container, horizontal_rule, row, slider, text},
};

#[derive(Debug, Clone)]
pub enum Message {
    Prev(String),
    PlayPause(String),
    Next(String),
    SetVolume(String, f64),
    Event(ServiceEvent<MprisPlayerService>),
    ConfigReloaded(MediaPlayerModuleConfig),
}

pub enum Action {
    None,
    Command(Task<Message>),
}

pub struct MediaPlayer {
    config: MediaPlayerModuleConfig,
    service: Option<MprisPlayerService>,
}

impl MediaPlayer {
    pub fn new(config: MediaPlayerModuleConfig) -> Self {
        Self {
            config,
            service: None,
        }
    }

    pub fn update(&mut self, message: Message) -> Action {
        match message {
            Message::Prev(s) => Action::Command(self.handle_command(s, PlayerCommand::Prev)),
            Message::PlayPause(s) => {
                Action::Command(self.handle_command(s, PlayerCommand::PlayPause))
            }
            Message::Next(s) => Action::Command(self.handle_command(s, PlayerCommand::Next)),
            Message::SetVolume(s, v) => {
                Action::Command(self.handle_command(s, PlayerCommand::Volume(v)))
            }
            Message::Event(event) => match event {
                ServiceEvent::Init(s) => {
                    self.service = Some(s);
                    Action::None
                }
                ServiceEvent::Update(d) => {
                    if let Some(service) = self.service.as_mut() {
                        service.update(d);
                    }
                    Action::None
                }
                ServiceEvent::Error(_) => Action::None,
            },
            Message::ConfigReloaded(c) => {
                self.config = c;
                Action::None
            }
        }
    }

    pub fn menu_view<'a>(&'a self, theme: &'a AshellTheme) -> Element<'a, Message> {
        match &self.service {
            None => text("Not connected to MPRIS service").into(),
            Some(s) => column!(
                text("Players").size(theme.font_size.lg),
                horizontal_rule(1),
                column(s.iter().map(|d| {
                    let title = text(self.get_title(d))
                        .wrapping(text::Wrapping::WordOrGlyph)
                        .width(Length::Fill);

                    let play_pause_icon = match d.state {
                        PlaybackStatus::Playing => Icons::Pause,
                        PlaybackStatus::Paused | PlaybackStatus::Stopped => Icons::Play,
                    };

                    let buttons = row![
                        button(icon(Icons::SkipPrevious))
                            .on_press(Message::Prev(d.service.clone()))
                            .padding([theme.space.xs, theme.space.md])
                            .style(theme.settings_button_style()),
                        button(icon(play_pause_icon))
                            .on_press(Message::PlayPause(d.service.clone()))
                            .style(theme.settings_button_style()),
                        button(icon(Icons::SkipNext))
                            .on_press(Message::Next(d.service.clone()))
                            .padding([theme.space.xs, theme.space.md])
                            .style(theme.settings_button_style()),
                    ]
                    .spacing(theme.space.xs);

                    let volume_slider = d.volume.map(|v| {
                        slider(0.0..=100.0, v, move |v| {
                            Message::SetVolume(d.service.clone(), v)
                        })
                    });

                    container(
                        Column::new()
                            .push(
                                row!(title, buttons)
                                    .spacing(theme.space.xs)
                                    .align_y(Vertical::Center),
                            )
                            .push_maybe(volume_slider)
                            .spacing(theme.space.xs),
                    )
                    .style(move |app_theme: &Theme| container::Style {
                        background: Background::Color(
                            app_theme
                                .extended_palette()
                                .secondary
                                .strong
                                .color
                                .scale_alpha(theme.opacity),
                        )
                        .into(),
                        border: Border::default().rounded(theme.radius.lg),
                        ..container::Style::default()
                    })
                    .padding(theme.space.md)
                    .width(Length::Fill)
                    .into()
                }))
                .spacing(theme.space.md)
            )
            .spacing(theme.space.xs)
            .into(),
        }
    }

    fn handle_command(&mut self, service_name: String, command: PlayerCommand) -> Task<Message> {
        match self.service.as_mut() {
            Some(s) => s
                .command(MprisPlayerCommand {
                    service_name,
                    command,
                })
                .map(Message::Event),
            _ => Task::none(),
        }
    }

    fn get_title(&self, d: &MprisPlayerData) -> String {
        match &d.metadata {
            Some(m) => truncate_text(&m.to_string(), self.config.max_title_length),
            None => "No Title".to_string(),
        }
    }

    pub fn view(&'_ self, theme: &AshellTheme) -> Option<Element<'_, Message>> {
        self.service.as_ref().and_then(|s| match s.len() {
            0 => None,
            _ => Some(
                row![
                    icon(Icons::MusicNote),
                    text(self.get_title(&s[0]))
                        .wrapping(text::Wrapping::WordOrGlyph)
                        .size(theme.font_size.sm)
                ]
                .align_y(Vertical::Center)
                .spacing(theme.space.xs)
                .into(),
            ),
        })
    }

    pub fn subscription(&self) -> Subscription<Message> {
        MprisPlayerService::subscribe().map(Message::Event)
    }
}
