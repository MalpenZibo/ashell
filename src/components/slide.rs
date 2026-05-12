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

#[derive(Debug, Clone, Copy)]
pub enum SlideDirection {
    Left,
    Right,
}

struct State {
    offset_anim: Animation<f32>,
    last_visible: bool,
    initialized: bool,
    key: u64,
}

/// Slides its content horizontally in/out based on `visible`.
/// Used for toast notifications to slide in/out from the screen edge.
pub const DEFAULT_DURATION: Duration = Duration::from_millis(200);

pub struct Slide<'a, Message, Theme = iced::Theme, Renderer = iced::Renderer>
where
    Renderer: iced::core::Renderer,
{
    content: Element<'a, Message, Theme, Renderer>,
    visible: bool,
    direction: SlideDirection,
    slide_distance: f32,
    duration: Duration,
    key: u64,
    animated: bool,
}

impl<'a, Message, Theme, Renderer> Slide<'a, Message, Theme, Renderer>
where
    Renderer: iced::core::Renderer,
{
    /// Sets a unique key that ties the widget's animation state to a specific
    /// item. When the key changes (e.g. a different toast occupies this tree
    /// position), state is reset so the new item doesn't inherit animations
    /// from the previous one.
    pub fn key(mut self, key: u64) -> Self {
        self.key = key;
        self
    }

    /// Disables the slide animation; the widget snaps between
    /// visible and off-screen positions instead of tweening.
    pub fn animated(mut self, animated: bool) -> Self {
        self.animated = animated;
        self
    }

    fn off_screen(&self) -> f32 {
        match self.direction {
            SlideDirection::Right => self.slide_distance,
            SlideDirection::Left => -self.slide_distance,
        }
    }
}

impl<'a, Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for Slide<'a, Message, Theme, Renderer>
where
    Message: Clone + 'a,
    Theme: 'a,
    Renderer: iced::core::Renderer + 'a,
{
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State {
            offset_anim: Animation::new(0.0),
            last_visible: false,
            initialized: false,
            key: self.key,
        })
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut Tree) {
        let state = tree.state.downcast_mut::<State>();
        if state.key != self.key {
            // Different content now occupies this tree position. Settle state
            // at the current target (no animation) — the content was already
            // visible in its previous slot, so we shouldn't replay the slide-in.
            let settled_at = if self.visible { 0.0 } else { self.off_screen() };
            state.offset_anim = Animation::new(settled_at);
            state.last_visible = self.visible;
            state.initialized = true;
            state.key = self.key;
        }
        tree.diff_children(std::slice::from_ref(&self.content));
    }

    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
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
        let child_size = child_node.size();

        let state = tree.state.downcast_mut::<State>();
        let now = Instant::now();
        let off_screen = self.off_screen();

        if !self.animated {
            let settled_at = if self.visible { 0.0 } else { off_screen };
            state.offset_anim = Animation::new(settled_at);
            state.last_visible = self.visible;
            state.initialized = true;
        } else if !state.initialized {
            // Always start off-screen, animate in if visible
            state.offset_anim = Animation::new(off_screen)
                .duration(self.duration)
                .easing(Easing::EaseOutCubic);
            state.last_visible = self.visible;
            state.initialized = true;
            if self.visible {
                state.offset_anim.go_mut(0.0, now);
            }
        } else if self.visible != state.last_visible {
            state.last_visible = self.visible;
            let target = if self.visible { 0.0 } else { off_screen };
            state.offset_anim.go_mut(target, now);
        }

        let offset = if state.offset_anim.is_animating(now) {
            state.offset_anim.interpolate_with(|v| v, now)
        } else if self.visible {
            0.0
        } else {
            off_screen
        };

        let translated_child = child_node.translate(Vector::new(offset, 0.0));
        layout::Node::with_children(child_size, vec![translated_child])
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
            if state.offset_anim.is_animating(*now) {
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
        let Some(child_layout) = layout.children().next() else {
            return;
        };
        self.content.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            child_layout,
            cursor,
            viewport,
        );
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

impl<'a, Message, Theme, Renderer> From<Slide<'a, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: Clone + 'a,
    Theme: 'a,
    Renderer: iced::core::Renderer + 'a,
{
    fn from(widget: Slide<'a, Message, Theme, Renderer>) -> Self {
        Self::new(widget)
    }
}

pub fn slide<'a, Message, Theme, Renderer>(
    visible: bool,
    direction: SlideDirection,
    slide_distance: f32,
    content: impl Into<Element<'a, Message, Theme, Renderer>>,
) -> Slide<'a, Message, Theme, Renderer>
where
    Renderer: iced::core::Renderer,
{
    Slide {
        content: content.into(),
        visible,
        direction,
        slide_distance,
        duration: DEFAULT_DURATION,
        key: 0,
        animated: true,
    }
}
