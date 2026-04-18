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
    width_anim: Animation<f32>,
    last_child_width: f32,
    initialized: bool,
}

impl State {
    fn new(duration: Duration, easing: Easing) -> Self {
        Self {
            width_anim: Animation::new(0.0).duration(duration).easing(easing),
            last_child_width: 0.0,
            initialized: false,
        }
    }
}

/// A widget that automatically animates width changes of its content.
///
/// Wrap any element in `AnimatedSize` and it will smoothly transition
/// when the content's natural width changes. No messages, subscriptions,
/// or manual state management needed.
///
/// # Example
/// ```ignore
/// animated_size(text("Hello"))
///     .duration(Duration::from_millis(400))
/// ```
pub struct AnimatedSize<'a, Message, Theme = iced::Theme, Renderer = iced::Renderer>
where
    Renderer: iced::core::Renderer,
{
    content: Element<'a, Message, Theme, Renderer>,
    duration: Duration,
    easing: Easing,
}

impl<'a, Message, Theme, Renderer> AnimatedSize<'a, Message, Theme, Renderer>
where
    Renderer: iced::core::Renderer,
{
    /// Sets the animation duration.
    pub fn duration(mut self, duration: Duration) -> Self {
        self.duration = duration;
        self
    }

    /// Sets the easing function.
    pub fn easing(mut self, easing: Easing) -> Self {
        self.easing = easing;
        self
    }
}

impl<'a, Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for AnimatedSize<'a, Message, Theme, Renderer>
where
    Message: Clone + 'a,
    Theme: 'a,
    Renderer: iced::core::Renderer + 'a,
{
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::new(self.duration, self.easing))
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
        // Layout child with unconstrained width to get its natural size
        let child_node =
            self.content
                .as_widget_mut()
                .layout(&mut tree.children[0], renderer, limits);
        let child_width = child_node.size().width;
        let child_height = child_node.size().height;

        let state = tree.state.downcast_mut::<State>();

        if !state.initialized {
            // First layout: set initial width without animation
            state.width_anim = Animation::new(child_width)
                .duration(self.duration)
                .easing(self.easing);
            state.last_child_width = child_width;
            state.initialized = true;
        } else if (child_width - state.last_child_width).abs() > 0.5 {
            // Child size changed: start animation from current position to new size
            state.last_child_width = child_width;
            state.width_anim.go_mut(child_width, Instant::now());
        }

        let now = Instant::now();
        let display_width = if state.width_anim.is_animating(now) {
            state.width_anim.interpolate_with(|v| v, now)
        } else {
            child_width
        };

        // Our node wraps the child but reports the animated width
        layout::Node::with_children(Size::new(display_width, child_height), vec![child_node])
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
        // Forward events to child
        self.content.as_widget_mut().update(
            &mut tree.children[0],
            event,
            layout.children().next().unwrap(),
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );

        // Drive animation on RedrawRequested
        if let event::Event::Window(iced::core::window::Event::RedrawRequested(_now)) = event {
            let state = tree.state.downcast_mut::<State>();
            if state.width_anim.is_animating(Instant::now()) {
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

        // Clip to our animated bounds
        renderer.with_layer(bounds, |renderer| {
            self.content.as_widget().draw(
                &tree.children[0],
                renderer,
                theme,
                style,
                layout.children().next().unwrap(),
                cursor,
                viewport,
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
        self.content.as_widget_mut().operate(
            &mut tree.children[0],
            layout.children().next().unwrap(),
            renderer,
            operation,
        );
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.content.as_widget().mouse_interaction(
            &tree.children[0],
            layout.children().next().unwrap(),
            cursor,
            viewport,
            renderer,
        )
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

impl<'a, Message, Theme, Renderer> From<AnimatedSize<'a, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: Clone + 'a,
    Theme: 'a,
    Renderer: iced::core::Renderer + 'a,
{
    fn from(widget: AnimatedSize<'a, Message, Theme, Renderer>) -> Self {
        Self::new(widget)
    }
}

/// Wraps content in a widget that automatically animates width changes.
pub fn animated_size<'a, Message, Theme, Renderer>(
    content: impl Into<Element<'a, Message, Theme, Renderer>>,
) -> AnimatedSize<'a, Message, Theme, Renderer>
where
    Renderer: iced::core::Renderer,
{
    AnimatedSize {
        content: content.into(),
        duration: Duration::from_millis(400),
        easing: Easing::EaseOutCubic,
    }
}
