//! Distribute content along the bar's main axis.
use iced::Animation;
use iced::advanced::layout::{self, Layout, Limits, Node};
use iced::advanced::overlay;
use iced::advanced::renderer;
use iced::advanced::widget::{Operation, Tree, tree};
use iced::advanced::{Clipboard, Shell, Widget, mouse};
use iced::animation::Easing;
use iced::{Alignment, Length, Padding, Pixels, Rectangle, Size, Vector, event};
use std::time::{Duration, Instant};

use super::Axis;

type Element<'a, Message, Theme, Renderer> = iced::core::Element<'a, Message, Theme, Renderer>;

struct State {
    center_main_anim: Animation<f32>,
    last_center_main: f32,
    initialized: bool,
}

impl State {
    fn new() -> Self {
        Self {
            center_main_anim: Animation::new(0.0),
            last_center_main: 0.0,
            initialized: false,
        }
    }
}

/// A container that distributes its contents along one axis.
#[allow(missing_debug_implementations)]
pub struct Centerbox<'a, Message, Theme = iced::Theme, Renderer = iced::Renderer> {
    spacing: f32,
    padding: Padding,
    width: Length,
    height: Length,
    align_items: Alignment,
    animated: bool,
    axis: Axis,
    children: [Element<'a, Message, Theme, Renderer>; 3],
}

impl<'a, Message, Theme, Renderer> Centerbox<'a, Message, Theme, Renderer>
where
    Renderer: iced::advanced::Renderer,
{
    /// Creates an empty [`Centerbox`].
    pub fn new(children: [Element<'a, Message, Theme, Renderer>; 3]) -> Self {
        Centerbox {
            spacing: 0.0,
            padding: Padding::ZERO,
            width: Length::Shrink,
            height: Length::Shrink,
            align_items: Alignment::Start,
            animated: true,
            axis: Axis::Horizontal,
            children,
        }
    }

    /// Enables or disables the position animation of the center element.
    /// When `false`, the center snaps to its target position instead.
    pub fn animated(mut self, animated: bool) -> Self {
        self.animated = animated;
        self
    }

    /// Sets the axis the [`Centerbox`] distributes its children along.
    pub fn axis(mut self, axis: Axis) -> Self {
        self.axis = axis;
        self
    }

    /// Sets the spacing _between_ elements along the main axis.
    ///
    /// Custom margins per element do not exist in iced. You should use this
    /// method instead! While less flexible, it helps you keep spacing between
    /// elements consistent.
    pub fn spacing(mut self, amount: impl Into<Pixels>) -> Self {
        self.spacing = amount.into().0;
        self
    }

    /// Sets the [`Padding`] of the [`Centerbox`].
    pub fn padding<P: Into<Padding>>(mut self, padding: P) -> Self {
        self.padding = padding.into();
        self
    }

    /// Sets the width of the [`Centerbox`].
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    /// Sets the height of the [`Centerbox`].
    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();
        self
    }

    /// Sets the cross-axis alignment of the contents of the [`Centerbox`].
    pub fn align_items(mut self, align: Alignment) -> Self {
        self.align_items = align;
        self
    }
}

impl<'a, Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for Centerbox<'a, Message, Theme, Renderer>
where
    Renderer: iced::advanced::Renderer,
{
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::new())
    }

    fn children(&self) -> Vec<Tree> {
        self.children.iter().map(Tree::new).collect()
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(&self.children)
    }

    fn size(&self) -> Size<Length> {
        Size {
            width: self.width,
            height: self.height,
        }
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let limits = limits
            .width(self.width)
            .height(self.height)
            .shrink(self.padding);

        let total_spacing = self.spacing * 3_i32.saturating_sub(1) as f32;
        let max_cross = self.axis.cross(limits.max());
        let max_main = self.axis.main(limits.max());

        let (main_length, cross_length) = match self.axis {
            Axis::Horizontal => (self.width, self.height),
            Axis::Vertical => (self.height, self.width),
        };

        let mut cross = match cross_length {
            Length::Shrink => 0.0,
            _ => max_cross,
        };

        let available = max_main - total_spacing;

        let mut nodes = [Node::default(), Node::default(), Node::default()];

        let mut remaining = match main_length {
            Length::Shrink => 0.0,
            _ => available.max(0.0),
        };

        let mut calculate_edge_layout =
            |i: usize, (child, tree): (&mut Element<'a, Message, Theme, Renderer>, &mut Tree)| {
                let fill_cross_factor = {
                    let size = child.as_widget().size();

                    match self.axis {
                        Axis::Horizontal => size.height.fill_factor(),
                        Axis::Vertical => size.width.fill_factor(),
                    }
                };

                let (child_max_main, child_max_cross) = (
                    remaining,
                    if fill_cross_factor != 0 {
                        cross
                    } else {
                        max_cross
                    },
                );

                let (max_width, max_height) = self.axis.pack(child_max_main, child_max_cross);
                let child_limits = Limits::new(Size::ZERO, Size::new(max_width, max_height));

                let layout = child.as_widget_mut().layout(tree, renderer, &child_limits);
                let size = layout.size();

                remaining -= self.axis.main(size);
                cross = cross.max(self.axis.cross(size));

                nodes[i] = layout;
            };

        calculate_edge_layout(0, (&mut self.children[0], &mut tree.children[0]));
        calculate_edge_layout(2, (&mut self.children[2], &mut tree.children[2]));
        calculate_edge_layout(1, (&mut self.children[1], &mut tree.children[1]));

        let (main_before, main_after, cross_before, _) = self.axis.padding_parts(self.padding);
        let (align_width, align_height) = self.axis.pack(0.0, cross);
        let align_space = Size::new(align_width, align_height);

        nodes[0].move_to_mut(self.axis.point(main_before, cross_before));
        {
            let (h, v) = self.axis.align(Alignment::Start, self.align_items);
            nodes[0].align_mut(h, v, align_space);
        }
        nodes[2].move_to_mut(self.axis.point(max_main + main_after, cross_before));
        {
            let (h, v) = self.axis.align(Alignment::End, self.align_items);
            nodes[2].align_mut(h, v, align_space);
        }

        let half_available = available / 2.0;
        let half_center_main = self.axis.main(nodes[1].size()) / 2.0;
        let first_main = self.axis.main(nodes[0].size());
        let last_main = self.axis.main(nodes[2].size());

        let target_center_main = if half_available - first_main < half_center_main
            || half_available - last_main < half_center_main
        {
            main_before + self.spacing + first_main + (available - first_main - last_main) / 2.0
        } else {
            max_main / 2. + (main_before + main_after) / 2.0
        };

        let state = tree.state.downcast_mut::<State>();
        let now = Instant::now();

        let display_center_main = if !self.animated {
            state.last_center_main = target_center_main;
            state.initialized = true;
            target_center_main
        } else if !state.initialized {
            state.center_main_anim = Animation::new(target_center_main)
                .duration(Duration::from_millis(100))
                .easing(Easing::EaseOutCubic);
            state.last_center_main = target_center_main;
            state.initialized = true;
            target_center_main
        } else if (target_center_main - state.last_center_main).abs() > 0.5 {
            state.last_center_main = target_center_main;
            state.center_main_anim.go_mut(target_center_main, now);
            state.center_main_anim.interpolate_with(|v| v, now)
        } else if state.center_main_anim.is_animating(now) {
            state.center_main_anim.interpolate_with(|v| v, now)
        } else {
            target_center_main
        };

        nodes[1].move_to_mut(self.axis.point(display_center_main, cross_before));
        {
            let (h, v) = self.axis.align(Alignment::Center, self.align_items);
            nodes[1].align_mut(h, v, align_space);
        }

        let main = self.axis.main(nodes[0].size())
            + self.axis.main(nodes[1].size())
            + self.axis.main(nodes[2].size())
            + total_spacing;

        let (intrinsic_width, intrinsic_height) = self.axis.pack(main, cross);
        let size = limits.resolve(
            self.width,
            self.height,
            Size::new(intrinsic_width, intrinsic_height),
        );

        Node::with_children(size.expand(self.padding), nodes.into())
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        operation.container(None, layout.bounds());
        operation.traverse(&mut |operation| {
            self.children
                .iter_mut()
                .zip(&mut tree.children)
                .zip(layout.children())
                .for_each(|((child, state), layout)| {
                    child
                        .as_widget_mut()
                        .operate(state, layout, renderer, operation);
                });
        });
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &event::Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        for ((child, state), layout) in self
            .children
            .iter_mut()
            .zip(&mut tree.children)
            .zip(layout.children())
        {
            child.as_widget_mut().update(
                state, event, layout, cursor, renderer, clipboard, shell, viewport,
            );
        }

        if let event::Event::Window(iced::core::window::Event::RedrawRequested(now)) = event {
            let state = tree.state.downcast_mut::<State>();
            if state.center_main_anim.is_animating(*now) {
                shell.request_redraw();
                shell.invalidate_layout();
            }
        }
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.children
            .iter()
            .zip(&tree.children)
            .zip(layout.children())
            .map(|((child, state), layout)| {
                child
                    .as_widget()
                    .mouse_interaction(state, layout, cursor, viewport, renderer)
            })
            .max()
            .unwrap_or_default()
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        if let Some(viewport) = layout.bounds().intersection(viewport) {
            for ((child, state), layout) in self
                .children
                .iter()
                .zip(&tree.children)
                .zip(layout.children())
            {
                child
                    .as_widget()
                    .draw(state, renderer, theme, style, layout, cursor, &viewport);
            }
        }
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        overlay::from_children(
            &mut self.children,
            tree,
            layout,
            renderer,
            viewport,
            translation,
        )
    }
}

impl<'a, Message, Theme, Renderer> From<Centerbox<'a, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: 'a,
    Theme: 'a,
    Renderer: iced::advanced::Renderer + 'a,
{
    #[inline]
    fn from(row: Centerbox<'a, Message, Theme, Renderer>) -> Self {
        Self::new(row)
    }
}
