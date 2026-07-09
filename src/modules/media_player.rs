use crate::{
    components::divider,
    components::icons::{StaticIcon, icon, icon_button},
    components::{ButtonSize, MenuSize},
    config::{MediaPlayerFormat, MediaPlayerModuleConfig, MediaPlayerTextField},
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
    Background, Border, Color, Element, Length, Subscription, Task, Theme,
    alignment::Vertical,
    futures::SinkExt,
    gradient::ColorStop,
    stream::channel,
    widget::{
        Stack,
        canvas::{self, Canvas, Fill, Frame, Geometry, Path},
        column, container, image, row, scrollable, slider, space, text,
    },
};
use std::any::TypeId;

const VISUALIZER_BAR_COUNT: usize = 32;
const VISUALIZER_FRAMERATE: u32 = 60;

const VISUALIZER_BG_BAR_MIN_WIDTH: f32 = 2.0;
const VISUALIZER_BG_BAR_MAX_WIDTH: f32 = 8.0;
const VISUALIZER_BG_BAR_GAP: f32 = 2.0;

struct VisualizerCanvas {
    bars: Vec<f32>,
    low: Color,
    mid: Color,
    high: Color,
    opacity: f32,
    // Corner radius of the surface behind the bars; the outermost bars round
    // their outer corners to match it (0 to keep square bars).
    radius: f32,
    min_bar_width: f32,
    max_bar_width: f32,
    gap: f32,
    inset: f32,
}

impl<M> canvas::Program<M> for VisualizerCanvas {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &iced::Renderer,
        _theme: &iced::Theme,
        bounds: iced::Rectangle,
        _cursor: iced::mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());

        if self.bars.is_empty() {
            return vec![frame.into_geometry()];
        }

        let src = self.bars.len();
        let avail = (bounds.width - 2.0 * self.inset).max(self.min_bar_width);
        // Fewest source bars that keep the width within max, adding thinner bars
        // as space grows (capped at the bars we have). With more space than 32
        // bars can fill at max, bars grow past max; the gap stays fixed.
        let n =
            (((avail + self.gap) / (self.max_bar_width + self.gap)).ceil() as usize).clamp(1, src);
        let bar_width = ((avail - (n - 1) as f32 * self.gap) / n as f32).max(self.min_bar_width);
        let step = bar_width + self.gap;

        // The gradient is vertical, so its colour depends only on `y`: every bar
        // shares the same fill. Build it once instead of per bar.
        let grad = canvas::gradient::Linear::new(
            iced::Point::new(0.0, 0.0),
            iced::Point::new(0.0, bounds.height),
        )
        .add_stops([
            ColorStop {
                offset: 0.0,
                color: self.high.scale_alpha(self.opacity),
            },
            ColorStop {
                offset: 0.5,
                color: self.mid.scale_alpha(self.opacity),
            },
            ColorStop {
                offset: 1.0,
                color: self.low.scale_alpha(self.opacity),
            },
        ]);
        let fill = Fill {
            style: canvas::Style::Gradient(canvas::Gradient::Linear(grad)),
            ..Fill::default()
        };

        for i in 0..n {
            let value = self.bars[(i * src / n).min(src - 1)];
            let height = value * bounds.height;
            let x = self.inset + i as f32 * step;
            let y = bounds.height - height;
            let position = iced::Point::new(x, y);
            let size = iced::Size::new(bar_width, height);

            let is_first = i == 0;
            let is_last = i == n - 1;
            let rect = if self.radius > 0.0 && (is_first || is_last) {
                // Match the outer corners of the first/last bar to the surface
                // rounding; top corners only when the bar reaches the top edge.
                let reaches_top = y <= self.radius;
                let radius = iced::border::Radius {
                    top_left: if is_first && reaches_top {
                        self.radius
                    } else {
                        0.0
                    },
                    bottom_left: if is_first { self.radius } else { 0.0 },
                    top_right: if is_last && reaches_top {
                        self.radius
                    } else {
                        0.0
                    },
                    bottom_right: if is_last { self.radius } else { 0.0 },
                };
                Path::rounded_rectangle(position, size, radius)
            } else {
                Path::rectangle(position, size)
            };

            frame.fill(&rect, fill);
        }

        vec![frame.into_geometry()]
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    Prev(String),
    PlayPause(String),
    Next(String),
    SetVolume(String, f64),
    Event(ServiceEvent<MprisPlayerService>),
    ConfigReloaded(MediaPlayerModuleConfig),
    Bars(Vec<f32>),
}

pub enum Action {
    None,
    Command(Task<Message>),
}

pub struct MediaPlayer {
    config: MediaPlayerModuleConfig,
    service: Option<MprisPlayerService>,
    bars: Vec<f32>,
}

impl MediaPlayer {
    pub fn new(config: MediaPlayerModuleConfig) -> Self {
        Self {
            config,
            service: None,
            bars: Vec::new(),
        }
    }

    /// The player to represent in the bar: the one currently playing, or the
    /// first known player when nothing is playing.
    fn active_player(&self) -> Option<&MprisPlayerData> {
        let players = self.service.as_ref()?.players();
        players
            .iter()
            .find(|p| p.state == PlaybackStatus::Playing)
            .or_else(|| players.first())
    }

    fn is_playing(&self) -> bool {
        self.active_player().map(|p| p.state) == Some(PlaybackStatus::Playing)
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
                    if !self.is_playing() {
                        self.bars.clear();
                    }
                    Action::None
                }
                ServiceEvent::Error(_) => Action::None,
            },
            Message::ConfigReloaded(c) => {
                self.config = c;
                Action::None
            }
            Message::Bars(bars) => {
                self.bars = bars;
                Action::None
            }
        }
    }

    pub fn menu_view<'a>(&'a self, is_closing: bool) -> Element<'a, Message> {
        let (space, font_size, opacity, radius, palette) = use_theme(|theme| {
            (
                theme.space,
                theme.font_size,
                theme.opacity,
                theme.radius,
                theme.iced_theme.palette(),
            )
        });
        container(match &self.service {
            None => Into::<Element<'a, Message>>::into(text(t!("media-player-not-connected"))),
            Some(service) => column!(
                text(t!("media-player-heading")).size(font_size.lg),
                divider(),
                container(scrollable(
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
                        // While the menu is closing, drop the cover: the album art
                        // is an `image` primitive that the menu's clip-reveal close
                        // animation cannot clip, so it would linger on top of the
                        // rolling-up menu.
                        let cover: Option<Element<'_, _>> = (!is_closing)
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
                                .padding(space.md)
                                .align_y(Vertical::Center)
                                .into(),
                            (Some(v), cover) => {
                                let controls =
                                    row![v, buttons].spacing(space.md).align_y(Vertical::Center);
                                column![metadata(description, cover), controls]
                                    .spacing(space.md)
                                    .padding(space.md)
                                    .into()
                            }
                            (None, cover) => {
                                let controls =
                                    row![space::horizontal().width(LEFT_COLUMN_WIDTH), buttons]
                                        .spacing(space.md)
                                        .align_y(Vertical::Center);
                                column![metadata(description, cover), controls]
                                    .spacing(space.md)
                                    .padding(space.md)
                                    .into()
                            }
                        };
                        let card_playing = self.config.show_visualizer
                            && d.state == PlaybackStatus::Playing
                            && !self.bars.is_empty()
                            && !is_closing;
                        let body: Element<'_, _> = if card_playing {
                            Stack::new()
                                .push(content)
                                .push_under(
                                    Canvas::new(VisualizerCanvas {
                                        bars: self.bars.clone(),
                                        low: palette.primary,
                                        mid: palette.warning,
                                        high: palette.danger,
                                        opacity: 0.1,
                                        radius: radius.lg,
                                        min_bar_width: VISUALIZER_BG_BAR_MIN_WIDTH,
                                        max_bar_width: VISUALIZER_BG_BAR_MAX_WIDTH,
                                        gap: VISUALIZER_BG_BAR_GAP,
                                        inset: 0.0,
                                    })
                                    .width(Length::Fill)
                                    .height(Length::Fill),
                                )
                                .into()
                        } else {
                            content
                        };
                        container(body)
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
                            .width(Length::Fill)
                            .into()
                    }))
                    .spacing(space.md)
                ))
                .max_height(600)
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
        let default_fields = [MediaPlayerTextField::Artist, MediaPlayerTextField::Title];
        let fields = if fields.is_empty() {
            &default_fields
        } else {
            fields
        };

        metadata.map_or_else(String::new, |metadata| {
            fields
                .iter()
                .filter_map(|field| Self::field_value(metadata, *field))
                .filter(|value| !value.is_empty())
                .collect::<Vec<_>>()
                .join(" - ")
        })
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
        let (space, font_size, palette) =
            use_theme(|theme| (theme.space, theme.font_size, theme.iced_theme.palette()));
        self.active_player().map(|player| {
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
                MediaPlayerFormat::Text => row![title()],
                MediaPlayerFormat::IconAndText => row![icon(StaticIcon::MusicNote), title()],
            }
                .align_y(Vertical::Center)
                .spacing(space.xs)
                .height(Length::Fill);

            let show_visualizer = self.config.show_visualizer
                && player.state == PlaybackStatus::Playing
                && !self.bars.is_empty();

            if show_visualizer {
                Stack::new()
                    .push(content)
                    .push_under(
                        Canvas::new(VisualizerCanvas {
                            bars: self.bars.clone(),
                            low: palette.primary,
                            mid: palette.warning,
                            high: palette.danger,
                            opacity: 0.1,
                            radius: 0.0,
                            min_bar_width: VISUALIZER_BG_BAR_MIN_WIDTH,
                            max_bar_width: VISUALIZER_BG_BAR_MAX_WIDTH,
                            gap: VISUALIZER_BG_BAR_GAP,
                            inset: space.xxs,
                        })
                        .width(Length::Fill)
                        .height(Length::Fill),
                    )
                    .into()
            } else {
                content.into()
            }
        })
    }

    pub fn subscription(&self) -> Subscription<Message> {
        let cava = (self.config.show_visualizer && self.is_playing())
            .then(|| self.cava_subscription().map(Message::Bars));
        Subscription::batch(
            [
                Some(MprisPlayerService::subscribe().map(Message::Event)),
                cava,
            ]
            .into_iter()
            .flatten(),
        )
    }

    fn cava_subscription(&self) -> Subscription<Vec<f32>> {
        struct Cava;

        Subscription::run_with(TypeId::of::<Cava>(), |_| {
            channel(16, async move |mut output| {
                let cava_config = format!(
                    "[general]\nbars = {VISUALIZER_BAR_COUNT}\nframerate = {VISUALIZER_FRAMERATE}\n\n\
                 [output]\nmethod = raw\nraw_target = /dev/stdout\ndata_format = ascii\nascii_max_range = 1000\n\n\
                 [smoothing]\nmonstercat = 1\n"
                );

                let config_path = std::env::temp_dir().join("ashell_cava.cfg");
                if let Err(e) = tokio::fs::write(&config_path, &cava_config).await {
                    log::error!("cava: failed to write config: {e}");
                    return;
                }

                let mut child = match tokio::process::Command::new("cava")
                    .arg("-p")
                    .arg(&config_path)
                    .stdout(std::process::Stdio::piped())
                    .stderr(std::process::Stdio::piped())
                    .kill_on_drop(true)
                    .spawn()
                {
                    Ok(c) => c,
                    Err(e) => {
                        log::warn!("cava: failed to spawn process: {e}");
                        return;
                    }
                };

                let stdout = match child.stdout.take() {
                    Some(s) => s,
                    None => {
                        log::error!("cava: no stdout");
                        return;
                    }
                };
                let stderr = child.stderr.take();

                use tokio::io::{AsyncBufReadExt, BufReader};
                let reader = BufReader::new(stdout);
                let mut lines = reader.lines();

                while let Ok(Some(line)) = lines.next_line().await {
                    let bars: Vec<f32> = line
                        .split(';')
                        .filter(|s| !s.is_empty())
                        .filter_map(|s| s.trim().parse::<f32>().ok())
                        .map(|v| v / 1000.0)
                        .collect();

                    if !bars.is_empty() {
                        let _ = output.send(bars).await;
                    }
                }

                // stdout closed: cava exited on its own. Surface why, so a rejected
                // config or an incompatible version is not a silent no-op.
                if let Ok(status) = child.wait().await
                    && !status.success()
                {
                    let mut err = String::new();
                    if let Some(mut stderr) = stderr {
                        use tokio::io::AsyncReadExt;
                        let _ = stderr.read_to_string(&mut err).await;
                    }
                    log::warn!("cava exited ({status}): {}", err.trim());
                }
            })
        })
    }
}
