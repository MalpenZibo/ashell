use std::collections::HashMap;

use guido::prelude::*;

use crate::config::{Config, WorkspacesModuleConfig};
use crate::services::compositor::{
    CompositorCommand, CompositorMonitor, CompositorStateSignals, CompositorWorkspace,
};
use crate::theme::ThemeColors;

const PILL_HEIGHT: f32 = 16.0;
const PILL_ACTIVE_WIDTH: f32 = 32.0;
const PILL_VISIBLE_WIDTH: f32 = 24.0;
const PILL_HIDDEN_WIDTH: f32 = 16.0;
const PILL_CORNER_RADIUS: f32 = 8.0;

/// base/weak/strong/text variants of a workspace (or background) color —
/// the port of upstream's generated palette pairs. Explicit values from an
/// `AppearanceColor::Complete` win; the rest is derived.
#[derive(Debug, Clone, Copy, PartialEq)]
struct ColorPair {
    base: Color,
    weak: Color,
    strong: Color,
    text: Color,
}

fn lerp_color(a: Color, b: Color, t: f32) -> Color {
    Color::rgba(
        a.r + (b.r - a.r) * t,
        a.g + (b.g - a.g) * t,
        a.b + (b.b - a.b) * t,
        a.a + (b.a - a.a) * t,
    )
}

/// Contrast-picked text color for a background.
fn readable_on(bg: Color, light: Color, dark: Color) -> Color {
    let luma = 0.299 * bg.r + 0.587 * bg.g + 0.114 * bg.b;
    if luma > 0.5 { dark } else { light }
}

fn pair_from(
    ac: &crate::config::AppearanceColor,
    surface: Color,
    theme: &ThemeColors,
) -> ColorPair {
    let base = ac.base();
    let weak = ac.weak().unwrap_or_else(|| lerp_color(base, surface, 0.65));
    ColorPair {
        base,
        weak,
        strong: ac.strong().unwrap_or_else(|| base.lighter(0.1)),
        text: ac
            .text()
            .unwrap_or_else(|| readable_on(base, theme.text, surface)),
    }
}

/// The background pairs (used by empty pills).
fn background_pair() -> ColorPair {
    let theme = expect_context::<ThemeColors>();
    with_context::<Config, _>(|c| {
        pair_from(&c.appearance.background_color, theme.background, &theme)
    })
    .unwrap()
}

fn workspace_pairs() -> Vec<ColorPair> {
    let theme = expect_context::<ThemeColors>();
    let ws = with_context::<Config, _>(|c| {
        c.appearance
            .workspace_colors
            .iter()
            .map(|ac| pair_from(ac, theme.background, &theme))
            .collect::<Vec<_>>()
    })
    .unwrap();
    if ws.is_empty() {
        vec![pair_from(
            &crate::config::AppearanceColor::Simple(hex_color::HexColor::rgb(122, 162, 247)),
            theme.background,
            &theme,
        )]
    } else {
        ws
    }
}

/// Pairs for special workspaces; falls back to the normal list like upstream.
fn special_workspace_pairs() -> Vec<ColorPair> {
    let theme = expect_context::<ThemeColors>();
    with_context::<Config, _>(|c| {
        c.appearance.special_workspace_colors.as_ref().map(|l| {
            l.iter()
                .map(|ac| pair_from(ac, theme.background, &theme))
                .collect::<Vec<_>>()
        })
    })
    .unwrap()
    .filter(|l| !l.is_empty())
    .unwrap_or_else(workspace_pairs)
}

// ── Display state ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Displayed {
    Active,
    Visible,
    Hidden,
}

impl Displayed {
    fn width(self) -> Length {
        match self {
            Displayed::Active => PILL_ACTIVE_WIDTH.into(),
            Displayed::Visible => PILL_VISIBLE_WIDTH.into(),
            Displayed::Hidden => PILL_HIDDEN_WIDTH.into(),
        }
    }
}

// ── UI workspace (computed from raw compositor state) ────────────────────────

/// Structural facts only — volatile state (active/visible, colors) is read
/// via per-pill memos, so a pill is rebuilt only when these fields change.
#[derive(Debug, Clone, PartialEq)]
struct UiWorkspace {
    id: i32,
    index: i32,
    name: String,
    monitor: String,
    is_special: bool,
}

/// Build the UI workspace list from raw compositor state.
/// Ports ashell's `calculate_ui_workspaces`: workspace filling, special
/// workspaces, virtual desktops, custom names, sorting.
fn calculate_ui_workspaces(
    config: &WorkspacesModuleConfig,
    workspaces: &[CompositorWorkspace],
    monitors: &[CompositorMonitor],
) -> Vec<UiWorkspace> {
    let monitor_order: HashMap<&str, usize> = monitors
        .iter()
        .enumerate()
        .map(|(idx, m)| (m.name.as_str(), idx))
        .collect();

    // Deduplicate by id
    let mut seen = std::collections::HashSet::new();
    let deduped: Vec<_> = workspaces.iter().filter(|w| seen.insert(w.id)).collect();

    let (special, normal): (Vec<_>, Vec<_>) = deduped.into_iter().partition(|w| w.is_special);

    let mut result: Vec<UiWorkspace> = Vec::new();

    // Special workspaces
    if !config.disable_special_workspaces {
        for w in &special {
            result.push(UiWorkspace {
                id: w.id,
                index: w.index,
                name: w.name.split(':').next_back().unwrap_or("").to_owned(),

                monitor: w.monitor.clone(),
                is_special: true,
            });
        }
    }

    // Normal workspaces (or virtual desktops)
    if config.enable_virtual_desktops {
        let monitor_count = monitors.len().max(1) as i32;
        let mut vdesks: HashMap<i32, u16> = HashMap::new();

        for w in &normal {
            let vdesk_id = ((w.id - 1) / monitor_count) + 1;
            *vdesks.entry(vdesk_id).or_insert(0) += w.windows;
        }

        for (&id, &_windows) in &vdesks {
            let idx = (id - 1) as usize;
            let name = config
                .workspace_names
                .get(idx)
                .cloned()
                .unwrap_or_else(|| id.to_string());
            result.push(UiWorkspace {
                id,
                index: id,
                name,

                monitor: String::new(),
                is_special: false,
            });
        }
    } else {
        for w in &normal {
            let name = if w.id > 0 {
                let idx = (w.id - 1) as usize;
                config
                    .workspace_names
                    .get(idx)
                    .cloned()
                    .unwrap_or_else(|| w.name.clone())
            } else {
                w.name.clone()
            };
            result.push(UiWorkspace {
                id: w.id,
                index: w.index,
                name,

                monitor: w.monitor.clone(),
                is_special: false,
            });
        }
    }

    // Workspace filling: add phantom workspaces for missing IDs
    if config.enable_workspace_filling && !result.is_empty() {
        // Fill by INDEX like upstream — on niri ids are arbitrary growing
        // numbers while the index is the 1-based position, so filling by id
        // fabricates phantom pills next to real workspaces
        let existing_indices: Vec<i32> = result.iter().map(|w| w.index).collect();
        let existing_ids: Vec<i32> = result.iter().map(|w| w.id).collect();
        let mut max_index = existing_indices
            .iter()
            .filter(|&&idx| idx > 0)
            .max()
            .copied()
            .unwrap_or(0);

        if let Some(max_cfg) = config.max_workspaces
            && max_cfg as i32 > max_index
        {
            max_index = max_cfg as i32;
        }

        for index in 1..=max_index {
            if !existing_indices.contains(&index) {
                let name = config
                    .workspace_names
                    .get((index - 1) as usize)
                    .cloned()
                    .unwrap_or_else(|| index.to_string());
                // Upstream reuses the index as id; keep keyed() rows unique
                // when a real workspace already owns that numeric id
                let id = if existing_ids.contains(&index) {
                    -(100_000 + index)
                } else {
                    index
                };
                result.push(UiWorkspace {
                    id,
                    index,
                    name,

                    monitor: String::new(),
                    is_special: false,
                });
            }
        }
    }

    // Sort
    if config.group_by_monitor {
        result.sort_by(|a, b| {
            let a_ord = monitor_order
                .get(a.monitor.as_str())
                .copied()
                .unwrap_or(usize::MAX);
            let b_ord = monitor_order
                .get(b.monitor.as_str())
                .copied()
                .unwrap_or(usize::MAX);
            a_ord
                .cmp(&b_ord)
                .then(a.index.cmp(&b.index))
                .then(a.id.cmp(&b.id))
        });
    } else {
        result.sort_by(|a, b| a.index.cmp(&b.index).then(a.id.cmp(&b.id)));
    }

    result
}

// ── Pill styling ─────────────────────────────────────────────────────────────
//
// Matches ashell's `workspace_button_style` exactly:
// - `ws_color`: Some(color) when the workspace has a monitor color assignment,
//   None for phantom/unassigned workspaces.
// - `displayed` only affects WIDTH, not colors.
// - Empty pills: surface background, 1px border in ws_color (invisible when None).
// - Occupied pills: ws_color background (surface when None), no border.

/// Resolve the workspace color pair. Returns None for workspaces without a
/// monitor (phantom/filled workspaces). An index past the end of the list
/// falls back to the theme primary pair, it does not wrap.
fn resolve_ws_pair(
    pairs: &[ColorPair],
    monitor_id: Option<i128>,
    fallback: ColorPair,
) -> Option<ColorPair> {
    monitor_id.map(|mid| {
        pairs
            .get(mid.unsigned_abs() as usize)
            .copied()
            .unwrap_or(fallback)
    })
}

fn compute_displayed(
    active_ws_id: Option<i32>,
    monitors: &[CompositorMonitor],
    ws_id: i32,
) -> Displayed {
    let is_active = active_ws_id == Some(ws_id);
    let is_visible = monitors.iter().any(|m| m.active_workspace_id == ws_id);
    match (is_active, is_visible) {
        (true, _) => Displayed::Active,
        (false, true) => Displayed::Visible,
        (false, false) => Displayed::Hidden,
    }
}

fn is_empty(workspaces: &[CompositorWorkspace], ws_id: i32) -> bool {
    workspaces
        .iter()
        .find(|w| w.id == ws_id)
        .is_none_or(|w| w.windows == 0)
}

// Upstream `workspace_button_style` matrix (minus the urgent branch, which
// waits on compositor urgency plumbing). `ws` None means "no monitor
// assignment": background pairs stand in for the whole palette.
fn pill_background(bg: ColorPair, ws: Option<ColorPair>, active: bool, empty: bool) -> Color {
    match (empty, active) {
        (true, true) => bg.strong,
        (true, false) => bg.weak,
        (false, true) => ws.map(|p| p.base).unwrap_or(bg.strong),
        (false, false) => ws.map(|p| p.weak).unwrap_or(bg.weak),
    }
}

fn pill_border_width(empty: bool) -> f32 {
    if empty { 1.0 } else { 0.0 }
}

fn pill_border_color(bg: ColorPair, ws: Option<ColorPair>, active: bool) -> Color {
    let ws = ws.unwrap_or(bg);
    if active { ws.base } else { ws.weak }
}

fn pill_text_color(theme: ThemeColors, ws: Option<ColorPair>, active: bool, empty: bool) -> Color {
    if empty {
        theme.text
    } else if active {
        ws.map(|p| p.text).unwrap_or(theme.text)
    } else {
        // On the dim weak background the normal text color reads fine
        theme.text
    }
}

// ── View ─────────────────────────────────────────────────────────────────────

pub fn view(state: CompositorStateSignals, svc: Service<CompositorCommand>) -> impl Widget {
    let theme = expect_context::<ThemeColors>();
    let config = with_context::<Config, _>(|c| c.workspaces.clone()).unwrap();
    let colors = workspace_pairs();
    let bg_pair = background_pair();
    let enable_vdesks = config.enable_virtual_desktops;

    let svc_scroll = svc;
    let svc_children = svc;

    let workspaces = state.workspaces;
    let monitors = state.monitors;
    let active_ws_id = state.active_workspace_id;

    container()
        .layout(
            Flex::row()
                .spacing(4)
                .cross_alignment(CrossAlignment::Center),
        )
        .on_scroll({
            let config = config.clone();
            let accum = std::cell::Cell::new(0.0f32);
            move |_dx, dy, source| {
                use crate::config::InvertScrollDirection as Inv;
                // Per-source inversion, then a small accumulator so trackpads
                // don't switch on every pixel event (upstream: sensibility 3)
                let inverted = match source {
                    ScrollSource::Wheel => {
                        matches!(config.invert_scroll_direction, Some(Inv::All | Inv::Mouse))
                    }
                    _ => matches!(
                        config.invert_scroll_direction,
                        Some(Inv::All | Inv::Trackpad)
                    ),
                };
                let dy = if inverted { -dy } else { dy };
                let acc = accum.get() + dy;
                if acc.abs() < 3.0 {
                    accum.set(acc);
                    return;
                }
                accum.set(0.0);
                let up = acc > 0.0;

                let mons = monitors.get();
                let current_id = active_ws_id.get();

                if enable_vdesks {
                    // Navigate whole virtual desktops via the vdesk dispatcher
                    let mc = mons.len().max(1) as i32;
                    let Some(active) = current_id else { return };
                    let cur_vdesk = ((active - 1) / mc) + 1;
                    let target = if up { cur_vdesk - 1 } else { cur_vdesk + 1 };
                    if target >= 1 {
                        svc_scroll.send(CompositorCommand::CustomDispatch(
                            "vdesk".to_string(),
                            target.to_string(),
                        ));
                    }
                    return;
                }

                // Navigate by position in the displayed (sorted) list, skipping
                // special workspaces — never off the ends
                let ws_raw = workspaces.get();
                let ui_ws = calculate_ui_workspaces(&config, &ws_raw, &mons);
                let normals: Vec<&UiWorkspace> = ui_ws.iter().filter(|w| !w.is_special).collect();
                let Some(pos) = normals.iter().position(|w| Some(w.id) == current_id) else {
                    return;
                };
                let next = if up {
                    pos.checked_sub(1).and_then(|p| normals.get(p))
                } else {
                    normals.get(pos + 1)
                };
                if let Some(next) = next
                    && Some(next.id) != current_id
                {
                    svc_scroll.send(CompositorCommand::FocusWorkspace(next.id));
                }
            }
        })
        .children(keyed(
            move || {
                let ws_raw = workspaces.get();
                let mons = monitors.get();
                calculate_ui_workspaces(&config, &ws_raw, &mons)
            },
            // Special workspaces have negative ids; the cast still yields a
            // unique key
            |uw| uw.id as u64,
            move |uw| {
                let id = uw.id;
                let label = uw.name;
                let is_special = uw.is_special;
                let colors = if is_special {
                    special_workspace_pairs()
                } else {
                    colors.clone()
                };
                let svc = svc_children;

                {
                    let primary_pair = pair_from(
                        &with_context::<Config, _>(|c| c.appearance.primary_color).unwrap(),
                        theme.background,
                        &theme,
                    );
                    // Per-pill reactive memos
                    let ws_color = create_memo({
                        let colors = colors.clone();
                        move || {
                            if enable_vdesks {
                                // Virtual desktops always have a color
                                resolve_ws_pair(&colors, Some(id as i128), primary_pair)
                            } else {
                                // Look up current monitor_id from live workspace data
                                let ws = workspaces.get();
                                let mid = ws.iter().find(|w| w.id == id).and_then(|w| w.monitor_id);
                                resolve_ws_pair(&colors, mid, primary_pair)
                            }
                        }
                    });

                    let displayed = create_memo(move || {
                        if is_special {
                            let mons = monitors.get();
                            if mons.iter().any(|m| m.special_workspace_id == id) {
                                Displayed::Active
                            } else {
                                Displayed::Hidden
                            }
                        } else if enable_vdesks {
                            let active = active_ws_id.get();
                            let mons = monitors.get();
                            let mc = mons.len().max(1) as i32;
                            let range_start = (id - 1) * mc + 1;
                            let range_end = id * mc;
                            let is_active =
                                active.is_some_and(|a| a >= range_start && a <= range_end);
                            if is_active {
                                Displayed::Active
                            } else {
                                Displayed::Hidden
                            }
                        } else {
                            compute_displayed(active_ws_id.get(), &monitors.get(), id)
                        }
                    });

                    let empty = create_memo(move || {
                        if enable_vdesks {
                            let ws = workspaces.get();
                            let mons = monitors.get();
                            let mc = mons.len().max(1) as i32;
                            let range_start = (id - 1) * mc + 1;
                            let range_end = id * mc;
                            ws.iter()
                                .filter(|w| w.id >= range_start && w.id <= range_end)
                                .all(|w| w.windows == 0)
                        } else {
                            is_empty(&workspaces.get(), id)
                        }
                    });

                    let mut pill = container()
                        .height(PILL_HEIGHT)
                        .background(move || {
                            pill_background(
                                bg_pair,
                                ws_color.get(),
                                displayed.get() == Displayed::Active,
                                empty.get(),
                            )
                        })
                        .corner_radius(PILL_CORNER_RADIUS)
                        .border(
                            move || pill_border_width(empty.get()),
                            move || {
                                pill_border_color(
                                    bg_pair,
                                    ws_color.get(),
                                    displayed.get() == Displayed::Active,
                                )
                            },
                        )
                        .layout(
                            Flex::row()
                                .main_alignment(MainAlignment::Center)
                                .cross_alignment(CrossAlignment::Center),
                        )
                        .overflow(Overflow::Hidden)
                        .child(
                            text(label.clone())
                                .color(move || {
                                    pill_text_color(
                                        theme,
                                        ws_color.get(),
                                        displayed.get() == Displayed::Active,
                                        empty.get(),
                                    )
                                })
                                .font_size(10)
                                .nowrap(),
                        )
                        .on_click({
                            let special_name = label.clone();
                            move || {
                                if is_special {
                                    // Specials toggle by name, not by focus id
                                    svc.send(CompositorCommand::ToggleSpecialWorkspace(
                                        special_name.clone(),
                                    ));
                                } else if displayed.get() == Displayed::Active {
                                    // Already focused: nothing to do
                                } else if enable_vdesks {
                                    svc.send(CompositorCommand::CustomDispatch(
                                        "vdesk".to_string(),
                                        id.to_string(),
                                    ));
                                } else {
                                    svc.send(CompositorCommand::FocusWorkspace(id));
                                }
                            }
                        })
                        .hover_state(|s| s.lighter(0.1).alpha(0.7).transform(Transform::scale(1.1)))
                        .animate_border_width(Transition::new(150, TimingFunction::EaseInOut))
                        .animate_border_color(Transition::new(150, TimingFunction::EaseInOut))
                        .animate_background(Transition::new(150, TimingFunction::EaseInOut))
                        .animate_transform(Transition::spring(SpringConfig::SNAPPY));

                    // Only plain numbered workspaces get the fixed-width
                    // treatment; named ones shrink to their label (upstream
                    // switches on "all ascii digits" the same way)
                    let is_numeric = !label.is_empty() && label.chars().all(|c| c.is_ascii_digit());
                    if is_special || !is_numeric {
                        // Shrink to content, padding varies by state
                        pill = pill.padding(move || -> Padding {
                            let px = match displayed.get() {
                                Displayed::Active => 12.0,
                                Displayed::Visible => 8.0,
                                Displayed::Hidden => 4.0,
                            };
                            Padding {
                                left: px,
                                right: px,
                                top: 0.0,
                                bottom: 0.0,
                            }
                        });
                    } else {
                        // Numbered workspaces: fixed width based on state
                        pill = pill
                            .width(move || displayed.get().width())
                            .animate_width(Transition::spring(SpringConfig::BOUNCY));
                    }

                    pill
                }
            },
        ))
}
