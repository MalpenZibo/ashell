use crate::{
    outputs::Outputs,
    services::{
        ReadOnlyService, ServiceEvent,
        compositor::{CompositorService, CompositorState, CompositorWindow},
    },
};
use iced::{
    Alignment, Color, Element, Length, Point, Rectangle, Renderer, Size, Subscription, SurfaceId,
    Theme,
    mouse::Cursor,
    widget::{
        canvas,
        canvas::{Geometry, Program},
        container,
    },
};
use std::collections::{BTreeMap, HashMap};

// Target minimap size in pixels; the layout is scaled to fit within it.
// MIN_TILE is enforced during layout, so a workspace with enough tiny tiles
// grows the canvas past the target rather than overlapping tiles. The growth
// is linear (MIN_TILE per stacked tile), so from 14 windows in one column the
// canvas exceeds the bar's content height and gets clipped.
const MAX_H: f32 = 16.0;
const MAX_W: f32 = 80.0;
const MIN_TILE: f32 = 2.0;
// Gap carved out of a tile's edge where it abuts a neighbour, so adjacent
// same-colour tiles don't merge into one block.
const TILE_GAP: f32 = 1.0;

/// An axis-aligned rectangle `(x, y, w, h)` in some 2D space.
type Rect = (f32, f32, f32, f32);

#[derive(Debug, Clone)]
pub enum Message {
    ServiceEvent(ServiceEvent<CompositorService>),
    /// The theme may have changed: drop the cached geometry so tiles are
    /// redrawn in the new colours.
    ConfigReloaded,
}

/// A workspace's minimap together with its tessellated geometry.
struct WorkspaceMinimap {
    canvas: MinimapCanvas,
    cache: canvas::Cache,
}

pub struct Minimap {
    service: Option<CompositorService>,
    // One entry per workspace shown on some output, rebuilt in update() on
    // compositor events; view() only hands out references, so no layout work
    // happens per render pass. Bars showing the same workspace share an entry.
    minimaps: HashMap<i32, WorkspaceMinimap>,
}

impl Minimap {
    pub fn new() -> Self {
        Self {
            service: None,
            minimaps: HashMap::new(),
        }
    }

    pub fn update(&mut self, message: Message) {
        match message {
            Message::ServiceEvent(event) => match event {
                ServiceEvent::Init(service) => {
                    self.service = Some(service);
                    self.rebuild();
                }
                ServiceEvent::Update(event) => {
                    if let Some(service) = &mut self.service {
                        service.update(event);
                        self.rebuild();
                    }
                }
                _ => {}
            },
            Message::ConfigReloaded => {
                for minimap in self.minimaps.values() {
                    minimap.cache.clear();
                }
            }
        }
    }

    fn rebuild(&mut self) {
        self.minimaps = self
            .service
            .as_ref()
            .map(|service| build_minimaps(service))
            .unwrap_or_default();
    }

    pub fn view(&self, id: SurfaceId, outputs: &Outputs) -> Option<Element<'_, Message>> {
        let service = self.service.as_ref()?;
        let ws_id = shown_workspace(service, outputs.get_monitor_name(id))?;
        let minimap = self.minimaps.get(&ws_id)?;
        let (w, h) = (minimap.canvas.width, minimap.canvas.height);
        Some(
            container(
                canvas(MinimapProgram {
                    canvas: &minimap.canvas,
                    cache: &minimap.cache,
                })
                .width(Length::Fixed(w))
                .height(Length::Fixed(h)),
            )
            .align_y(Alignment::Center)
            .into(),
        )
    }

    pub fn subscription(&self) -> Subscription<Message> {
        CompositorService::subscribe().map(Message::ServiceEvent)
    }
}

/// The workspace a bar on `monitor_name` shows: that monitor's active
/// workspace, or the focused one when the compositor doesn't know the output.
fn shown_workspace(state: &CompositorState, monitor_name: Option<&str>) -> Option<i32> {
    monitor_name
        .and_then(|name| state.monitors.iter().find(|m| m.name == name))
        .map(|m| m.active_workspace_id)
        .or(state.focused_workspace_id)
}

/// Build a minimap for every workspace some bar can show (see
/// [`shown_workspace`]), skipping workspaces with no tiled windows.
fn build_minimaps(state: &CompositorState) -> HashMap<i32, WorkspaceMinimap> {
    let mut minimaps = HashMap::new();
    let shown = state
        .monitors
        .iter()
        .map(|m| m.active_workspace_id)
        .chain(state.focused_workspace_id);
    for ws_id in shown {
        if minimaps.contains_key(&ws_id) {
            continue;
        }
        let windows: Vec<&CompositorWindow> = state
            .windows
            .iter()
            .filter(|w| w.workspace_id == Some(ws_id))
            .collect();
        if let Some(canvas) = build_canvas(&windows) {
            minimaps.insert(
                ws_id,
                WorkspaceMinimap {
                    canvas,
                    cache: canvas::Cache::new(),
                },
            );
        }
    }
    minimaps
}

#[derive(Debug, Clone, Copy)]
enum TileRole {
    Normal,
    Focused,
    Urgent,
}

fn role_of(w: &CompositorWindow) -> TileRole {
    if w.is_focused {
        TileRole::Focused
    } else if w.is_urgent {
        TileRole::Urgent
    } else {
        TileRole::Normal
    }
}

fn tile_color(role: TileRole, theme: &Theme) -> Color {
    match role {
        TileRole::Focused => theme.palette().primary,
        TileRole::Urgent => theme.palette().danger,
        TileRole::Normal => theme.extended_palette().background.strong.color,
    }
}

#[derive(Debug, Clone, Copy)]
struct CanvasTile {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    inset_r: f32,
    inset_b: f32,
    role: TileRole,
}

/// A fully positioned minimap, in canvas (post-scale) pixel coordinates.
struct MinimapCanvas {
    tiles: Vec<CanvasTile>,
    width: f32,
    height: f32,
}

struct MinimapProgram<'a> {
    canvas: &'a MinimapCanvas,
    cache: &'a canvas::Cache,
}

impl<Message> Program<Message> for MinimapProgram<'_> {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        _cursor: Cursor,
    ) -> Vec<Geometry> {
        // The geometry is cached until the compositor state changes or the
        // config is reloaded (update() rebuilds or clears the cache).
        let geometry = self.cache.draw(renderer, bounds.size(), |frame| {
            // Tiled windows. Insets shrink a tile only on sides with a
            // neighbour, so adjacent tiles stay separated while outer edges
            // stay flush. Tile sizes are already >= MIN_TILE from layout, so
            // the inset leaves at least a sliver visible.
            for t in &self.canvas.tiles {
                frame.fill_rectangle(
                    Point::new(t.x, t.y),
                    Size::new(t.w - t.inset_r, t.h - t.inset_b),
                    tile_color(t.role, theme),
                );
            }
        });

        vec![geometry]
    }
}

/// A tile placed by [`layout_tiled`], with adjacency known by construction:
/// a tile has a neighbour below unless it is the last in its column, and a
/// neighbour to the right iff the next column reaches below the tile's top
/// edge (all columns share it).
struct PlacedTile<'a> {
    rect: Rect,
    win: &'a CompositorWindow,
    neighbor_right: bool,
    neighbor_below: bool,
}

/// Lay tiled windows out left to right by column, top to bottom within a
/// column, with positions derived from the grid index. Sizes are multiplied
/// by `scale` and clamped to `min_size` before positions accumulate, so a
/// clamped tile pushes its neighbours along instead of overlapping them.
fn layout_tiled<'a>(
    tiled: &[&'a CompositorWindow],
    scale: f32,
    min_size: f32,
) -> Vec<PlacedTile<'a>> {
    let mut by_col: BTreeMap<u32, Vec<&CompositorWindow>> = BTreeMap::new();
    for w in tiled {
        let (col, _) = w.tile_position.unwrap_or((0, 0));
        by_col.entry(col).or_default().push(w);
    }
    for tiles in by_col.values_mut() {
        tiles.sort_by_key(|w| w.tile_position.map(|(_, r)| r).unwrap_or(0));
    }
    let cols: Vec<Vec<&CompositorWindow>> = by_col.into_values().collect();

    let tile_h = |w: &CompositorWindow| (w.tile_height.max(1.0) * scale).max(min_size);
    let col_heights: Vec<f32> = cols
        .iter()
        .map(|tiles| tiles.iter().map(|w| tile_h(w)).sum())
        .collect();

    let mut placed = Vec::new();
    let mut x = 0.0_f32;
    for (i, tiles) in cols.iter().enumerate() {
        let col_w = (tiles
            .iter()
            .map(|w| w.tile_width)
            .fold(0.0_f32, f32::max)
            .max(1.0)
            * scale)
            .max(min_size);
        let next_col_h = col_heights.get(i + 1).copied().unwrap_or(0.0);
        let mut y = 0.0_f32;
        for (j, w) in tiles.iter().enumerate() {
            let h = tile_h(w);
            placed.push(PlacedTile {
                rect: (x, y, col_w, h),
                win: w,
                neighbor_right: next_col_h > y,
                neighbor_below: j + 1 < tiles.len(),
            });
            y += h;
        }
        x += col_w;
    }
    placed
}

/// Build the minimap from the workspace's tiled windows, laid out by grid
/// index and scaled to fit. Floating windows have no grid position and aren't
/// rendered; showing them may be explored later, once niri exposes enough
/// information to place them. Returns `None` when there are no tiled windows
/// to draw.
fn build_canvas(windows: &[&CompositorWindow]) -> Option<MinimapCanvas> {
    let tiled: Vec<&CompositorWindow> =
        windows.iter().copied().filter(|w| !w.is_floating).collect();

    // Measuring pass in layout space (no clamp) to pick the scale, then the
    // real pass in canvas space where MIN_TILE re-flows positions.
    let measured = layout_tiled(&tiled, 1.0, 0.0);
    if measured.is_empty() {
        return None;
    }
    let layout_w = measured
        .iter()
        .map(|t| t.rect.0 + t.rect.2)
        .fold(0.0_f32, f32::max)
        .max(1.0);
    let layout_h = measured
        .iter()
        .map(|t| t.rect.1 + t.rect.3)
        .fold(0.0_f32, f32::max)
        .max(1.0);
    let scale = (MAX_H / layout_h).min(MAX_W / layout_w);

    let placed = layout_tiled(&tiled, scale, MIN_TILE);

    let tiles: Vec<CanvasTile> = placed
        .iter()
        .map(|t| {
            let (x, y, w, h) = t.rect;
            CanvasTile {
                x,
                y,
                w,
                h,
                inset_r: if t.neighbor_right { TILE_GAP } else { 0.0 },
                inset_b: if t.neighbor_below { TILE_GAP } else { 0.0 },
                role: role_of(t.win),
            }
        })
        .collect();

    let width = tiles.iter().map(|t| t.x + t.w).fold(1.0_f32, f32::max);
    let height = tiles.iter().map(|t| t.y + t.h).fold(1.0_f32, f32::max);
    Some(MinimapCanvas {
        tiles,
        width,
        height,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::compositor::types::CompositorMonitor;

    fn win(id: u64, col: u32, row: u32, w: f32, h: f32) -> CompositorWindow {
        CompositorWindow {
            id,
            workspace_id: Some(1),
            is_focused: false,
            is_floating: false,
            is_urgent: false,
            tile_position: Some((col, row)),
            tile_width: w,
            tile_height: h,
        }
    }

    fn layout(windows: &[CompositorWindow], scale: f32, min_size: f32) -> Vec<PlacedTile<'_>> {
        let refs: Vec<&CompositorWindow> = windows.iter().collect();
        layout_tiled(&refs, scale, min_size)
    }

    #[test]
    fn grid_positions_follow_cumulative_sizes() {
        let windows = [
            win(1, 0, 0, 100.0, 200.0),
            win(2, 0, 1, 100.0, 100.0),
            win(3, 1, 0, 50.0, 300.0),
        ];
        let placed = layout(&windows, 1.0, 0.0);

        assert_eq!(placed.len(), 3);
        assert_eq!(placed[0].rect, (0.0, 0.0, 100.0, 200.0));
        assert_eq!(placed[1].rect, (0.0, 200.0, 100.0, 100.0));
        assert_eq!(placed[2].rect, (100.0, 0.0, 50.0, 300.0));
    }

    #[test]
    fn neighbor_flags_follow_adjacency() {
        // Column 1 is shorter (150) than column 0's second tile's top (200),
        // so that tile has no right-hand neighbour while the first one does.
        let windows = [
            win(1, 0, 0, 100.0, 200.0),
            win(2, 0, 1, 100.0, 100.0),
            win(3, 1, 0, 50.0, 150.0),
        ];
        let placed = layout(&windows, 1.0, 0.0);

        assert!(placed[0].neighbor_right);
        assert!(placed[0].neighbor_below);
        assert!(!placed[1].neighbor_right);
        assert!(!placed[1].neighbor_below);
        assert!(!placed[2].neighbor_right);
        assert!(!placed[2].neighbor_below);
    }

    #[test]
    fn min_size_reflows_instead_of_overlapping() {
        // At this scale every tile height collapses below min_size; clamped
        // tiles must stack contiguously, not overlap.
        let windows = [
            win(1, 0, 0, 100.0, 10.0),
            win(2, 0, 1, 100.0, 10.0),
            win(3, 0, 2, 100.0, 10.0),
        ];
        let placed = layout(&windows, 0.05, 2.0);

        assert_eq!(placed[0].rect, (0.0, 0.0, 5.0, 2.0));
        assert_eq!(placed[1].rect, (0.0, 2.0, 5.0, 2.0));
        assert_eq!(placed[2].rect, (0.0, 4.0, 5.0, 2.0));
    }

    #[test]
    fn missing_tile_position_defaults_to_origin() {
        let mut w = win(1, 0, 0, 100.0, 100.0);
        w.tile_position = None;
        let windows = [w];
        let placed = layout(&windows, 1.0, 0.0);
        assert_eq!(placed[0].rect, (0.0, 0.0, 100.0, 100.0));
    }

    #[test]
    fn build_canvas_scales_to_target() {
        let windows = [win(1, 0, 0, 160.0, 32.0)];
        let refs: Vec<&CompositorWindow> = windows.iter().collect();
        let canvas = build_canvas(&refs).unwrap();

        assert_eq!(canvas.width, MAX_W);
        assert_eq!(canvas.height, MAX_H);
        assert_eq!(canvas.tiles.len(), 1);
    }

    #[test]
    fn build_canvas_grows_past_target_when_clamped() {
        // Ten stacked tiles scale to 1.6px each; MIN_TILE re-flows them to
        // 2px, growing the canvas past MAX_H instead of overlapping.
        let windows: Vec<CompositorWindow> =
            (0..10).map(|i| win(i, 0, i as u32, 100.0, 100.0)).collect();
        let refs: Vec<&CompositorWindow> = windows.iter().collect();
        let canvas = build_canvas(&refs).unwrap();

        assert_eq!(canvas.height, 10.0 * MIN_TILE);
        assert!(canvas.height > MAX_H);
        assert_eq!(canvas.width, MIN_TILE);
    }

    fn monitor(name: &str, active_workspace_id: i32) -> CompositorMonitor {
        CompositorMonitor {
            id: 0,
            name: name.to_string(),
            active_workspace_id,
            special_workspace_id: -1,
        }
    }

    fn on_workspace(mut w: CompositorWindow, ws_id: i32) -> CompositorWindow {
        w.workspace_id = Some(ws_id);
        w
    }

    /// Two outputs, DP-1 showing workspace 1 (focused) and HDMI-A-1 showing
    /// workspace 2, each with its own tiled windows.
    fn two_monitor_state() -> CompositorState {
        CompositorState {
            monitors: vec![monitor("DP-1", 1), monitor("HDMI-A-1", 2)],
            focused_workspace_id: Some(1),
            windows: vec![
                on_workspace(win(1, 0, 0, 100.0, 100.0), 1),
                on_workspace(win(2, 0, 0, 100.0, 100.0), 2),
                on_workspace(win(3, 1, 0, 100.0, 100.0), 2),
                // On a workspace no output shows.
                on_workspace(win(4, 0, 0, 100.0, 100.0), 3),
            ],
            ..Default::default()
        }
    }

    #[test]
    fn each_output_shows_its_active_workspace() {
        let state = two_monitor_state();
        assert_eq!(shown_workspace(&state, Some("DP-1")), Some(1));
        assert_eq!(shown_workspace(&state, Some("HDMI-A-1")), Some(2));
    }

    #[test]
    fn unknown_output_falls_back_to_focused_workspace() {
        let state = two_monitor_state();
        assert_eq!(shown_workspace(&state, Some("eDP-1")), Some(1));
        assert_eq!(shown_workspace(&state, None), Some(1));

        let unfocused = CompositorState {
            focused_workspace_id: None,
            ..two_monitor_state()
        };
        assert_eq!(shown_workspace(&unfocused, Some("eDP-1")), None);
    }

    #[test]
    fn output_without_workspace_does_not_fall_back() {
        let mut state = two_monitor_state();
        state.monitors[1].active_workspace_id = -1;
        assert_eq!(shown_workspace(&state, Some("HDMI-A-1")), Some(-1));
        assert!(!build_minimaps(&state).contains_key(&-1));
    }

    #[test]
    fn minimaps_built_per_shown_workspace() {
        let minimaps = build_minimaps(&two_monitor_state());

        let mut ids: Vec<i32> = minimaps.keys().copied().collect();
        ids.sort_unstable();
        assert_eq!(ids, [1, 2]);
        assert_eq!(minimaps[&1].canvas.tiles.len(), 1);
        assert_eq!(minimaps[&2].canvas.tiles.len(), 2);
    }

    #[test]
    fn empty_shown_workspace_has_no_minimap() {
        let mut state = two_monitor_state();
        state.windows.retain(|w| w.workspace_id != Some(2));
        let minimaps = build_minimaps(&state);
        assert!(minimaps.contains_key(&1));
        assert!(!minimaps.contains_key(&2));
    }

    #[test]
    fn build_canvas_ignores_floating_and_handles_empty() {
        let mut floating = win(1, 0, 0, 100.0, 100.0);
        floating.is_floating = true;
        let windows = [floating];
        let refs: Vec<&CompositorWindow> = windows.iter().collect();
        assert!(build_canvas(&refs).is_none());
        assert!(build_canvas(&[]).is_none());
    }
}
