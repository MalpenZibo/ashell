use crate::{
    outputs::Outputs,
    services::{
        ReadOnlyService, ServiceEvent,
        compositor::{CompositorService, CompositorState, CompositorWindow},
    },
    theme::use_theme,
};
use iced::{
    Alignment, Color, Element, Length, Point, Rectangle, Renderer, Size, Subscription, SurfaceId,
    Theme, border,
    mouse::Cursor,
    widget::{
        canvas,
        canvas::{Geometry, Path, Program, Stroke, path},
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
// Column bottoms closer than this are drawn as one edge: tile sizes leave out
// the compositor's gaps between stacked windows, so a stacked column comes out
// a little shorter than a single-window one.
const EDGE_SLACK: f32 = 1.0;
const VIEWPORT_STROKE_W: f32 = 1.0;
const FLOATING_STROKE_W: f32 = 1.0;

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
/// [`shown_workspace`]), skipping workspaces with nothing to draw.
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
        if let Some(canvas) = build_canvas(&windows, viewport_rect(state, ws_id)) {
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

/// The output's on-screen viewport, in workspace-layout coordinates:
/// `(view_point.x, view_point.y, width, height)`. `None` unless the
/// compositor exposes both the view point and the monitor's logical size.
fn viewport_rect(state: &CompositorState, ws_id: i32) -> Option<Rect> {
    let ws = state.workspaces.iter().find(|w| w.id == ws_id)?;
    let mon = state.monitors.iter().find(|m| m.name == ws.monitor)?;
    let (px, py) = mon.view_point?;
    let (w, h) = mon.logical_size?;
    Some((px, py, w as f32, h as f32))
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
enum Shape {
    /// Tiled window: a filled rectangle, inset on sides with a neighbour.
    Tile,
    /// Floating window placed in the layout: filled rectangle with a halo,
    /// drawn on top of the tiling.
    Floating,
}

#[derive(Debug, Clone, Copy)]
struct CanvasTile {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    inset_r: f32,
    inset_b: f32,
    outer: Corners,
    role: TileRole,
    shape: Shape,
}

impl CanvasTile {
    /// How far the tile's drawing reaches past its rect: a floating window's
    /// halo.
    fn halo(&self) -> f32 {
        match self.shape {
            Shape::Floating => FLOATING_STROKE_W,
            Shape::Tile => 0.0,
        }
    }
}

/// The corners of a tile that lie on the minimap's outline, in the order
/// top-left, top-right, bottom-right, bottom-left.
type Corners = [bool; 4];

/// A fully positioned minimap, in canvas (post-scale) pixel coordinates.
struct MinimapCanvas {
    tiles: Vec<CanvasTile>,
    viewport: Option<Rect>,
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
            // Base layer: tiled windows. Insets shrink a tile only on sides
            // with a neighbour, so adjacent tiles stay separated while outer
            // edges stay flush with the viewport. Tile sizes are already
            // >= MIN_TILE from layout, so the inset leaves a sliver visible.
            let radius = use_theme(|t| t.radius.sm);
            for t in &self.canvas.tiles {
                if !matches!(t.shape, Shape::Tile) {
                    continue;
                }
                let size = Size::new(t.w - t.inset_r, t.h - t.inset_b);
                let r = clamp_radius(radius, size);
                frame.fill(
                    &Path::new(|b| rounded_rect(b, Point::new(t.x, t.y), size, t.outer, r)),
                    tile_color(t.role, theme),
                );
            }

            // Viewport outline over the tiles, rounded like the map's outline.
            // Stroked rectangles centre the stroke on the path, so nudge the
            // rect inward to keep it aligned with the canvas extent, and
            // shrink the radius with it so the outer edge keeps the theme's.
            if let Some((vx, vy, vw, vh)) = self.canvas.viewport {
                let inset = VIEWPORT_STROKE_W / 2.0;
                let size = Size::new(
                    (vw - VIEWPORT_STROKE_W).max(0.0),
                    (vh - VIEWPORT_STROKE_W).max(0.0),
                );
                let r = clamp_radius((radius - inset).max(0.0), size);
                let path = Path::new(|b| {
                    rounded_rect(b, Point::new(vx + inset, vy + inset), size, [true; 4], r);
                });
                frame.stroke(
                    &path,
                    Stroke::default()
                        .with_width(VIEWPORT_STROKE_W)
                        .with_color(theme.palette().text),
                );
            }

            // Floating windows on top of everything, each a standalone shape
            // with all its corners rounded. A halo in the canvas background
            // colour separates the floating fill from inactive tiles beneath
            // it, which would otherwise share the same colour and merge; its
            // radius grows by its width so the two stay concentric.
            let halo = theme.extended_palette().background.base.color;
            for t in &self.canvas.tiles {
                if !matches!(t.shape, Shape::Floating) {
                    continue;
                }
                let size = Size::new(t.w, t.h);
                let r = clamp_radius(radius, size);
                let halo_origin = Point::new(t.x - FLOATING_STROKE_W, t.y - FLOATING_STROKE_W);
                let halo_size =
                    Size::new(t.w + 2.0 * FLOATING_STROKE_W, t.h + 2.0 * FLOATING_STROKE_W);
                frame.fill(
                    &Path::new(|b| {
                        rounded_rect(b, halo_origin, halo_size, t.outer, r + FLOATING_STROKE_W);
                    }),
                    halo,
                );
                frame.fill(
                    &Path::new(|b| rounded_rect(b, Point::new(t.x, t.y), size, t.outer, r)),
                    tile_color(t.role, theme),
                );
            }
        });

        vec![geometry]
    }
}

/// Clamp a corner radius so the rounding fits a rectangle of `size`.
fn clamp_radius(radius: f32, size: Size) -> f32 {
    radius.min(size.width / 2.0).min(size.height / 2.0)
}

/// Add a rectangle to `b`, rounding the given corners with `radius`.
fn rounded_rect(b: &mut path::Builder, origin: Point, size: Size, corners: Corners, radius: f32) {
    let [tl, tr, br, bl] = corners.map(|c| if c { radius } else { 0.0 });
    b.rounded_rectangle(
        origin,
        size,
        border::Radius {
            top_left: tl,
            top_right: tr,
            bottom_right: br,
            bottom_left: bl,
        },
    );
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
    outer: Corners,
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
        let col_h = col_heights[i];
        let prev_col_h = i.checked_sub(1).map_or(0.0, |p| col_heights[p]);
        let next_col_h = col_heights.get(i + 1).copied().unwrap_or(0.0);
        let mut y = 0.0_f32;
        for (j, w) in tiles.iter().enumerate() {
            let h = tile_h(w);
            let (top, bottom) = (j == 0, j + 1 == tiles.len());
            placed.push(PlacedTile {
                rect: (x, y, col_w, h),
                win: w,
                neighbor_right: next_col_h > y,
                neighbor_below: !bottom,
                outer: [
                    top && i == 0,
                    top && i + 1 == cols.len(),
                    bottom && next_col_h < col_h - EDGE_SLACK,
                    bottom && prev_col_h < col_h - EDGE_SLACK,
                ],
            });
            y += h;
        }
        x += col_w;
    }
    placed
}

/// Build the minimap in one workspace-layout coordinate space: tiled windows
/// from their grid index, floating windows from their absolute `floating_pos`,
/// and the output's viewport drawn as an outlined rectangle. Returns `None`
/// only when there are neither windows nor a viewport to draw.
fn build_canvas(windows: &[&CompositorWindow], viewport: Option<Rect>) -> Option<MinimapCanvas> {
    let tiled: Vec<&CompositorWindow> =
        windows.iter().copied().filter(|w| !w.is_floating).collect();

    // Measuring pass in layout space (no clamp), used only to pick the scale
    // and the bounding box; the real tiled pass below re-flows in canvas
    // space where MIN_TILE is enforced.
    let measured = layout_tiled(&tiled, 1.0, 0.0);

    // Floating windows stay in layout space: they are placed absolutely, not
    // packed, so they scale directly and take no part in the re-flow.
    let floating: Vec<(Rect, TileRole)> = windows
        .iter()
        .copied()
        .filter(|w| w.is_floating)
        .filter_map(|w| {
            w.floating_pos
                .map(|(x, y)| ((x, y, w.tile_width, w.tile_height), role_of(w)))
        })
        .collect();

    // Nothing to draw only when there are no windows and no viewport; an empty
    // workspace still shows its viewport rectangle.
    if measured.is_empty() && floating.is_empty() && viewport.is_none() {
        return None;
    }

    // Bounding box over everything placed in layout space (windows + viewport).
    let mut bounds: Vec<Rect> = measured.iter().map(|t| t.rect).collect();
    bounds.extend(floating.iter().map(|(r, _)| *r));
    bounds.extend(viewport);
    let min_x = bounds.iter().map(|r| r.0).fold(f32::MAX, f32::min);
    let min_y = bounds.iter().map(|r| r.1).fold(f32::MAX, f32::min);
    let max_x = bounds.iter().map(|r| r.0 + r.2).fold(f32::MIN, f32::max);
    let max_y = bounds.iter().map(|r| r.1 + r.3).fold(f32::MIN, f32::max);
    let layout_w = (max_x - min_x).max(1.0);
    let layout_h = (max_y - min_y).max(1.0);
    let scale = (MAX_H / layout_h).min(MAX_W / layout_w);

    // Tiled pass in canvas space. The tiled grid's layout origin is x = 0,
    // which lands at -min_x * scale on the canvas. While MIN_TILE clamping is
    // inactive the re-flowed positions match the scaled layout exactly, so
    // tiles stay aligned with the viewport and floating windows; once it
    // engages, tiles shift rather than overlap.
    let (dx, dy) = (-min_x * scale, -min_y * scale);
    let placed = layout_tiled(&tiled, scale, MIN_TILE);

    let mut tiles: Vec<CanvasTile> = placed
        .iter()
        .map(|t| {
            let (x, y, w, h) = t.rect;
            CanvasTile {
                x: x + dx,
                y: y + dy,
                w,
                h,
                inset_r: if t.neighbor_right { TILE_GAP } else { 0.0 },
                inset_b: if t.neighbor_below { TILE_GAP } else { 0.0 },
                outer: t.outer,
                role: role_of(t.win),
                shape: Shape::Tile,
            }
        })
        .collect();
    tiles.extend(floating.iter().map(|&((x, y, w, h), role)| CanvasTile {
        x: (x - min_x) * scale,
        y: (y - min_y) * scale,
        w: (w * scale).max(MIN_TILE),
        h: (h * scale).max(MIN_TILE),
        inset_r: 0.0,
        inset_b: 0.0,
        // A floating window is a shape of its own: every corner is outer.
        outer: [true; 4],
        role,
        shape: Shape::Floating,
    }));

    let viewport = viewport.map(|(x, y, w, h)| {
        (
            (x - min_x) * scale,
            (y - min_y) * scale,
            w * scale,
            h * scale,
        )
    });

    // A floating window's halo reaches past it, so shift everything right
    // and down as far as a halo would stick out over the top or left edge.
    let (shift_x, shift_y) = tiles.iter().fold((0.0_f32, 0.0_f32), |(sx, sy), t| {
        (sx.max(t.halo() - t.x), sy.max(t.halo() - t.y))
    });
    for t in &mut tiles {
        t.x += shift_x;
        t.y += shift_y;
    }
    let viewport = viewport.map(|(x, y, w, h)| (x + shift_x, y + shift_y, w, h));

    // Canvas extent from what's actually drawn, halos included, since the
    // MIN_TILE re-flow can exceed the scaled layout size.
    let mut width = tiles
        .iter()
        .map(|t| t.x + t.w + t.halo())
        .fold(1.0_f32, f32::max);
    let mut height = tiles
        .iter()
        .map(|t| t.y + t.h + t.halo())
        .fold(1.0_f32, f32::max);
    if let Some((vx, vy, vw, vh)) = viewport {
        width = width.max(vx + vw);
        height = height.max(vy + vh);
    }

    Some(MinimapCanvas {
        tiles,
        viewport,
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
            floating_pos: None,
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
    fn only_corners_on_the_outline_are_outer() {
        let windows = [
            win(1, 0, 0, 100.0, 200.0),
            win(2, 0, 1, 100.0, 100.0),
            win(3, 1, 0, 50.0, 150.0),
        ];
        let placed = layout(&windows, 1.0, 0.0);

        assert_eq!(placed[0].outer, [true, false, false, false]);
        assert_eq!(placed[1].outer, [false, false, true, true]);
        assert_eq!(placed[2].outer, [false, true, true, false]);
    }

    #[test]
    fn nearly_level_columns_share_their_bottom_edge() {
        let windows = [
            win(1, 0, 0, 100.0, 16.0),
            win(2, 1, 0, 100.0, 7.75),
            win(3, 1, 1, 100.0, 7.75),
        ];
        let placed = layout(&windows, 1.0, 0.0);

        assert_eq!(placed[0].outer, [true, false, false, true]);
        assert_eq!(placed[2].outer, [false, false, true, false]);
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
        let canvas = build_canvas(&refs, None).unwrap();

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
        let canvas = build_canvas(&refs, None).unwrap();

        assert_eq!(canvas.height, 10.0 * MIN_TILE);
        assert!(canvas.height > MAX_H);
        assert_eq!(canvas.width, MIN_TILE);
    }

    fn monitor(name: &str, active_workspace_id: i32) -> CompositorMonitor {
        CompositorMonitor {
            id: 0,
            name: name.to_string(),
            focused: false,
            active_workspace_id,
            active_workspace_name: String::new(),
            special_workspace_name: String::new(),
            view_point: None,
            logical_size: None,
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
    fn build_canvas_handles_empty_and_unplaced_floating() {
        // A floating window without a position can't be drawn; with nothing
        // else to show the canvas is None.
        let mut floating = win(1, 0, 0, 100.0, 100.0);
        floating.is_floating = true;
        let windows = [floating];
        let refs: Vec<&CompositorWindow> = windows.iter().collect();
        assert!(build_canvas(&refs, None).is_none());
        assert!(build_canvas(&[], None).is_none());
    }

    #[test]
    fn viewport_offsets_layout_origin() {
        // The viewport extends left of column 1 (gaps/centering), so layout
        // x = 0 lands inside the canvas, not at its left edge.
        let windows = [win(1, 0, 0, 48.0, 64.0)];
        let refs: Vec<&CompositorWindow> = windows.iter().collect();
        let canvas = build_canvas(&refs, Some((-16.0, 0.0, 80.0, 64.0))).unwrap();

        // layout 80x64 -> scale 0.25; origin shifted by 16 * 0.25.
        assert_eq!(canvas.tiles[0].x, 4.0);
        assert_eq!(canvas.tiles[0].w, 12.0);
        assert_eq!(canvas.viewport, Some((0.0, 0.0, 20.0, 16.0)));
        assert_eq!(canvas.width, 20.0);
        assert_eq!(canvas.height, 16.0);
    }

    #[test]
    fn empty_workspace_still_shows_viewport() {
        let canvas = build_canvas(&[], Some((0.0, 0.0, 160.0, 32.0))).unwrap();
        assert!(canvas.tiles.is_empty());
        assert_eq!(canvas.viewport, Some((0.0, 0.0, MAX_W, MAX_H)));
    }

    #[test]
    fn floating_windows_scale_directly() {
        let mut floating = win(2, 0, 0, 20.0, 20.0);
        floating.is_floating = true;
        floating.floating_pos = Some((10.0, 10.0));
        let windows = [win(1, 0, 0, 160.0, 32.0), floating];
        let refs: Vec<&CompositorWindow> = windows.iter().collect();
        let canvas = build_canvas(&refs, None).unwrap();

        assert_eq!(canvas.tiles.len(), 2);
        let float = &canvas.tiles[1];
        assert!(matches!(float.shape, Shape::Floating));
        assert_eq!((float.x, float.y, float.w, float.h), (5.0, 5.0, 10.0, 10.0));
    }

    #[test]
    fn floating_halo_stays_inside_the_canvas() {
        // A floating window in the map's top-left corner: everything shifts by
        // the halo's width so the halo isn't clipped, and the extent counts it.
        let mut floating = win(2, 0, 0, 20.0, 20.0);
        floating.is_floating = true;
        floating.floating_pos = Some((0.0, 0.0));
        let windows = [win(1, 0, 0, 160.0, 32.0), floating];
        let refs: Vec<&CompositorWindow> = windows.iter().collect();
        let canvas = build_canvas(&refs, None).unwrap();

        let (tile, float) = (&canvas.tiles[0], &canvas.tiles[1]);
        assert_eq!((float.x, float.y), (FLOATING_STROKE_W, FLOATING_STROKE_W));
        assert_eq!((tile.x, tile.y), (FLOATING_STROKE_W, FLOATING_STROKE_W));
        assert_eq!(canvas.width, MAX_W + FLOATING_STROKE_W);
        assert_eq!(canvas.height, MAX_H + FLOATING_STROKE_W);
    }
}
