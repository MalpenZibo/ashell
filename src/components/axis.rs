//! Bar-axis helpers.
//!
//! ashell lays a bar out along a *main* axis (horizontal for a top/bottom bar,
//! vertical for a left/right bar) and a perpendicular *cross* axis (the bar's
//! thickness). iced exposes `iced_core::layout::flex::Axis` but its
//! `main`/`cross`/`pack` helpers are private, so widgets that need to reason
//! about both orientations (the name-agnostic `Centerbox`, `MenuWrapper` and
//! the module group layout) share the small helpers defined here instead.

use iced::{Alignment, Padding, Point, Size};

/// Direction of the bar's *main* axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Axis {
    Horizontal,
    Vertical,
}

impl Axis {
    /// Extent of the main axis.
    pub fn main(self, size: Size) -> f32 {
        match self {
            Axis::Horizontal => size.width,
            Axis::Vertical => size.height,
        }
    }

    /// Extent of the cross axis.
    pub fn cross(self, size: Size) -> f32 {
        match self {
            Axis::Horizontal => size.height,
            Axis::Vertical => size.width,
        }
    }

    /// Combine a main and a cross component into `(width, height)`.
    pub fn pack<T>(self, main: T, cross: T) -> (T, T) {
        match self {
            Axis::Horizontal => (main, cross),
            Axis::Vertical => (cross, main),
        }
    }

    /// Build a [`Point`] from a main and a cross coordinate.
    pub fn point(self, main: f32, cross: f32) -> Point {
        let (x, y) = self.pack(main, cross);
        Point::new(x, y)
    }

    /// Build a [`Padding`] from a main and a cross inset.
    pub fn padding(self, main: f32, cross: f32) -> Padding {
        match self {
            Axis::Horizontal => Padding::new(cross).left(main).right(main),
            Axis::Vertical => Padding::new(main).left(cross).right(cross),
        }
    }

    /// Split a [`Padding`] into `(main_before, main_after, cross_before, cross_after)`.
    pub fn padding_parts(self, padding: Padding) -> (f32, f32, f32, f32) {
        match self {
            Axis::Horizontal => (padding.left, padding.right, padding.top, padding.bottom),
            Axis::Vertical => (padding.top, padding.bottom, padding.left, padding.right),
        }
    }

    /// Map main/cross alignments into `(horizontal, vertical)` alignments.
    pub fn align(self, main: Alignment, cross: Alignment) -> (Alignment, Alignment) {
        match self {
            Axis::Horizontal => (main, cross),
            Axis::Vertical => (cross, main),
        }
    }
}

/// The screen edge a menu anchors to, mirroring the bar's anchoring edge.
///
/// `Left`/`Right` are exercised once `Position::{Left, Right}` land (PR2), so
/// the problematic variants are allowed until then.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BarEdge {
    Top,
    Bottom,
    Left,
    Right,
}

impl BarEdge {
    /// The axis perpendicular to this edge, i.e. the bar's main axis. A menu
    /// is centered/clamped along this axis and pinned along the other.
    pub fn axis(self) -> Axis {
        match self {
            BarEdge::Top | BarEdge::Bottom => Axis::Horizontal,
            BarEdge::Left | BarEdge::Right => Axis::Vertical,
        }
    }

    /// Alignment along the anchored (pin) axis that attaches content to this edge.
    pub fn anchor_alignment(self) -> Alignment {
        match self {
            BarEdge::Top | BarEdge::Left => Alignment::Start,
            BarEdge::Bottom | BarEdge::Right => Alignment::End,
        }
    }

    /// Whether the anchored edge is the *start* of the pin axis (top or left).
    pub fn is_start(self) -> bool {
        self.anchor_alignment() == Alignment::Start
    }
}
