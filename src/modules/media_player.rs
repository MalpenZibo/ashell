use crate::{
    components::divider,
    components::icons::{StaticIcon, icon, icon_button},
    components::{ButtonSize, MenuSize},
    config::{
        MediaPlayerFormat, MediaPlayerMenuField, MediaPlayerModuleConfig, MediaPlayerTextField,
    },
    services::{
        ReadOnlyService, Service, ServiceEvent,
        mpris::{
            MprisPlayerCommand, MprisPlayerData, MprisPlayerMetadata, MprisPlayerService,
            PlaybackStatus, PlayerCommand,
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
                    let metadata = d.metadata.as_ref();
                    let description = self.description_view(metadata, LEFT_COLUMN_WIDTH);

                    let play_pause_icon = match d.state {
                        PlaybackStatus::Playing => StaticIcon::Pause,
                        PlaybackStatus::Paused | PlaybackStatus::Stopped => StaticIcon::Play,
                    };

                    let buttons: Option<Element<'_, _>> = self
                        .config
                        .menu_fields
                        .contains(&MediaPlayerMenuField::Controls)
                        .then(|| {
                            container(
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
                            .center_x(RIGHT_COLUMN_WIDTH)
                            .into()
                        });
                    let volume_slider: Option<Element<'_, _>> = self
                        .config
                        .menu_fields
                        .contains(&MediaPlayerMenuField::Volume)
                        .then(|| {
                            d.volume.map(|v| {
                                slider(0.0..=100.0, v, move |v| {
                                    Message::SetVolume(d.service.clone(), v)
                                })
                                .width(LEFT_COLUMN_WIDTH)
                                .into()
                            })
                        })
                        .flatten();
                    let cover: Option<Element<'_, _>> = self
                        .config
                        .menu_fields
                        .contains(&MediaPlayerMenuField::Cover)
                        .then(|| {
                            d.metadata
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
                                        .unwrap_or_else(|| {
                                            text(t!("media-player-loading-cover")).into()
                                        });
                                    container(inner).center_x(RIGHT_COLUMN_WIDTH).into()
                                })
                        })
                        .flatten();
                    let content: Element<'_, _> = match (description, cover, volume_slider, buttons)
                    {
                        (Some(description), None, None, Some(buttons)) => {
                            row![description, buttons]
                                .spacing(space.md)
                                .align_y(Vertical::Center)
                                .into()
                        }
                        (description, cover, volume_slider, buttons) => {
                            let metadata_row = match (description, cover) {
                                (Some(description), Some(cover)) => Some(
                                    row![description, cover]
                                        .spacing(space.md)
                                        .align_y(Vertical::Center)
                                        .into(),
                                ),
                                (Some(description), None) => Some(row![description].into()),
                                (None, Some(cover)) => Some(
                                    row![space::horizontal().width(LEFT_COLUMN_WIDTH), cover]
                                        .spacing(space.md)
                                        .align_y(Vertical::Center)
                                        .into(),
                                ),
                                (None, None) => None,
                            };
                            let controls_row = match (volume_slider, buttons) {
                                (Some(v), Some(buttons)) => Some(
                                    row![v, buttons]
                                        .spacing(space.md)
                                        .align_y(Vertical::Center)
                                        .into(),
                                ),
                                (Some(v), None) => Some(row![v].into()),
                                (None, Some(buttons)) => Some(
                                    row![space::horizontal().width(LEFT_COLUMN_WIDTH), buttons]
                                        .spacing(space.md)
                                        .align_y(Vertical::Center)
                                        .into(),
                                ),
                                (None, None) => None,
                            };
                            let rows = [metadata_row, controls_row]
                                .into_iter()
                                .flatten()
                                .collect::<Vec<_>>();
                            if rows.is_empty() {
                                space::horizontal().width(Length::Fill).into()
                            } else {
                                column(rows).spacing(space.md).into()
                            }
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

    fn field_value(metadata: &MprisPlayerMetadata, field: MediaPlayerTextField) -> Option<String> {
        match field {
            MediaPlayerTextField::Artist => {
                metadata.artists.as_ref().map(|artists| artists.join(", "))
            }
            MediaPlayerTextField::Title => metadata.title.clone(),
            MediaPlayerTextField::Album => metadata.album.clone(),
        }
    }

    fn format_metadata_fields(
        metadata: Option<&MprisPlayerMetadata>,
        fields: &[MediaPlayerTextField],
    ) -> String {
        metadata.map_or_else(String::new, |metadata| {
            fields
                .iter()
                .filter_map(|field| Self::field_value(metadata, *field))
                .filter(|value| !value.is_empty())
                .collect::<Vec<_>>()
                .join(" - ")
        })
    }

    fn menu_field_text(
        metadata: Option<&MprisPlayerMetadata>,
        field: MediaPlayerMenuField,
    ) -> Option<String> {
        match field {
            MediaPlayerMenuField::Title => Some(
                metadata
                    .and_then(|metadata| metadata.title.clone())
                    .unwrap_or_else(|| t!("media-player-no-title")),
            ),
            MediaPlayerMenuField::Artist => Some(
                metadata
                    .and_then(|metadata| metadata.artists.clone())
                    .map(|artists| artists.join(", "))
                    .unwrap_or_else(|| t!("media-player-unknown-artist")),
            ),
            MediaPlayerMenuField::Album => Some(
                metadata
                    .and_then(|metadata| metadata.album.clone())
                    .unwrap_or_else(|| t!("media-player-unknown-album")),
            ),
            MediaPlayerMenuField::Cover
            | MediaPlayerMenuField::Controls
            | MediaPlayerMenuField::Volume => None,
        }
    }

    fn description_view<'a>(
        &'a self,
        metadata: Option<&'a MprisPlayerMetadata>,
        width: Length,
    ) -> Option<Element<'a, Message>> {
        let font_size = use_theme(|theme| theme.font_size);
        let lines = self
            .config
            .menu_fields
            .iter()
            .filter_map(|field| Self::menu_field_text(metadata, *field))
            .enumerate()
            .map(|(index, value)| {
                let mut line = text(truncate_text(&value, self.config.max_title_length))
                    .wrapping(text::Wrapping::WordOrGlyph)
                    .width(Length::Fill);
                if index > 0 {
                    line = line.size(font_size.sm);
                }
                line.into()
            })
            .collect::<Vec<_>>();

        if lines.is_empty() {
            None
        } else {
            Some(
                column(lines)
                    .spacing(use_theme(|theme| theme.space.xxs))
                    .width(width)
                    .into(),
            )
        }
    }

    fn get_title(&self, d: &MprisPlayerData) -> String {
        let title =
            Self::format_metadata_fields(d.metadata.as_ref(), &self.config.indicator_fields);
        if title.is_empty() {
            t!("media-player-no-title")
        } else {
            truncate_text(&title, self.config.max_title_length)
        }
    }

    pub fn view(&'_ self) -> Option<Element<'_, Message>> {
        let (space, font_size) = use_theme(|theme| (theme.space, theme.font_size));
        self.service.as_ref().and_then(|s| {
            s.players().first().map(|player| {
                let title = || {
                    container(
                        text(self.get_title(player))
                            .wrapping(text::Wrapping::None)
                            .size(font_size.sm),
                    )
                    .clip(true)
                };

                let content = match self.config.indicator_format {
                    MediaPlayerFormat::Icon => row![icon(StaticIcon::MusicNote)],
                    MediaPlayerFormat::Title => row![title()],
                    MediaPlayerFormat::IconAndTitle => row![icon(StaticIcon::MusicNote), title()],
                };

                content.align_y(Vertical::Center).spacing(space.xs).into()
            })
        })
    }

    pub fn subscription(&self) -> Subscription<Message> {
        MprisPlayerService::subscribe().map(Message::Event)
    }
}
