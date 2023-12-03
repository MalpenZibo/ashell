use iced::{
    advanced::{
        layout::{self, Node},
        mouse, overlay, renderer,
        widget::{Operation, OperationOutputWrapper, Tree},
        Clipboard, Layout, Shell, Widget,
    },
    event, Alignment, Element, Event, Length, Limits, Padding, Pixels, Point, Rectangle, Size,
};

#[allow(missing_debug_implementations)]
pub struct Centerbox<'a, Message, Renderer = iced::Renderer> {
    spacing: f32,
    padding: Padding,
    width: Length,
    height: Length,
    align_items: Alignment,
    children: [Element<'a, Message, Renderer>; 3],
}

impl<'a, Message, Renderer> Centerbox<'a, Message, Renderer> {
    pub fn new(children: [Element<'a, Message, Renderer>; 3]) -> Self {
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

    /// Sets the [`Padding`] of the [`Row`].
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

impl<'a, Message, Renderer> Widget<Message, Renderer> for Centerbox<'a, Message, Renderer>
where
    Renderer: iced::advanced::Renderer,
{
    fn children(&self) -> Vec<Tree> {
        self.children.iter().map(Tree::new).collect()
    }

    fn diff(&mut self, tree: &mut Tree) {
        tree.diff_children(&mut self.children)
    }

    fn width(&self) -> Length {
        self.width
    }

    fn height(&self) -> Length {
        self.height
    }

    fn layout(&self, renderer: &Renderer, limits: &layout::Limits) -> layout::Node {
        let limits_without_padding = limits.width(self.width).height(self.height);

        let spacing = self.spacing;
        let padding = self.padding;
        let align_items = self.align_items;
        let items = &self.children;

        let total_spacing = spacing * items.len().saturating_sub(1) as f32;
        let limits = limits_without_padding.pad(padding);
        let max_height = limits.max().height;

        let mut height = limits.min().height.max(limits.fill().height);
        let mut available_width = limits.max().width - total_spacing;

        let mut nodes: [Node; 3] = [Node::default(), Node::default(), Node::default()];
        let mut edge_nodes_layout = |i: usize, child: &Element<'a, Message, Renderer>| {
            let (max_width, max_height) = (available_width, max_height);

            let child_limits = Limits::new(Size::ZERO, Size::new(max_width, max_height));

            let layout = child.as_widget().layout(renderer, &child_limits);

            let size = layout.size();

            available_width -= size.width;
            height = height.max(size.height);

            nodes[i] = layout;
        };

        edge_nodes_layout(0, &items[0]);
        edge_nodes_layout(2, &items[2]);

        let remaining_width = available_width.max(0.0);

        let child_limits = Limits::new(Size::ZERO, Size::new(remaining_width, max_height));

        let layout = items[1].as_widget().layout(renderer, &child_limits);

        height = height.max(layout.size().height);

        nodes[1] = layout;

        let left_width = nodes[0].size().width;
        let right_width = nodes[2].size().width;

        nodes[0].move_to(Point::new(padding.left, padding.top));
        nodes[0].align(Alignment::Start, align_items, Size::new(0.0, height));

        nodes[2].move_to(Point::new(
            limits_without_padding.max().width - padding.right,
            padding.top,
        ));
        nodes[2].align(Alignment::End, align_items, Size::new(0.0, height));

        let half_available_width = limits_without_padding.max().width / 2.;
        let half_width = nodes[1].size().width / 2.0;
        if half_available_width - right_width < half_width {
            nodes[1].move_to(Point::new(
                half_available_width
                    - (half_width - (half_available_width - right_width)).abs()
                    - spacing,
                padding.top,
            ));
        } else if half_available_width - left_width < half_width {
            nodes[1].move_to(Point::new(
                half_available_width
                    + (half_width - (half_available_width - left_width)).abs()
                    + spacing,
                padding.top,
            ));
        } else {
            nodes[1].move_to(Point::new(half_available_width, padding.top));
        }
        nodes[1].align(Alignment::Center, align_items, Size::new(0.0, height));

        let size = limits.resolve(Size::new(limits.max().width, height));

        Node::with_children(size.pad(padding), nodes.to_vec())
    }

    fn operate(
        &self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation<OperationOutputWrapper<Message>>,
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
        theme: &Renderer::Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        for ((child, state), layout) in self
            .children
            .iter()
            .zip(&tree.children)
            .zip(layout.children())
        {
            child
                .as_widget()
                .draw(state, renderer, theme, style, layout, cursor, viewport);
        }
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
    ) -> Option<overlay::Element<'b, Message, Renderer>> {
        overlay::from_children(&mut self.children, tree, layout, renderer)
    }
}

impl<'a, Message, Renderer> From<Centerbox<'a, Message, Renderer>>
    for Element<'a, Message, Renderer>
where
    Message: 'a,
    Renderer: iced::advanced::Renderer + 'a,
{
    fn from(centerbox: Centerbox<'a, Message, Renderer>) -> Self {
        Self::new(centerbox)
    }
}
