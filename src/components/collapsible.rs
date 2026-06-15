use iced::{
    Animation, Length, Rectangle, Size, Vector,
    animation::Easing,
    core::{
        Clipboard, Layout, Shell, Widget, event, layout, mouse, overlay, renderer,
        widget::{Operation, Tree, tree},
    },
};
use std::time::{Duration, Instant};

type Element<'a, Message, Theme, Renderer> = iced::core::Element<'a, Message, Theme, Renderer>;

struct State {
    height_anim: Animation<f32>,
    last_expanded: bool,
    initialized: bool,
}

impl State {
    fn new() -> Self {
        Self {
            height_anim: Animation::new(0.0),
            last_expanded: false,
            initialized: false,
        }
    }
}

/// State-driven collapse/expand: stays in the tree and animates height
/// between 0 and the child's natural height based on `expanded`.
pub struct Collapsible<'a, Message, Theme = iced::Theme, Renderer = iced::Renderer>
where
    Renderer: iced::core::Renderer,
{
    content: Element<'a, Message, Theme, Renderer>,
    expanded: bool,
    duration: Duration,
    easing: Easing,
    animated: bool,
    open_padding_top: f32,
    open_padding_bottom: f32,
}

impl<'a, Message, Theme, Renderer> Collapsible<'a, Message, Theme, Renderer>
where
    Renderer: iced::core::Renderer,
{
    pub fn duration(mut self, duration: Duration) -> Self {
        self.duration = duration;
        self
    }

    pub fn easing(mut self, easing: Easing) -> Self {
        self.easing = easing;
        self
    }

    /// When false, snap between expanded/collapsed without tweening.
    pub fn animated(mut self, animated: bool) -> Self {
        self.animated = animated;
        self
    }

    /// Top padding baked into the animated height, so neighbors with
    /// `spacing=0` still see a gap open up.
    pub fn open_padding_top(mut self, padding: f32) -> Self {
        self.open_padding_top = padding;
        self
    }

    /// Bottom counterpart of [`open_padding_top`](Self::open_padding_top),
    /// for Collapsibles sitting above their trigger.
    pub fn open_padding_bottom(mut self, padding: f32) -> Self {
        self.open_padding_bottom = padding;
        self
    }
}

impl<'a, Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for Collapsible<'a, Message, Theme, Renderer>
where
    Message: Clone + 'a,
    Theme: 'a,
    Renderer: iced::core::Renderer + 'a,
{
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::new())
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
    }

    fn size(&self) -> Size<Length> {
        Size::new(Length::Shrink, Length::Shrink)
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let state = tree.state.downcast_mut::<State>();
        let now = Instant::now();

        // Skip child layout when settled-collapsed: some widgets (scrollables)
        // misbehave in zero-height limits.
        if !self.expanded
            && state.initialized
            && !state.last_expanded
            && !state.height_anim.is_animating(now)
        {
            return layout::Node::new(Size::ZERO);
        }

        let child_node =
            self.content
                .as_widget_mut()
                .layout(&mut tree.children[0], renderer, limits);
        let child_width = child_node.size().width;
        let child_height = child_node.size().height;

        let expanded_height = child_height + self.open_padding_top + self.open_padding_bottom;
        let target_height = if self.expanded { expanded_height } else { 0.0 };

        if !self.animated {
            state.height_anim = Animation::new(target_height);
            state.last_expanded = self.expanded;
            state.initialized = true;
        } else if !state.initialized {
            state.height_anim = Animation::new(target_height)
                .duration(self.duration)
                .easing(self.easing);
            state.last_expanded = self.expanded;
            state.initialized = true;
        } else if self.expanded != state.last_expanded {
            state.last_expanded = self.expanded;
            state.height_anim.go_mut(target_height, now);
        } else if self.expanded
            && (expanded_height - state.height_anim.value()).abs() > 0.5
            && !state.height_anim.is_animating(now)
        {
            // Content height changed while expanded — snap to new height
            state.height_anim = Animation::new(expanded_height)
                .duration(self.duration)
                .easing(self.easing);
        }

        let display_height = if state.height_anim.is_animating(now) {
            state.height_anim.interpolate_with(|v| v, now)
        } else {
            target_height
        };

        let positioned_child = child_node.translate(Vector::new(0.0, self.open_padding_top));
        layout::Node::with_children(
            Size::new(child_width, display_height),
            vec![positioned_child],
        )
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
        if let Some(child_layout) = layout.children().next() {
            self.content.as_widget_mut().update(
                &mut tree.children[0],
                event,
                child_layout,
                cursor,
                renderer,
                clipboard,
                shell,
                viewport,
            );
        }

        if let event::Event::Window(iced::core::window::Event::RedrawRequested(now)) = event {
            let state = tree.state.downcast_mut::<State>();
            if state.height_anim.is_animating(*now) {
                shell.request_redraw();
                shell.invalidate_layout();
            }
        }
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
        let bounds = layout.bounds();
        if bounds.height < 0.5 || layout.children().next().is_none() {
            return;
        }
        // A nested scrollable re-clips to its own bounds ∩ viewport (the layer
        // clip resets rather than intersects), so pass our animating bounds as
        // the viewport or its content spills past the shrinking height.
        let Some(child_viewport) = bounds.intersection(viewport) else {
            return;
        };
        renderer.with_layer(bounds, |renderer| {
            self.content.as_widget().draw(
                &tree.children[0],
                renderer,
                theme,
                style,
                layout.children().next().unwrap(),
                cursor,
                &child_viewport,
            );
        });
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        if let Some(child_layout) = layout.children().next() {
            self.content.as_widget_mut().operate(
                &mut tree.children[0],
                child_layout,
                renderer,
                operation,
            );
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
        if let Some(child_layout) = layout.children().next() {
            self.content.as_widget().mouse_interaction(
                &tree.children[0],
                child_layout,
                cursor,
                viewport,
                renderer,
            )
        } else {
            mouse::Interaction::default()
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
        self.content.as_widget_mut().overlay(
            &mut tree.children[0],
            layout.children().next()?,
            renderer,
            viewport,
            translation,
        )
    }
}

impl<'a, Message, Theme, Renderer> From<Collapsible<'a, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: Clone + 'a,
    Theme: 'a,
    Renderer: iced::core::Renderer + 'a,
{
    fn from(widget: Collapsible<'a, Message, Theme, Renderer>) -> Self {
        Self::new(widget)
    }
}

pub fn collapsible<'a, Message, Theme, Renderer>(
    expanded: bool,
    content: impl Into<Element<'a, Message, Theme, Renderer>>,
) -> Collapsible<'a, Message, Theme, Renderer>
where
    Renderer: iced::core::Renderer,
{
    Collapsible {
        content: content.into(),
        expanded,
        duration: Duration::from_millis(200),
        easing: Easing::EaseOutCubic,
        animated: true,
        open_padding_top: 0.0,
        open_padding_bottom: 0.0,
    }
}
