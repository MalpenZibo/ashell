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
    Background, Border, Color, Element, Length, Subscription, Task, Theme,
    alignment::Vertical,
    futures::SinkExt,
    gradient::ColorStop,
    stream::channel,
    widget::{
        Stack,
        canvas::{self, Canvas, Fill, Frame, Geometry, Path},
        column, container, image, row, slider, space, text,
    },
};
use std::any::TypeId;

const VISUALIZER_BAR_COUNT: usize = 12;
const VISUALIZER_FRAMERATE: u32 = 60;

struct VisualizerCanvas {
    bars: Vec<f32>,
    low: Color,
    mid: Color,
    high: Color,
    opacity: f32,
    // Corner radius of the surface behind the bars; the outermost bars round
    // their outer corners to match it (0 to keep square bars).
    radius: f32,
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

        let n = VISUALIZER_BAR_COUNT.min(self.bars.len());
        // Fill the full width edge to edge: `n` bars plus `n - 1` gaps span the
        // whole width, with each gap a third of a bar.
        let bar_width = (bounds.width / (n as f32 + (n as f32 - 1.0) / 3.0)).max(1.0);
        let gap = bar_width / 3.0;
        let step = bar_width + gap;

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
            let height = self.bars[i] * bounds.height;
            let x = i as f32 * step;
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

    fn is_playing(&self) -> bool {
        self.service
            .as_ref()
            .and_then(|s| s.players().first().map(|p| p.state))
            == Some(PlaybackStatus::Playing)
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

    pub fn menu_view<'a>(&'a self) -> Element<'a, Message> {
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
                        && !self.bars.is_empty();
                    let body: Element<'_, _> = if card_playing {
                        Stack::new()
                            .push(content)
                            .push_under(
                                Canvas::new(VisualizerCanvas {
                                    bars: self.bars.clone(),
                                    low: palette.primary,
                                    mid: palette.warning,
                                    high: palette.danger,
                                    opacity: 0.2,
                                    radius: radius.lg,
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
        let (space, font_size, palette) =
            use_theme(|theme| (theme.space, theme.font_size, theme.iced_theme.palette()));
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

                let visualizer = (self.config.show_visualizer
                    && player.state == PlaybackStatus::Playing
                    && !self.bars.is_empty())
                .then(|| {
                    container(
                        Canvas::new(VisualizerCanvas {
                            bars: self.bars.clone(),
                            low: palette.primary,
                            mid: palette.warning,
                            high: palette.danger,
                            opacity: 1.,
                            radius: 0.0,
                        })
                        .width(Length::Fixed((VISUALIZER_BAR_COUNT * 4) as f32))
                        .height(Length::Fill),
                    )
                    .padding(space.xxs)
                });

                row![icon(StaticIcon::MusicNote)]
                    .push(title)
                    .push(visualizer)
                    .align_y(Vertical::Center)
                    .spacing(space.xs)
                    .into()
            })
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
