//! Distribute content horizontally.
use iced::advanced::layout::{self, Layout, Limits, Node};
use iced::advanced::overlay;
use iced::advanced::renderer;
use iced::advanced::widget::{Operation, Tree};
use iced::advanced::{Clipboard, Shell, Widget, mouse};
use iced::{
    Alignment, Element, Event, Length, Padding, Pixels, Point, Rectangle, Size, Vector, event,
};

/// A container that distributes its contents horizontally.
#[allow(missing_debug_implementations)]
pub struct Centerbox<'a, Message, Theme = iced::Theme, Renderer = iced::Renderer> {
    spacing: f32,
    padding: Padding,
    width: Length,
    height: Length,
    align_items: Alignment,
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
            children,
        }
    }

    /// Sets the horizontal spacing _between_ elements.
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

    /// Sets the vertical alignment of the contents of the [`Centerbox`] .
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
    fn children(&self) -> Vec<Tree> {
        self.children.iter().map(Tree::new).collect()
    }

    fn diff(&mut self, tree: &mut Tree) {
        tree.diff_children(&mut self.children)
    }

    fn size(&self) -> Size<Length> {
        Size {
            width: self.width,
            height: self.height,
        }
    }

    fn layout(
        &self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let limits = limits
            .width(self.width)
            .height(self.height)
            .shrink(self.padding);

        let total_spacing = self.spacing * 3_i32.saturating_sub(1) as f32;
        let max_cross = limits.max().height;

        let mut cross = match self.height {
            Length::Shrink => 0.0,
            _ => max_cross,
        };

        let available = limits.max().width - total_spacing;

        let mut nodes = [Node::default(), Node::default(), Node::default()];

        let mut remaining = match self.width {
            Length::Shrink => 0.0,
            _ => available.max(0.0),
        };

        let mut calculate_edge_layout =
            |i: usize, (child, tree): (&Element<'a, Message, Theme, Renderer>, &mut Tree)| {
                let fill_cross_factor = {
                    let size = child.as_widget().size();

                    size.height.fill_factor()
                };

                let (max_width, max_height) = (
                    remaining,
                    if fill_cross_factor != 0 {
                        cross
                    } else {
                        max_cross
                    },
                );

                let child_limits = Limits::new(Size::ZERO, Size::new(max_width, max_height));

                let layout = child.as_widget().layout(tree, renderer, &child_limits);
                let size = layout.size();

                remaining -= size.width;
                cross = cross.max(size.height);

                nodes[i] = layout;
            };

        calculate_edge_layout(0, (&self.children[0], &mut tree.children[0]));
        calculate_edge_layout(2, (&self.children[2], &mut tree.children[2]));
        calculate_edge_layout(1, (&self.children[1], &mut tree.children[1]));

        nodes[0].move_to_mut(Point::new(self.padding.left, self.padding.top));
        nodes[0].align_mut(Alignment::Start, self.align_items, Size::new(0.0, cross));
        nodes[2].move_to_mut(Point::new(
            limits.max().width + self.padding.right,
            self.padding.top,
        ));
        nodes[2].align_mut(Alignment::End, self.align_items, Size::new(0.0, cross));

        let half_available = available / 2.0;
        let half_center_width = nodes[1].size().width / 2.0;

        if half_available - nodes[0].size().width < half_center_width
            || half_available - nodes[2].size().width < half_center_width
        {
            nodes[1].move_to_mut(Point::new(
                self.padding.left
                    + self.spacing
                    + nodes[0].size().width
                    + (available - nodes[0].size().width - nodes[2].size().width) / 2.0,
                self.padding.top,
            ));
        } else {
            nodes[1].move_to_mut(Point::new(
                limits.max().width / 2. + self.padding.horizontal() / 2.0,
                self.padding.top,
            ));
        }
        nodes[1].align_mut(Alignment::Center, self.align_items, Size::new(0.0, cross));

        let main =
            nodes[0].size().width + nodes[1].size().width + nodes[2].size().width + total_spacing;

        let (intrinsic_width, intrinsic_height) = (main, cross);
        let size = limits.resolve(
            self.width,
            self.height,
            Size::new(intrinsic_width, intrinsic_height),
        );

        Node::with_children(size.expand(self.padding), nodes.into())
    }

    fn operate(
        &self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        operation.container(None, layout.bounds(), &mut |operation| {
            self.children
                .iter()
                .zip(&mut tree.children)
                .zip(layout.children())
                .for_each(|((child, state), layout)| {
                    child
                        .as_widget()
                        .operate(state, layout, renderer, operation);
                });
        });
    }

    fn on_event(
        &mut self,
        tree: &mut Tree,
        event: Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) -> event::Status {
        self.children
            .iter_mut()
            .zip(&mut tree.children)
            .zip(layout.children())
            .map(|((child, state), layout)| {
                child.as_widget_mut().on_event(
                    state,
                    event.clone(),
                    layout,
                    cursor,
                    renderer,
                    clipboard,
                    shell,
                    viewport,
                )
            })
            .fold(event::Status::Ignored, event::Status::merge)
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
        layout: Layout<'_>,
        renderer: &Renderer,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        overlay::from_children(&mut self.children, tree, layout, renderer, translation)
    }
}

impl<'a, Message, Theme, Renderer> From<Centerbox<'a, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: 'a,
    Theme: 'a,
    Renderer: iced::advanced::Renderer + 'a,
{
    fn from(row: Centerbox<'a, Message, Theme, Renderer>) -> Self {
        Self::new(row)
    }
}
