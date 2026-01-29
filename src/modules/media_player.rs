use crate::{
    components::icons::{IconButtonSize, StaticIcon, icon, icon_button},
    config::{MediaPlayerFormat, MediaPlayerModuleConfig},
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
    widget::{Row, column, container, horizontal_rule, horizontal_space, image, row, slider, text},
};
use std::collections::{HashMap, HashSet};

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
    covers: HashMap<String, image::Handle>,
}

impl MediaPlayer {
    pub fn new(config: MediaPlayerModuleConfig) -> Self {
        Self {
            config,
            service: None,
            covers: HashMap::new(),
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
                    self.sync_cover_handles();
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
            Some(service) => column!(
                text("Players").size(theme.font_size.lg),
                horizontal_rule(1),
                column(service.players().iter().map(|d| {
                    let m = d.metadata.as_ref();
                    let title = m
                        .and_then(|m| m.title.clone())
                        .unwrap_or("No Title".to_string());
                    let artists = m
                        .and_then(|m| m.artists.clone())
                        .map(|a| a.join(", "))
                        .unwrap_or("Unknown Artist".to_string());
                    let album = m
                        .and_then(|m| m.album.clone())
                        .unwrap_or("Unknown Album".to_string());
                    let title = text(truncate_text(&title, self.config.max_title_length))
                        .wrapping(text::Wrapping::WordOrGlyph)
                        .width(Length::Fill);
                    let artists = text(truncate_text(&artists, self.config.max_title_length))
                        .wrapping(text::Wrapping::WordOrGlyph)
                        .size(theme.font_size.sm)
                        .width(Length::Fill);
                    let album = text(truncate_text(&album, self.config.max_title_length))
                        .wrapping(text::Wrapping::WordOrGlyph)
                        .size(theme.font_size.sm)
                        .width(Length::Fill);
                    let description = column![title, artists, album]
                        .spacing(theme.space.xxs)
                        .width(Length::Fill);

                    let play_pause_icon = match d.state {
                        PlaybackStatus::Playing => StaticIcon::Pause,
                        PlaybackStatus::Paused | PlaybackStatus::Stopped => StaticIcon::Play,
                    };

                    let buttons = row![
                        icon_button(theme, StaticIcon::SkipPrevious)
                            .on_press(Message::Prev(d.service.clone()))
                            .size(IconButtonSize::Large),
                        icon_button(theme, play_pause_icon)
                            .on_press(Message::PlayPause(d.service.clone()))
                            .size(IconButtonSize::Large),
                        icon_button(theme, StaticIcon::SkipNext)
                            .on_press(Message::Next(d.service.clone()))
                            .size(IconButtonSize::Large),
                    ]
                    .align_y(Vertical::Center)
                    .spacing(theme.space.xs);
                    let volume_slider: Element<'_, _> = match d.volume {
                        Some(v) => slider(0.0..=100.0, v, move |v| {
                            Message::SetVolume(d.service.clone(), v)
                        })
                        .width(Length::Fill)
                        .into(),
                        None => horizontal_space().into(),
                    };
                    let controls = Row::new()
                        .push(volume_slider)
                        .push(buttons)
                        .spacing(theme.space.md)
                        .align_y(Vertical::Center);

                    // Is it possible to dynamically size the cover to match the buttons?
                    let buttons_width =
                        IconButtonSize::Large.container_size() * 3. + theme.space.xs as f32 * 2.;

                    let cover: Option<Element<_, _>> = d
                        .metadata
                        .as_ref()
                        .and_then(|m| m.art_url.as_ref())
                        .map(|url| {
                            self.covers
                                .get(url)
                                .map(|handle| {
                                    image(handle)
                                        .filter_method(image::FilterMethod::Linear)
                                        .width(buttons_width)
                                        .into()
                                })
                                .unwrap_or_else(|| {
                                    text("Loading cover...")
                                        .width(buttons_width)
                                        .height(buttons_width)
                                        .into()
                                })
                        });
                    let metadata = row![description]
                        .push_maybe(cover)
                        .spacing(theme.space.md)
                        .align_y(Vertical::Center);

                    container(
                        column![metadata, controls].spacing(theme.space.xs), // .align_y(Vertical::Center),
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
        self.service.as_ref().and_then(|s| {
            s.players().first().map(|player| {
                let title =
                    (self.config.indicator_format == MediaPlayerFormat::IconAndTitle).then(|| {
                        container(
                            text(self.get_title(player))
                                .wrapping(text::Wrapping::None)
                                .size(theme.font_size.sm),
                        )
                        .clip(true)
                    });

                row![icon(StaticIcon::MusicNote)]
                    .push_maybe(title)
                    .align_y(Vertical::Center)
                    .spacing(theme.space.xs)
                    .into()
            })
        })
    }

    pub fn subscription(&self) -> Subscription<Message> {
        MprisPlayerService::subscribe().map(Message::Event)
    }

    fn sync_cover_handles(&mut self) {
        let Some(service) = &self.service else {
            return;
        };

        let desired_urls: HashSet<String> = service
            .players()
            .iter()
            .filter_map(|player| player.metadata.as_ref()?.art_url.clone())
            .collect();
        self.covers.retain(|url, _| desired_urls.contains(url));
        let unloaded_urls: HashSet<String> = desired_urls
            .difference(&self.covers.keys().cloned().collect())
            .cloned()
            .collect();

        for url in unloaded_urls {
            let Some(cover) = service.get_cover(&url) else {
                continue;
            };
            self.covers
                .insert(url.clone(), image::Handle::from_bytes(cover.clone()));
        }
    }
}
