use std::collections::{HashMap, HashSet};

use crate::{
    components::icons::{IconButtonSize, StaticIcon, icon, icon_button},
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
    core::image::Bytes,
    widget::{Column, column, container, horizontal_rule, image, row, slider, text},
};
use itertools::Itertools;

type CoverData = Bytes;

#[derive(Debug, Clone)]
pub enum Message {
    Prev(String),
    PlayPause(String),
    Next(String),
    SetVolume(String, f64),
    Event(ServiceEvent<MprisPlayerService>),
    ConfigReloaded(MediaPlayerModuleConfig),
    CoverLoaded(String, CoverData),
    CoverLoadFailed(String, String),
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
                        service.update(d.clone());
                    }
                    self.check_cover_update(&d)
                }
                ServiceEvent::Error(_) => Action::None,
            },
            Message::ConfigReloaded(c) => {
                self.config = c;
                Action::None
            }
            Message::CoverLoaded(url, data) => {
                log::debug!("Loaded cover from {}", url);
                self.covers.insert(url, image::Handle::from_bytes(data));
                Action::None
            }
            Message::CoverLoadFailed(url, err) => {
                log::error!("Failed to load cover from {}: {}", url, err);
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

                    let cover: Option<Element<_, _>> = d
                        .metadata
                        .as_ref()
                        .and_then(|m| m.art_url.as_ref())
                        .map(|url| {
                            // TODO: width is copied from impl From<IconButton> for Element
                            // (3 buttons of IconButtonSize::Large, with xs spacing inbetween)
                            // It doesn't look like it's possible to limit cover to the width of
                            // buttons without hardcoding the width, but we should at least use a
                            // common constant for the button size.
                            let width = 38 * 3 + theme.space.xs * 2;
                            let img = self.covers.get(url);
                            img.map(|img| {
                                image(img)
                                    .filter_method(image::FilterMethod::Linear)
                                    .width(width)
                                    .into()
                            })
                            .unwrap_or_else(|| {
                                text("Loading cover...").width(width).height(width).into()
                            })
                        });
                    let right = match cover {
                        Some(cover) => column![cover, buttons].spacing(theme.space.xs),
                        None => column![buttons],
                    };

                    let volume_slider = d.volume.map(|v| {
                        slider(0.0..=100.0, v, move |v| {
                            Message::SetVolume(d.service.clone(), v)
                        })
                    });

                    container(
                        Column::new()
                            .push(
                                row!(title, right)
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

    fn check_cover_update(&mut self, data: &Vec<MprisPlayerData>) -> Action {
        let urls: HashSet<_> = data
            .iter()
            .filter_map(|player| player.metadata.as_ref().and_then(|m| m.art_url.clone()))
            .collect();

        let unused_covers = self
            .covers
            .keys()
            .filter(|url| !urls.contains(url.as_str()))
            .cloned()
            .collect_vec();
        for url in unused_covers {
            self.covers.remove(url.as_str());
            log::debug!("Removed unused cover for {}", url);
        }

        let tasks = urls
            .iter()
            .filter(|url| !self.covers.contains_key(url.as_str()))
            .map(|url| {
                let url = url.clone();
                Task::perform(Self::fetch_cover(url.clone()), move |result| match result {
                    Ok(data) => Message::CoverLoaded(url.clone(), data),
                    Err(e) => Message::CoverLoadFailed(url.clone(), e.to_string()),
                })
            })
            .collect_vec();

        if tasks.is_empty() {
            Action::None
        } else {
            Action::Command(Task::batch(tasks))
        }
    }

    // TODO: handle non-HTTP URLs (e.g. file://)?
    async fn fetch_cover(url: String) -> Result<CoverData, reqwest::Error> {
        let response = reqwest::get(url).await?;
        response.bytes().await
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
                    icon(StaticIcon::MusicNote),
                    container(
                        text(self.get_title(&s[0]))
                            .wrapping(text::Wrapping::None)
                            .size(theme.font_size.sm)
                    )
                    .clip(true)
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
