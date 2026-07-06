use crate::{
    components::divider,
    components::icons::{StaticIcon, icon, icon_button},
    components::{ButtonSize, MenuSize},
    config::{MediaPlayerFormat, MediaPlayerModuleConfig},
    services::{
        ReadOnlyService, Service, ServiceEvent,
        mpris::{
            MprisPlayerCommand, MprisPlayerData, MprisPlayerService, PlaybackStatus, PlayerCommand,
        },
    },
    t,
    theme::use_theme,
    utils::truncate_text,
};
use iced::{
    Background, Border, Element, Length, Subscription, Task, Theme,
    alignment::Vertical,
    widget::{column, container, image, row, slider, space, text},
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

    pub fn menu_view<'a>(&'a self) -> Element<'a, Message> {
        let (space, font_size, opacity, radius) =
            use_theme(|theme| (theme.space, theme.font_size, theme.opacity, theme.radius));
        container(match &self.service {
            None => Into::<Element<'a, Message>>::into(text(t!("media-player-not-connected"))),
            Some(service) => column!(
                text(t!("media-player-heading")).size(font_size.lg),
                divider(),
                column(service.players().iter().map(|d| {
                    const LEFT_COLUMN_WIDTH: Length = Length::FillPortion(3);
                    const RIGHT_COLUMN_WIDTH: Length = Length::FillPortion(2);
                    let m = d.metadata.as_ref();
                    let title = m
                        .and_then(|m| m.title.clone())
                        .unwrap_or_else(|| t!("media-player-no-title"));
                    let artists = m
                        .and_then(|m| m.artists.clone())
                        .map(|a| a.join(", "))
                        .unwrap_or_else(|| t!("media-player-unknown-artist"));
                    let album = m
                        .and_then(|m| m.album.clone())
                        .unwrap_or_else(|| t!("media-player-unknown-album"));
                    let title = text(truncate_text(&title, self.config.max_title_length))
                        .wrapping(text::Wrapping::WordOrGlyph)
                        .width(Length::Fill);
                    let artists = text(truncate_text(&artists, self.config.max_title_length))
                        .wrapping(text::Wrapping::WordOrGlyph)
                        .size(font_size.sm)
                        .width(Length::Fill);
                    let album = text(truncate_text(&album, self.config.max_title_length))
                        .wrapping(text::Wrapping::WordOrGlyph)
                        .size(font_size.sm)
                        .width(Length::Fill);
                    let description = column![title, artists, album]
                        .spacing(space.xxs)
                        .width(LEFT_COLUMN_WIDTH);

                    let play_pause_icon = match d.state {
                        PlaybackStatus::Playing => StaticIcon::Pause,
                        PlaybackStatus::Paused | PlaybackStatus::Stopped => StaticIcon::Play,
                    };

                    let buttons = container(
                        row![
                            icon_button(StaticIcon::SkipPrevious)
                                .on_press(Message::Prev(d.service.clone()))
                                .size(ButtonSize::Large),
                            icon_button(play_pause_icon)
                                .on_press(Message::PlayPause(d.service.clone()))
                                .size(ButtonSize::Large),
                            icon_button(StaticIcon::SkipNext)
                                .on_press(Message::Next(d.service.clone()))
                                .size(ButtonSize::Large),
                        ]
                        .align_y(Vertical::Center)
                        .spacing(space.xs),
                    )
                    .center_x(RIGHT_COLUMN_WIDTH);
                    let volume_slider: Option<Element<'_, _>> = d.volume.map(|v| {
                        slider(0.0..=100.0, v, move |v| {
                            Message::SetVolume(d.service.clone(), v)
                        })
                        .width(LEFT_COLUMN_WIDTH)
                        .into()
                    });
                    let cover: Option<Element<'_, _>> = d
                        .metadata
                        .as_ref()
                        .and_then(|m| m.art_url.as_ref())
                        .map(|url| {
                            let inner: Element<'_, _> = service
                                .get_cover(url)
                                .map(|handle| {
                                    image(handle)
                                        .filter_method(image::FilterMethod::Linear)
                                        .into()
                                })
                                .unwrap_or_else(|| text(t!("media-player-loading-cover")).into());
                            container(inner).center_x(RIGHT_COLUMN_WIDTH).into()
                        });
                    let metadata = |description, cover| -> Element<'_, _> {
                        row![description]
                            .push(cover)
                            .spacing(space.md)
                            .align_y(Vertical::Center)
                            .into()
                    };
                    let content: Element<'_, _> = match (volume_slider, cover) {
                        (None, None) => row![description, buttons]
                            .spacing(space.md)
                            .align_y(Vertical::Center)
                            .into(),
                        (Some(v), cover) => {
                            let controls =
                                row![v, buttons].spacing(space.md).align_y(Vertical::Center);
                            column![metadata(description, cover), controls]
                                .spacing(space.md)
                                .into()
                        }
                        (None, cover) => {
                            let controls =
                                row![space::horizontal().width(LEFT_COLUMN_WIDTH), buttons]
                                    .spacing(space.md)
                                    .align_y(Vertical::Center);
                            column![metadata(description, cover), controls]
                                .spacing(space.md)
                                .into()
                        }
                    };
                    container(content)
                        .style(move |app_theme: &Theme| container::Style {
                            background: Background::Color(
                                app_theme
                                    .extended_palette()
                                    .background
                                    .weak
                                    .color
                                    .scale_alpha(opacity),
                            )
                            .into(),
                            border: Border::default().rounded(radius.lg),
                            ..container::Style::default()
                        })
                        .padding(space.md)
                        .width(Length::Fill)
                        .into()
                }))
                .spacing(space.md)
            )
            .spacing(space.xs)
            .into(),
        })
        .width(MenuSize::Large)
        .into()
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
            None => t!("media-player-no-title"),
        }
    }

    pub fn view(&'_ self) -> Option<Element<'_, Message>> {
        let (space, font_size) = use_theme(|theme| (theme.space, theme.font_size));
        self.service.as_ref().and_then(|s| {
            s.players().first().map(|player| {
                let title =
                    (self.config.indicator_format == MediaPlayerFormat::IconAndTitle).then(|| {
                        container(
                            text(self.get_title(player))
                                .wrapping(text::Wrapping::None)
                                .size(font_size.sm),
                        )
                        .clip(true)
                    });

                row![icon(StaticIcon::MusicNote)]
                    .push(title)
                    .align_y(Vertical::Center)
                    .spacing(space.xs)
                    .into()
            })
        })
    }

    pub fn subscription(&self) -> Subscription<Message> {
        MprisPlayerService::subscribe().map(Message::Event)
    }
}
