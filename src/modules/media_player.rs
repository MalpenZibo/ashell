use crate::{
    components::icons::{IconButtonSize, StaticIcon, icon, icon_button},
    config::{
        MediaPlayerFormat, MediaPlayerModuleConfig, VisualizerChannels, VisualizerColor,
        VisualizerMonoOption,
    },
    menu::MenuSize,
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
    Background, Border, Color, Element, Length, Subscription, Task, Theme,
    alignment::Vertical,
    futures::SinkExt,
    stream::channel,
    widget::{
        canvas::{self, Canvas, Fill, Frame, Geometry, Path},
        column, container, horizontal_rule, horizontal_space, image, row, slider, text,
    },
};
use std::collections::{HashMap, HashSet};

fn cava_subscription<M, F>(
    id: impl std::hash::Hash + 'static,
    bar_count: u32,
    framerate: u32,
    channels: VisualizerChannels,
    mono_option: VisualizerMonoOption,
    make_msg: F,
) -> Subscription<M>
where
    M: Send + 'static,
    F: Fn(Vec<f32>) -> M + Send + 'static,
{
    Subscription::run_with_id(
        id,
        channel(16, async move |mut output| {
            let mono_opt = match mono_option {
                VisualizerMonoOption::Average => "average",
                VisualizerMonoOption::Left => "left",
                VisualizerMonoOption::Right => "right",
            };
            let channels_str = match channels {
                VisualizerChannels::Stereo => "stereo",
                VisualizerChannels::Mono => "mono",
            };
            let cava_config = format!(
                "[general]\nbars = {bar_count}\nframerate = {framerate}\n\n\
                 [output]\nmethod = raw\nraw_target = /dev/stdout\ndata_format = ascii\nascii_max_range = 1000\n\
                 channels = {channels_str}\nmono_option = {mono_opt}\n\n\
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
                .stderr(std::process::Stdio::null())
                .spawn()
            {
                Ok(c) => c,
                Err(e) => {
                    log::error!("cava: failed to spawn process: {e}");
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
                    let _ = output.send(make_msg(bars)).await;
                }
            }

            let _ = child.kill().await;
        }),
    )
}

enum VisualizerFill {
    Flat(Color),
    Gradient {
        low: Color,
        mid: Option<Color>,
        high: Color,
    },
}

struct VisualizerCanvas {
    bars: Vec<f32>,
    bar_count: usize,
    fill: VisualizerFill,
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

        let n = self.bar_count.min(self.bars.len());
        let bar_width = (bounds.width / (n as f32 * 4.0 / 3.0)).max(1.0);
        let gap = (bar_width / 3.0).max(1.0);
        let step = bar_width + gap;

        for i in 0..n {
            let height = self.bars[i] * bounds.height;
            let x = i as f32 * step;
            let y = bounds.height - height;

            let rect = Path::rectangle(iced::Point::new(x, y), iced::Size::new(bar_width, height));

            let fill: Fill = match &self.fill {
                VisualizerFill::Flat(color) => (*color).into(),
                VisualizerFill::Gradient { low, mid, high } => {
                    use iced::gradient::ColorStop;
                    let stops: &[ColorStop] = match mid {
                        Some(mid) => &[
                            ColorStop {
                                offset: 0.0,
                                color: *high,
                            },
                            ColorStop {
                                offset: 0.5,
                                color: *mid,
                            },
                            ColorStop {
                                offset: 1.0,
                                color: *low,
                            },
                        ],
                        None => &[
                            ColorStop {
                                offset: 0.0,
                                color: *high,
                            },
                            ColorStop {
                                offset: 1.0,
                                color: *low,
                            },
                        ],
                    };
                    let grad = canvas::gradient::Linear::new(
                        iced::Point::new(x, 0.0),
                        iced::Point::new(x, bounds.height),
                    )
                    .add_stops(stops.iter().copied());
                    Fill {
                        style: canvas::Style::Gradient(canvas::Gradient::Linear(grad)),
                        ..Fill::default()
                    }
                }
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
    covers: HashMap<String, image::Handle>,
    bars: Vec<f32>,
}

impl MediaPlayer {
    pub fn new(config: MediaPlayerModuleConfig) -> Self {
        Self {
            config,
            service: None,
            covers: HashMap::new(),
            bars: vec![],
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
                    let is_playing = self
                        .service
                        .as_ref()
                        .and_then(|s| {
                            s.players()
                                .first()
                                .map(|p| p.state == PlaybackStatus::Playing)
                        })
                        .unwrap_or(false);
                    if !is_playing {
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

    pub fn menu_view<'a>(&'a self, theme: &'a AshellTheme) -> Element<'a, Message> {
        container(match &self.service {
            None => Into::<Element<'a, Message>>::into(text("Not connected to MPRIS service")),
            Some(service) => column!(
                text("Players").size(theme.font_size.lg),
                horizontal_rule(1),
                column(service.players().iter().map(|d| {
                    const LEFT_COLUMN_WIDTH: Length = Length::FillPortion(3);
                    const RIGHT_COLUMN_WIDTH: Length = Length::FillPortion(2);
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
                        .width(LEFT_COLUMN_WIDTH);

                    let play_pause_icon = match d.state {
                        PlaybackStatus::Playing => StaticIcon::Pause,
                        PlaybackStatus::Paused | PlaybackStatus::Stopped => StaticIcon::Play,
                    };

                    let buttons = container(
                        row![
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
                        .spacing(theme.space.xs),
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
                            let inner: Element<'_, _> = self
                                .covers
                                .get(url)
                                .map(|handle| {
                                    image(handle)
                                        .filter_method(image::FilterMethod::Linear)
                                        .into()
                                })
                                .unwrap_or_else(|| text("Loading cover...").into());
                            container(inner).center_x(RIGHT_COLUMN_WIDTH).into()
                        });
                    let metadata = |description, cover| -> Element<'_, _> {
                        row![description]
                            .push_maybe(cover)
                            .spacing(theme.space.md)
                            .align_y(Vertical::Center)
                            .into()
                    };
                    let content: Element<'_, _> = match (volume_slider, cover) {
                        (None, None) => row![description, buttons]
                            .spacing(theme.space.md)
                            .align_y(Vertical::Center)
                            .into(),
                        (Some(v), cover) => {
                            let controls = row![v, buttons]
                                .spacing(theme.space.md)
                                .align_y(Vertical::Center);
                            column![metadata(description, cover), controls]
                                .spacing(theme.space.md)
                                .into()
                        }
                        (None, cover) => {
                            let controls =
                                row![horizontal_space().width(LEFT_COLUMN_WIDTH), buttons]
                                    .spacing(theme.space.md)
                                    .align_y(Vertical::Center);
                            column![metadata(description, cover), controls]
                                .spacing(theme.space.md)
                                .into()
                        }
                    };
                    container(content)
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
        })
        .max_width(MenuSize::Large)
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

                let visualizer = (self.config.show_visualizer
                    && player.state == PlaybackStatus::Playing
                    && !self.bars.is_empty())
                .then(|| {
                    let bar_count = self.config.visualizer_bar_count as usize;
                    let padding = self.config.visualizer_padding;
                    let palette = theme.get_theme().palette();
                    let fill = match &self.config.visualizer_color {
                        VisualizerColor::Text => VisualizerFill::Flat(palette.text),
                        VisualizerColor::Primary => VisualizerFill::Flat(palette.primary),
                        VisualizerColor::Success => VisualizerFill::Flat(palette.success),
                        VisualizerColor::Danger => VisualizerFill::Flat(palette.danger),
                        VisualizerColor::Hex(h) => {
                            VisualizerFill::Flat(Color::from_rgb8(h.r, h.g, h.b))
                        }
                        VisualizerColor::Gradient { low, mid, high } => VisualizerFill::Gradient {
                            low: Color::from_rgb8(low.r, low.g, low.b),
                            mid: mid.map(|m| Color::from_rgb8(m.r, m.g, m.b)),
                            high: Color::from_rgb8(high.r, high.g, high.b),
                        },
                    };
                    container(
                        Canvas::new(VisualizerCanvas {
                            bars: self.bars.clone(),
                            bar_count,
                            fill,
                        })
                        .width(Length::Fixed((bar_count * 4).max(20) as f32))
                        .height(Length::Fill),
                    )
                    .padding([padding, 0])
                    .height(Length::Fill)
                });

                row![icon(StaticIcon::MusicNote)]
                    .push_maybe(title)
                    .push_maybe(visualizer)
                    .align_y(Vertical::Center)
                    .spacing(theme.space.xs)
                    .into()
            })
        })
    }

    pub fn subscription(&self) -> Subscription<Message> {
        let cava = self.config.show_visualizer.then(|| {
            cava_subscription(
                (
                    "media_player_cava",
                    self.config.visualizer_bar_count,
                    self.config.visualizer_framerate,
                    self.config.visualizer_channels,
                    self.config.visualizer_mono_option,
                ),
                self.config.visualizer_bar_count,
                self.config.visualizer_framerate,
                self.config.visualizer_channels,
                self.config.visualizer_mono_option,
                Message::Bars,
            )
        });
        Subscription::batch(
            [
                Some(MprisPlayerService::subscribe().map(Message::Event)),
                cava,
            ]
            .into_iter()
            .flatten(),
        )
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
