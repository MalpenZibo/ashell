use std::sync::Arc;

use guido::prelude::*;

use crate::components::{ButtonSize, StaticIcon, buttons::icon_button, icon, slider};
use crate::config::{
    MediaPlayerFormat, MediaPlayerModuleConfig, MediaPlayerTextField, MediaPlayerVisualizer,
};
use crate::services::compat::{ServiceSignal, image, run_service};
use crate::services::mpris::{
    MprisPlayerCommand, MprisPlayerMetadata, MprisPlayerService, PlaybackStatus, PlayerCommand,
};
use crate::theme::ThemeColors;
use crate::truncate_text;

pub struct MediaPlayerHandle {
    pub data: ServiceSignal<MprisPlayerService>,
    pub svc: Service<MprisPlayerCommand>,
    /// Latest cava frame, empty while the visualizer is off/silent.
    pub bars: RwSignal<Vec<f32>>,
    /// Turns the cava process on/off (visualizer wanted && playing).
    pub gate: tokio::sync::watch::Sender<bool>,
}

pub fn create() -> MediaPlayerHandle {
    let (data, svc) = run_service::<MprisPlayerService>();
    let bars = create_signal(Vec::new());
    let (gate, gate_rx) = tokio::sync::watch::channel(false);

    let framerate =
        with_context::<crate::config::Config, _>(|c| c.media_player.visualizer_framerate)
            .unwrap_or(30)
            .clamp(1, 144);
    let bars_w = bars.writer();
    let _cava = create_service::<(), _, _>(move |_rx, _ctx| async move {
        run_cava(framerate, gate_rx, bars_w).await;
    });

    MediaPlayerHandle {
        data,
        svc,
        bars,
        gate,
    }
}

// ── cava visualizer ──────────────────────────────────────────────────────────
//
// Mirrors upstream: an external `cava` process in raw-ascii mode, 32 bars,
// frames deduplicated (cava emits at a fixed rate even in silence).

const VISUALIZER_BAR_COUNT: usize = 32;
const BAR_MIN_WIDTH: f32 = 2.0;
const BAR_GAP: f32 = 2.0;
const BG_BAR_MAX_WIDTH: f32 = 8.0;
const BESIDE_BAR_MAX_WIDTH: f32 = 4.0;
const BESIDE_WIDTH: f32 = 64.0;

async fn run_cava(
    framerate: u32,
    mut gate: tokio::sync::watch::Receiver<bool>,
    writer: WriteSignal<Vec<f32>>,
) {
    use tokio::io::AsyncBufReadExt;

    loop {
        // Park until the visualizer is wanted
        while !*gate.borrow() {
            if gate.changed().await.is_err() {
                return;
            }
        }

        let cfg_path = std::env::temp_dir().join("ashell_cava.cfg");
        let cfg = format!(
            "[general]\nbars = {VISUALIZER_BAR_COUNT}\nframerate = {framerate}\n\
             [output]\nmethod = raw\nraw_target = /dev/stdout\ndata_format = ascii\n\
             ascii_max_range = 1000\n[smoothing]\nmonstercat = 1\n"
        );
        if let Err(e) = tokio::fs::write(&cfg_path, cfg).await {
            log::warn!("cava: cannot write config: {e}");
            return;
        }

        let child = tokio::process::Command::new("cava")
            .arg("-p")
            .arg(&cfg_path)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .kill_on_drop(true)
            .spawn();
        let mut child = match child {
            Ok(c) => c,
            Err(e) => {
                log::warn!("cava: failed to spawn (is it installed?): {e}");
                // Don't retry-spin when cava isn't available
                while *gate.borrow() {
                    if gate.changed().await.is_err() {
                        return;
                    }
                }
                continue;
            }
        };
        let stdout = child.stdout.take().expect("cava stdout piped");
        let mut lines = tokio::io::BufReader::new(stdout).lines();
        let mut last_frame: Vec<u8> = Vec::new();

        loop {
            tokio::select! {
                line = lines.next_line() => match line {
                    Ok(Some(line)) => {
                        let bars: Vec<f32> = line
                            .split(';')
                            .filter(|s| !s.is_empty())
                            .filter_map(|s| s.trim().parse::<f32>().ok())
                            .map(|v| (v / 1000.0).clamp(0.0, 1.0))
                            .collect();
                        let quantized: Vec<u8> =
                            bars.iter().map(|v| (v * 255.0) as u8).collect();
                        if quantized != last_frame {
                            last_frame = quantized;
                            writer.set(bars);
                        }
                    }
                    _ => break, // process ended
                },
                res = gate.changed() => {
                    if res.is_err() || !*gate.borrow() {
                        let _ = child.kill().await;
                        break;
                    }
                }
            }
        }
        writer.set(Vec::new());
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }
}

fn with_alpha(c: Color, a: f32) -> Color {
    Color::rgba(c.r, c.g, c.b, a)
}

fn lerp_color(a: Color, b: Color, t: f32) -> Color {
    Color::rgba(
        a.r + (b.r - a.r) * t,
        a.g + (b.g - a.g) * t,
        a.b + (b.b - a.b) * t,
        a.a + (b.a - a.a) * t,
    )
}

/// Sampled bar-top color: primary → warning → danger over the amplitude,
/// approximating upstream's shared three-stop vertical gradient.
fn amplitude_color(theme: &ThemeColors, v: f32) -> Color {
    if v <= 0.5 {
        lerp_color(theme.primary, theme.warning, v * 2.0)
    } else {
        lerp_color(theme.warning, theme.danger, (v - 0.5) * 2.0)
    }
}

/// Row of bottom-aligned gradient bars filling the space it is given; the
/// bar count adapts to the resulting width like upstream's canvas.
fn visualizer_view(bars: RwSignal<Vec<f32>>, opacity: f32, max_bar_width: f32) -> impl Widget {
    let theme = expect_context::<ThemeColors>();
    // Self-measure: the number of bars follows the width the layout ends up
    // giving us, so it can only be known after a layout pass
    let wr = create_widget_ref();

    container()
        .widget_ref(wr)
        .width(fill())
        .height(fill())
        .layout(
            Flex::row()
                .spacing(BAR_GAP)
                .cross_alignment(CrossAlignment::End),
        )
        .children(move || {
            let rect = wr.rect().get();
            let data = bars.get();
            if data.is_empty() || rect.height <= 0.0 {
                return Vec::new();
            }
            let avail = rect.width.max(BAR_MIN_WIDTH);
            let n = (((avail + BAR_GAP) / (max_bar_width + BAR_GAP)).ceil() as usize)
                .clamp(1, VISUALIZER_BAR_COUNT);
            let bar_w = ((avail - (n - 1) as f32 * BAR_GAP) / n as f32).max(BAR_MIN_WIDTH);

            (0..n)
                .map(|i| {
                    let v = data[(i * VISUALIZER_BAR_COUNT / n).min(VISUALIZER_BAR_COUNT - 1)];
                    let top = with_alpha(amplitude_color(&theme, v), opacity);
                    let bottom = with_alpha(theme.primary, opacity);
                    container()
                        .width(bar_w)
                        .height((rect.height * v).max(1.0))
                        .corner_radius(1)
                        .gradient(LinearGradient::vertical(top, bottom))
                })
                .collect()
        })
}

fn field_value(metadata: &MprisPlayerMetadata, field: MediaPlayerTextField) -> Option<String> {
    match field {
        MediaPlayerTextField::Artist => metadata.artists.as_ref().map(|a| a.join(", ")),
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
            .filter_map(|field| field_value(metadata, *field))
            .filter(|value| !value.is_empty())
            .collect::<Vec<_>>()
            .join(" - ")
    })
}

fn cover_source(handle: &image::Handle) -> ImageSource {
    match handle {
        image::Handle::Path(p) => ImageSource::Path(p.clone()),
        image::Handle::Bytes(b) => ImageSource::Bytes(Arc::from(b.as_ref())),
    }
}

/// Bar view: music-note icon and/or "artist - title", representing the
/// playing player (or the first known one). Hidden while no player exists.
pub fn view(
    data: ServiceSignal<MprisPlayerService>,
    bars: RwSignal<Vec<f32>>,
    config: MediaPlayerModuleConfig,
) -> impl Widget {
    let theme = expect_context::<ThemeColors>();
    let format = config.indicator_format;
    let fields = config.indicator_fields.clone();
    let max_len = config.max_text_length;
    let placement = config.indicator_visualizer;

    // (indicator label, playing) for the active player; None hides the module
    let active = create_memo(move || {
        data.with(|s| {
            let s = s.as_ref()?;
            let players = s.players();
            let p = players
                .iter()
                .find(|p| p.state == PlaybackStatus::Playing)
                .or_else(|| players.first())?;
            let value = format_metadata_fields(p.metadata.as_ref(), &fields);
            let label = if value.is_empty() {
                "No title".to_string()
            } else {
                truncate_text(&value, max_len)
            };
            Some((label, p.state == PlaybackStatus::Playing))
        })
    });

    // bool memo so per-frame cava updates don't rebuild the whole module
    let viz_active = create_memo(move || {
        active.get().is_some_and(|(_, playing)| playing) && bars.with(|b| !b.is_empty())
    });

    container().child(move || {
        active.get().map(|(label, _)| {
            let mut row = container().height(fill()).layout(
                Flex::row()
                    .spacing(4)
                    .cross_alignment(CrossAlignment::Center),
            );
            if format != MediaPlayerFormat::Text {
                row = row.child(icon().kind(StaticIcon::MusicNote).color(theme.text));
            }
            if format != MediaPlayerFormat::Icon {
                row = row.child(
                    container()
                        .overflow(Overflow::Hidden)
                        .child(text(label).color(theme.text).font_size(12).nowrap()),
                );
            }

            let beside = || {
                container()
                    .width(BESIDE_WIDTH)
                    .height(fill())
                    .padding([2, 0])
                    .child(visualizer_view(bars, 1.0, BESIDE_BAR_MAX_WIDTH))
            };
            let side_row = |a: AnyWidget, b: AnyWidget| {
                container()
                    .height(fill())
                    .layout(
                        Flex::row()
                            .spacing(4)
                            .cross_alignment(CrossAlignment::Center),
                    )
                    .child(a)
                    .child(b)
            };

            match placement {
                Some(MediaPlayerVisualizer::Background) if viz_active.get() => {
                    // Icon-only content gets a fixed runway for the bars
                    let content: AnyWidget = if format == MediaPlayerFormat::Icon {
                        container()
                            .width(BESIDE_WIDTH)
                            .height(fill())
                            .layout(
                                Flex::row()
                                    .cross_alignment(CrossAlignment::Center)
                                    .main_alignment(MainAlignment::Center),
                            )
                            .child(row)
                            .into_any()
                    } else {
                        row.into_any()
                    };
                    container()
                        .height(fill())
                        .layout(ZStack::new())
                        // Follows both axes: as wide as the content below it
                        .child(
                            container()
                                .width(fill())
                                .height(fill())
                                .padding([2, 0])
                                .child(visualizer_view(bars, 0.35, BG_BAR_MAX_WIDTH)),
                        )
                        // Fills the bar height, leads the width
                        .child(container().height(fill()).child(content))
                        .into_any()
                }
                Some(MediaPlayerVisualizer::Before) if viz_active.get() => {
                    side_row(beside().into_any(), row.into_any()).into_any()
                }
                Some(MediaPlayerVisualizer::After) if viz_active.get() => {
                    side_row(row.into_any(), beside().into_any()).into_any()
                }
                _ => row.into_any(),
            }
        })
    })
}

/// Menu: one card per player with metadata, cover art, transport controls
/// and an optional volume slider.
pub fn menu_view(
    data: ServiceSignal<MprisPlayerService>,
    svc: Service<MprisPlayerCommand>,
    bars: RwSignal<Vec<f32>>,
    config: MediaPlayerModuleConfig,
) -> impl Widget {
    let theme = expect_context::<ThemeColors>();
    let max_len = config.max_text_length;
    let menu_visualizer = config.menu_visualizer;
    let has_bars = create_memo(move || bars.with(|b| !b.is_empty()));

    container()
        .width(fill())
        .layout(Flex::column().spacing(8))
        .child(text("Media player").color(theme.text).font_size(16))
        .child(crate::components::divider())
        .child(move || {
            struct Card {
                service: String,
                title: String,
                artists: String,
                album: String,
                playing: bool,
                volume: Option<f64>,
                cover: Option<Option<ImageSource>>,
            }

            let cards: Vec<Card> = data.with(|s| {
                let Some(s) = s.as_ref() else {
                    return Vec::new();
                };
                s.players()
                    .iter()
                    .map(|d| {
                        let m = d.metadata.as_ref();
                        Card {
                            service: d.service.clone(),
                            title: m
                                .and_then(|m| m.title.clone())
                                .unwrap_or_else(|| "No title".to_string()),
                            artists: m
                                .and_then(|m| m.artists.clone())
                                .map(|a| a.join(", "))
                                .unwrap_or_else(|| "Unknown artist".to_string()),
                            album: m
                                .and_then(|m| m.album.clone())
                                .unwrap_or_else(|| "Unknown album".to_string()),
                            playing: d.state == PlaybackStatus::Playing,
                            volume: d.volume,
                            // Outer None: no art url; inner None: still loading
                            cover: m
                                .and_then(|m| m.art_url.as_ref())
                                .map(|url| s.get_cover(url).map(cover_source)),
                        }
                    })
                    .collect()
            });

            if cards.is_empty() {
                return Some(
                    container()
                        .width(fill())
                        .padding(8)
                        .child(text("Not connected").color(theme.text).font_size(14))
                        .into_any(),
                );
            }

            let mut col = container()
                .width(fill())
                .height(at_most(600))
                .scrollable(ScrollAxis::Vertical)
                .layout(Flex::column().spacing(8));

            for card in cards {
                let svc_prev = svc.clone();
                let svc_play = svc.clone();
                let svc_next = svc.clone();
                let svc_vol = svc.clone();
                let (s1, s2, s3, s4) = (
                    card.service.clone(),
                    card.service.clone(),
                    card.service.clone(),
                    card.service.clone(),
                );

                let description = container()
                    .width(fill())
                    .layout(Flex::column().spacing(2))
                    .child(
                        text(truncate_text(&card.title, max_len))
                            .color(theme.text)
                            .font_size(14),
                    )
                    .child(
                        text(truncate_text(&card.artists, max_len))
                            .color(theme.text)
                            .font_size(12),
                    )
                    .child(
                        text(truncate_text(&card.album, max_len))
                            .color(theme.text)
                            .font_size(12),
                    );

                let cover: Option<AnyWidget> = card.cover.map(|c| match c {
                    Some(src) => image(src).width(120).height(120).into_any(),
                    None => text("Loading cover...")
                        .color(theme.text)
                        .font_size(12)
                        .into_any(),
                });

                let buttons = container()
                    .layout(
                        Flex::row()
                            .spacing(4)
                            .cross_alignment(CrossAlignment::Center),
                    )
                    .child(
                        icon_button()
                            .icon(StaticIcon::SkipPrevious)
                            .size(ButtonSize::Large)
                            .on_click(move || {
                                svc_prev.send(MprisPlayerCommand {
                                    service_name: s1.clone(),
                                    command: PlayerCommand::Prev,
                                });
                            }),
                    )
                    .child(
                        icon_button()
                            .icon(if card.playing {
                                StaticIcon::Pause
                            } else {
                                StaticIcon::Play
                            })
                            .size(ButtonSize::Large)
                            .on_click(move || {
                                svc_play.send(MprisPlayerCommand {
                                    service_name: s2.clone(),
                                    command: PlayerCommand::PlayPause,
                                });
                            }),
                    )
                    .child(
                        icon_button()
                            .icon(StaticIcon::SkipNext)
                            .size(ButtonSize::Large)
                            .on_click(move || {
                                svc_next.send(MprisPlayerCommand {
                                    service_name: s3.clone(),
                                    command: PlayerCommand::Next,
                                });
                            }),
                    );

                let metadata_row = container()
                    .width(fill())
                    .layout(
                        Flex::row()
                            .spacing(12)
                            .cross_alignment(CrossAlignment::Center),
                    )
                    .child(description)
                    .maybe_child(cover);

                let controls = container()
                    .width(fill())
                    .layout(
                        Flex::row()
                            .spacing(12)
                            .cross_alignment(CrossAlignment::Center)
                            .main_alignment(MainAlignment::SpaceBetween),
                    )
                    .maybe_child(card.volume.map(|v| {
                        container().width(fill()).child(
                            slider()
                                .value(v as i32)
                                .kind(StaticIcon::Speaker3)
                                .muted(false)
                                .on_change(move |vol| {
                                    svc_vol.send(MprisPlayerCommand {
                                        service_name: s4.clone(),
                                        command: PlayerCommand::Volume(vol as f64),
                                    });
                                }),
                        )
                    }))
                    .child(buttons);

                let card_body = container()
                    .width(fill())
                    .padding(12)
                    .corner_radius(16)
                    .background(theme.background.lighter(0.05))
                    .layout(Flex::column().spacing(12))
                    .child(metadata_row)
                    .child(controls);

                col = col.child(if menu_visualizer && card.playing && has_bars.get() {
                    container()
                        .width(fill())
                        .layout(ZStack::new())
                        // Follows the card body behind it
                        .child(
                            container()
                                .width(fill())
                                .height(fill())
                                .corner_radius(16)
                                .overflow(Overflow::Hidden)
                                .child(visualizer_view(bars, 0.25, BG_BAR_MAX_WIDTH)),
                        )
                        .child(card_body)
                        .into_any()
                } else {
                    card_body.into_any()
                });
            }

            Some(col.into_any())
        })
}
