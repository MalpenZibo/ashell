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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnimationAxis {
    Width,
    Height,
    Both,
}

struct State {
    width_anim: Animation<f32>,
    height_anim: Animation<f32>,
    last_child_width: f32,
    last_child_height: f32,
    initialized: bool,
}

impl State {
    fn new(duration: Duration, easing: Easing) -> Self {
        let anim = || Animation::new(0.0).duration(duration).easing(easing);
        Self {
            width_anim: anim(),
            height_anim: anim(),
            last_child_width: 0.0,
            last_child_height: 0.0,
            initialized: false,
        }
    }
}

/// Smoothly animates size changes of its content along one or both axes.
/// Defaults to width-only; configure with [`axis`](Self::axis).
pub struct AnimatedSize<'a, Message, Theme = iced::Theme, Renderer = iced::Renderer>
where
    Renderer: iced::core::Renderer,
{
    content: Element<'a, Message, Theme, Renderer>,
    duration: Duration,
    easing: Easing,
    axis: AnimationAxis,
    animate_initial: bool,
}

impl<'a, Message, Theme, Renderer> AnimatedSize<'a, Message, Theme, Renderer>
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

    pub fn axis(mut self, axis: AnimationAxis) -> Self {
        self.axis = axis;
        self
    }

    /// Animate from zero on the first layout (for appear/disappear elements).
    pub fn animate_initial(mut self, animate: bool) -> Self {
        self.animate_initial = animate;
        self
    }
}

impl<'a, Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for AnimatedSize<'a, Message, Theme, Renderer>
where
    Message: 'a,
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
        let child_node =
            self.content
                .as_widget_mut()
                .layout(&mut tree.children[0], renderer, limits);
        let child_width = child_node.size().width;
        let child_height = child_node.size().height;

        let state = tree.state.downcast_mut::<State>();
        let now = Instant::now();
        let animate_width = matches!(self.axis, AnimationAxis::Width | AnimationAxis::Both);
        let animate_height = matches!(self.axis, AnimationAxis::Height | AnimationAxis::Both);

        if !state.initialized {
            let initial_width = if self.animate_initial {
                0.0
            } else {
                child_width
            };
            let initial_height = if self.animate_initial {
                0.0
            } else {
                child_height
            };
            state.width_anim = Animation::new(initial_width)
                .duration(self.duration)
                .easing(self.easing);
            state.height_anim = Animation::new(initial_height)
                .duration(self.duration)
                .easing(self.easing);
            state.last_child_width = child_width;
            state.last_child_height = child_height;
            state.initialized = true;
            if self.animate_initial {
                if animate_width {
                    state.width_anim.go_mut(child_width, now);
                }
                if animate_height {
                    state.height_anim.go_mut(child_height, now);
                }
            }
        } else {
            if animate_width && (child_width - state.last_child_width).abs() > 0.5 {
                state.last_child_width = child_width;
                state.width_anim.go_mut(child_width, now);
            }
            if animate_height && (child_height - state.last_child_height).abs() > 0.5 {
                state.last_child_height = child_height;
                state.height_anim.go_mut(child_height, now);
            }
        }

        let display_width = if animate_width && state.width_anim.is_animating(now) {
            state.width_anim.interpolate_with(|v| v, now)
        } else {
            child_width
        };

        let display_height = if animate_height && state.height_anim.is_animating(now) {
            state.height_anim.interpolate_with(|v| v, now)
        } else {
            child_height
        };

        layout::Node::with_children(Size::new(display_width, display_height), vec![child_node])
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

        if let event::Event::Window(iced::core::window::Event::RedrawRequested(now)) = event {
            let state = tree.state.downcast_mut::<State>();
            if state.width_anim.is_animating(*now) || state.height_anim.is_animating(*now) {
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
        if bounds.width < 0.5 || bounds.height < 0.5 {
            return;
        }
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
    Message: 'a,
    Theme: 'a,
    Renderer: iced::core::Renderer + 'a,
{
    fn from(widget: AnimatedSize<'a, Message, Theme, Renderer>) -> Self {
        Self::new(widget)
    }
}

/// Wraps content in an [`AnimatedSize`] with default settings (width-only, 100ms).
pub fn animated_size<'a, Message, Theme, Renderer>(
    content: impl Into<Element<'a, Message, Theme, Renderer>>,
) -> AnimatedSize<'a, Message, Theme, Renderer>
where
    Renderer: iced::core::Renderer,
{
    AnimatedSize {
        content: content.into(),
        duration: Duration::from_millis(100),
        easing: Easing::EaseOutCubic,
        axis: AnimationAxis::Width,
        animate_initial: false,
    }
}
