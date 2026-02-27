use guido::prelude::*;

use crate::config::Config;
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
        c.appearance.workspace_colors.iter().map(|c| c.base()).collect::<Vec<_>>()
    }).unwrap();
    if ws.is_empty() {
        let theme = expect_context::<ThemeColors>();
        vec![theme.primary, theme.success, theme.warning]
    } else {
        ws
    }
}

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

fn workspace_color(monitor_id: Option<i128>) -> Color {
    let idx = monitor_id.unwrap_or(0).unsigned_abs() as usize;
    let colors = workspace_colors();
    colors[idx % colors.len()]
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

fn pill_background(theme: ThemeColors, color: Color, displayed: Displayed, empty: bool) -> Color {
    if empty {
        theme.surface()
    } else {
        match displayed {
            Displayed::Active | Displayed::Visible => color,
            Displayed::Hidden => Color::rgba(color.r, color.g, color.b, 0.4),
        }
    }
}

fn pill_border_width(empty: bool) -> f32 {
    if empty { 1.0 } else { 0.0 }
}

fn pill_text_color(theme: ThemeColors, displayed: Displayed, empty: bool) -> Color {
    if empty {
        match displayed {
            Displayed::Active | Displayed::Visible => theme.text,
            Displayed::Hidden => Color::rgba(theme.text.r, theme.text.g, theme.text.b, 0.4),
        }
    } else {
        match displayed {
            Displayed::Active | Displayed::Visible => theme.background,
            Displayed::Hidden => theme.text,
        }
    }
}

fn pill_border_color(color: Color, displayed: Displayed, empty: bool) -> Color {
    if empty {
        match displayed {
            Displayed::Active | Displayed::Visible => color,
            Displayed::Hidden => Color::rgba(color.r, color.g, color.b, 0.4),
        }
    } else {
        Color::TRANSPARENT
    }
}

pub fn view(state: CompositorStateSignals, svc: Service<CompositorCommand>) -> impl Widget {
    let theme = expect_context::<ThemeColors>();
    let svc_scroll = svc.clone();
    let svc_children = svc;

    // Direct per-field signals — no Memo workaround needed
    let workspaces = state.workspaces;
    let monitors = state.monitors;
    let active_ws_id = state.active_workspace_id;

    container()
        .layout(
            Flex::row()
                .spacing(4.0)
                .cross_alignment(CrossAlignment::Center),
        )
        .on_scroll(move |_dx, dy, _source| {
            let dir = if dy > 0.0 { -1 } else { 1 };
            svc_scroll.send(CompositorCommand::ScrollWorkspace(dir));
        })
        .children(move || {
            let ws = workspaces.get();
            let svc = svc_children.clone();

            ws.iter()
                .filter(|w| !w.is_special)
                .map(|w| {
                    let id = w.id;
                    let color = workspace_color(w.monitor_id);
                    let svc = svc.clone();

                    (id as u64, move || {
                        // Per-pill derived values — only repaint when this pill's state changes
                        let displayed = create_memo(move || {
                            let active = active_ws_id.get();
                            let mons = monitors.get();
                            compute_displayed(active, &mons, id)
                        });
                        let empty = create_memo(move || is_empty(&workspaces.get(), id));

                        let label = id.to_string();
                        container()
                            .width(move || displayed.get().width())
                            .height(PILL_HEIGHT)
                            .background(move || {
                                pill_background(theme, color, displayed.get(), empty.get())
                            })
                            .corner_radius(PILL_CORNER_RADIUS)
                            .border(
                                move || pill_border_width(empty.get()),
                                move || pill_border_color(color, displayed.get(), empty.get()),
                            )
                            .layout(
                                Flex::row()
                                    .main_alignment(MainAlignment::Center)
                                    .cross_alignment(CrossAlignment::Center),
                            )
                            .overflow(Overflow::Hidden)
                            .child(
                                text(label)
                                    .color(move || pill_text_color(theme, displayed.get(), empty.get()))
                                    .font_size(10.0)
                                    .nowrap(),
                            )
                            .on_click(move || {
                                svc.send(CompositorCommand::FocusWorkspace(id));
                            })
                            .hover_state(|s| s.lighter(0.1).alpha(0.7))
                            .animate_width(Transition {
                                duration_ms: 150.0,
                                timing: TimingFunction::EaseInOut,
                                delay_ms: 0.0,
                            })
                            .animate_background(Transition {
                                duration_ms: 150.0,
                                timing: TimingFunction::EaseInOut,
                                delay_ms: 0.0,
                            })
                    })
                })
                .collect::<Vec<_>>()
        })
}
