use crate::services::{
    ReadOnlyService, ServiceEvent,
    compositor::{CompositorService, CompositorWindow},
};
use iced::{
    Alignment, Color, Element, Length, Point, Rectangle, Renderer, Size, Subscription, Theme,
    mouse::Cursor,
    widget::{
        canvas,
        canvas::{Frame, Geometry, Program},
        container,
    },
};
use std::collections::BTreeMap;

// Target minimap size in pixels; the layout is scaled to fit within it.
const MAX_H: f32 = 16.0;
const MAX_W: f32 = 80.0;
const MIN_TILE: f32 = 2.0;
// Gap carved out of a tile's edge where it abuts a neighbour, so adjacent
// same-colour tiles don't merge into one block.
const TILE_GAP: f32 = 1.0;
// Tolerance for "shared edge" — coordinates come from compositor floats, so
// allow sub-pixel slack.
const EPS: f32 = 0.5;

/// An axis-aligned rectangle `(x, y, w, h)` in some 2D space.
type Rect = (f32, f32, f32, f32);

#[derive(Debug, Clone)]
pub enum Message {
    ServiceEvent(ServiceEvent<CompositorService>),
}

pub struct Minimap {
    service: Option<CompositorService>,
}

impl Minimap {
    pub fn new() -> Self {
        Self { service: None }
    }

    pub fn update(&mut self, message: Message) {
        match message {
            Message::ServiceEvent(event) => match event {
                ServiceEvent::Init(service) => {
                    self.service = Some(service);
                }
                ServiceEvent::Update(event) => {
                    if let Some(service) = &mut self.service {
                        service.update(event);
                    }
                }
                _ => {}
            },
        }
    }

    pub fn view(&self) -> Option<Element<'_, Message>> {
        let service = self.service.as_ref()?;
        let active_id = service.active_workspace_id?;
        let windows: Vec<&CompositorWindow> = service
            .windows
            .iter()
            .filter(|w| w.workspace_id == Some(active_id))
            .collect();

        let minimap = build_canvas(&windows)?;

        let (w, h) = (minimap.width, minimap.height);
        Some(
            container(
                canvas(minimap)
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

impl<Message> Program<Message> for MinimapCanvas {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        _cursor: Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());

        // Tiled windows. Insets shrink a tile only on sides with a neighbour,
        // so adjacent tiles stay separated while outer edges stay flush.
        for t in &self.tiles {
            frame.fill_rectangle(
                Point::new(t.x, t.y),
                Size::new(
                    (t.w - t.inset_r).max(MIN_TILE),
                    (t.h - t.inset_b).max(MIN_TILE),
                ),
                tile_color(t.role, theme),
            );
        }

        vec![frame.into_geometry()]
    }
}

fn has_neighbor_right(rects: &[Rect], x: f32, y: f32, w: f32, h: f32) -> bool {
    rects
        .iter()
        .any(|&(ox, oy, _, oh)| (ox - (x + w)).abs() < EPS && oy < y + h - EPS && oy + oh > y + EPS)
}

fn has_neighbor_below(rects: &[Rect], x: f32, y: f32, w: f32, h: f32) -> bool {
    rects
        .iter()
        .any(|&(ox, oy, ow, _)| (oy - (y + h)).abs() < EPS && ox < x + w - EPS && ox + ow > x + EPS)
}

/// Lay tiled windows out in workspace-layout space (column 1 at x = 0):
/// columns left to right by cumulative max width, tiles top to bottom within
/// a column by cumulative height. Positions are derived from the grid index.
fn layout_tiled<'a>(tiled: &[&'a CompositorWindow]) -> Vec<(Rect, &'a CompositorWindow)> {
    let mut by_col: BTreeMap<u32, Vec<&CompositorWindow>> = BTreeMap::new();
    for w in tiled {
        let (col, _) = w.tile_position.unwrap_or((0, 0));
        by_col.entry(col).or_default().push(w);
    }
    for tiles in by_col.values_mut() {
        tiles.sort_by_key(|w| w.tile_position.map(|(_, r)| r).unwrap_or(0));
    }

    let mut placed = Vec::new();
    let mut x = 0.0_f32;
    for tiles in by_col.values() {
        let col_w = tiles
            .iter()
            .map(|w| w.tile_width)
            .fold(0.0_f32, f32::max)
            .max(1.0);
        let mut y = 0.0_f32;
        for w in tiles {
            let h = w.tile_height.max(1.0);
            placed.push(((x, y, col_w, h), *w));
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
    let placed = layout_tiled(&tiled);
    if placed.is_empty() {
        return None;
    }

    let rects: Vec<Rect> = placed.iter().map(|(r, _)| *r).collect();
    let layout_w = rects
        .iter()
        .map(|r| r.0 + r.2)
        .fold(0.0_f32, f32::max)
        .max(1.0);
    let layout_h = rects
        .iter()
        .map(|r| r.1 + r.3)
        .fold(0.0_f32, f32::max)
        .max(1.0);
    let scale = (MAX_H / layout_h).min(MAX_W / layout_w);

    let tiles: Vec<CanvasTile> = placed
        .iter()
        .map(|&((x, y, w, h), win)| CanvasTile {
            x: x * scale,
            y: y * scale,
            w: (w * scale).max(MIN_TILE),
            h: (h * scale).max(MIN_TILE),
            inset_r: if has_neighbor_right(&rects, x, y, w, h) {
                TILE_GAP
            } else {
                0.0
            },
            inset_b: if has_neighbor_below(&rects, x, y, w, h) {
                TILE_GAP
            } else {
                0.0
            },
            role: role_of(win),
        })
        .collect();

    Some(MinimapCanvas {
        tiles,
        width: (layout_w * scale).max(1.0),
        height: (layout_h * scale).max(1.0),
    })
}
