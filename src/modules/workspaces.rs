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

fn workspace_colors() -> Vec<Color> {
    let ws = with_context::<Config, _>(|c| {
        c.appearance
            .workspace_colors
            .iter()
            .map(|c| c.base())
            .collect::<Vec<_>>()
    })
    .unwrap();
    if ws.is_empty() {
        let theme = expect_context::<ThemeColors>();
        vec![theme.primary, theme.success, theme.warning]
    } else {
        ws
    }
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

#[derive(Debug, Clone)]
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
        let existing_ids: Vec<i32> = result.iter().map(|w| w.id).collect();
        let mut max_id = existing_ids
            .iter()
            .filter(|&&id| id > 0)
            .max()
            .copied()
            .unwrap_or(0);

        if let Some(max_cfg) = config.max_workspaces
            && max_cfg as i32 > max_id
        {
            max_id = max_cfg as i32;
        }

        for id in 1..=max_id {
            if !existing_ids.contains(&id) {
                let name = if id > 0 {
                    let idx = (id - 1) as usize;
                    config
                        .workspace_names
                        .get(idx)
                        .cloned()
                        .unwrap_or_else(|| id.to_string())
                } else {
                    id.to_string()
                };
                result.push(UiWorkspace {
                    id,
                    index: id,
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

/// Resolve the workspace color. Returns None for workspaces without a monitor
/// (phantom/filled workspaces), which makes their border blend with the background.
fn resolve_ws_color(colors: &[Color], monitor_id: Option<i128>) -> Option<Color> {
    monitor_id.map(|mid| {
        let idx = mid.unsigned_abs() as usize;
        colors[idx % colors.len()]
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

fn pill_background(theme: ThemeColors, ws_color: Option<Color>, empty: bool) -> Color {
    if empty {
        theme.background.lighter(0.1)
    } else {
        ws_color.unwrap_or(theme.background.lighter(0.1))
    }
}

fn pill_border_width(empty: bool) -> f32 {
    if empty { 1.0 } else { 0.0 }
}

fn pill_border_color(theme: ThemeColors, ws_color: Option<Color>, active: bool) -> Color {
    if active {
        // Workspace color when assigned → visible colored border.
        // Surface color when unassigned → border blends with background.
        ws_color.unwrap_or(theme.background.lighter(0.8))
    } else {
        Color::TRANSPARENT
    }
}

fn pill_text_color(theme: ThemeColors, ws_color: Option<Color>, empty: bool) -> Color {
    if empty {
        theme.text
    } else if ws_color.is_some() {
        // Dark text on colored background
        theme.background
    } else {
        // Light text on surface background
        theme.text
    }
}

// ── View ─────────────────────────────────────────────────────────────────────

pub fn view(state: CompositorStateSignals, svc: Service<CompositorCommand>) -> impl Widget {
    let theme = expect_context::<ThemeColors>();
    let config = with_context::<Config, _>(|c| c.workspaces.clone()).unwrap();
    let colors = workspace_colors();
    let enable_vdesks = config.enable_virtual_desktops;

    let svc_scroll = svc.clone();
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
            move |_dx, dy, _source| {
                let ws_raw = workspaces.get();
                let mons = monitors.get();
                let ui_ws = calculate_ui_workspaces(&config, &ws_raw, &mons);
                let current_id = active_ws_id.get();

                let next = if dy > 0.0 {
                    // Scroll up → previous workspace (lower id)
                    current_id.and_then(|cur| {
                        ui_ws.iter().filter(|w| w.id < cur).max_by_key(|w| w.id)
                    })
                } else {
                    // Scroll down → next workspace (higher id)
                    current_id.and_then(|cur| {
                        ui_ws.iter().filter(|w| w.id > cur).min_by_key(|w| w.id)
                    })
                };

                if let Some(next) = next {
                    svc_scroll.send(CompositorCommand::FocusWorkspace(next.id));
                }
            }
        })
        .children(move || {
            let ws_raw = workspaces.get();
            let mons = monitors.get();
            let svc = svc_children.clone();
            let colors = colors.clone();

            let ui_workspaces = calculate_ui_workspaces(&config, &ws_raw, &mons);

            ui_workspaces
                .into_iter()
                .map(|uw| {
                    let id = uw.id;
                    let label = uw.name;
                    let is_special = uw.is_special;
                    let colors = colors.clone();
                    let svc = svc.clone();

                    (id as u64, move || {
                        // Per-pill reactive memos
                        let ws_color = create_memo({
                            let colors = colors.clone();
                            move || {
                                if enable_vdesks {
                                    // Virtual desktops always have a color
                                    resolve_ws_color(&colors, Some(id as i128))
                                } else {
                                    // Look up current monitor_id from live workspace data
                                    let ws = workspaces.get();
                                    let mid =
                                        ws.iter().find(|w| w.id == id).and_then(|w| w.monitor_id);
                                    resolve_ws_color(&colors, mid)
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
                            .background(move || pill_background(theme, ws_color.get(), empty.get()))
                            .corner_radius(PILL_CORNER_RADIUS)
                            .border(
                                move || pill_border_width(empty.get()),
                                move || {
                                    pill_border_color(
                                        theme,
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
                                text(label)
                                    .color(move || {
                                        pill_text_color(theme, ws_color.get(), empty.get())
                                    })
                                    .font_size(10)
                                    .nowrap(),
                            )
                            .on_click(move || {
                                if enable_vdesks {
                                    svc.send(CompositorCommand::CustomDispatch(
                                        "vdesk".to_string(),
                                        id.to_string(),
                                    ));
                                } else {
                                    svc.send(CompositorCommand::FocusWorkspace(id));
                                }
                            })
                            .hover_state(|s| {
                                s.lighter(0.1).alpha(0.7).transform(Transform::scale(1.1))
                            })
                            .animate_border_width(Transition {
                                duration_ms: 150.0,
                                timing: TimingFunction::EaseInOut,
                                delay_ms: 0.0,
                            })
                            .animate_border_color(Transition {
                                duration_ms: 150.0,
                                timing: TimingFunction::EaseInOut,
                                delay_ms: 0.0,
                            })
                            .animate_background(Transition {
                                duration_ms: 150.0,
                                timing: TimingFunction::EaseInOut,
                                delay_ms: 0.0,
                            })
                            .animate_transform(Transition::spring(SpringConfig::SNAPPY));

                        if is_special {
                            // Special workspaces: shrink to content, padding varies by state
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
                            // Normal workspaces: fixed width based on state
                            pill = pill
                                .width(move || displayed.get().width())
                                .animate_width(Transition::spring(SpringConfig::BOUNCY));
                        }

                        pill
                    })
                })
                .collect::<Vec<_>>()
        })
}
