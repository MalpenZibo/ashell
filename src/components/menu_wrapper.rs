use crate::components::motion::{BASE, EASE_IN, EASE_IN_OUT, EASE_OUT, FAST};
use iced::advanced::layout::{self, Layout};
use iced::advanced::renderer;
use iced::advanced::widget::{Operation, Tree};
use iced::advanced::{Clipboard, Shell, Widget, mouse};
use iced::core::widget::tree;
use iced::{
    Animation, Background, Border, Color, Length, Padding, Point, Rectangle, Shadow, Size, Vector,
    alignment, event, overlay, touch,
};
use std::time::{Duration, Instant};

type Element<'a, Message, Theme, Renderer> = iced::core::Element<'a, Message, Theme, Renderer>;

/// Below this the tween counts as settled, so it cannot leave a sliver of
/// menu on screen or keep a redraw loop alive.
const SETTLED_EPSILON: f32 = 0.001;

/// How far the content travels while opening. Small values read as the menu
/// landing rather than flying in from the bar.
const REVEAL_DISTANCE: f32 = 8.0;

struct State {
    /// Horizontal position of the menu, in window coordinates.
    x_anim: Animation<f32>,
    /// Open amount, `0.0` closed through `1.0` open.
    progress_anim: Animation<f32>,
    last_open: bool,
    last_x: f32,
    initialized: bool,
}

impl State {
    fn new(x: f32) -> Self {
        Self {
            x_anim: Animation::new(x),
            progress_anim: Animation::new(0.0),
            last_open: false,
            last_x: x,
            initialized: false,
        }
    }

    /// How open the menu currently is, in `0.0..=1.0`.
    fn progress(&self, open: bool, now: Instant) -> f32 {
        let target = if open { 1.0 } else { 0.0 };
        let progress = self.progress_anim.interpolate_with(|v| v, now);

        // Snap once within `SETTLED_EPSILON` so no resampling error can leave a
        // sliver of a closed menu, or a hairline gap under an open one.
        if (progress - target).abs() < SETTLED_EPSILON {
            target
        } else {
            progress
        }
    }

    /// Advances both tweens to this build's targets and returns the x position
    /// to draw at. Retargeting from the current value keeps a reversal smooth.
    fn animate(
        &mut self,
        x: f32,
        open: bool,
        duration: Duration,
        animated: bool,
        now: Instant,
    ) -> f32 {
        if !animated {
            self.x_anim = Animation::new(x);
            self.progress_anim = Animation::new(if open { 1.0 } else { 0.0 });
            self.last_x = x;
            self.last_open = open;
            self.initialized = true;
            return x;
        }

        if !self.initialized {
            self.initialized = true;
            self.x_anim = Animation::new(x);
            self.last_x = x;
            self.progress_anim = Animation::new(if open { 0.0 } else { 1.0 })
                .duration(duration)
                .easing(EASE_OUT);
            self.last_open = open;
            if open {
                self.progress_anim.go_mut(1.0, now);
            }
            return x;
        }

        if open != self.last_open {
            self.last_open = open;
            let current = self.progress(open, now);
            self.progress_anim = Animation::new(current).duration(duration).easing(if open {
                EASE_OUT
            } else {
                EASE_IN
            });
            self.progress_anim.go_mut(if open { 1.0 } else { 0.0 }, now);
        }

        if (x - self.last_x).abs() > 0.5 {
            self.last_x = x;
            // Travelling between buttons covers more ground than a reveal, so
            // it gets slightly more time.
            let current = self.x_anim.interpolate_with(|v| v, now);
            self.x_anim = Animation::new(current)
                .duration(duration + FAST)
                .easing(EASE_IN_OUT);
            self.x_anim.go_mut(x, now);
        }

        self.x_anim.interpolate_with(|v| v, now)
    }
}

#[allow(missing_debug_implementations)]
pub struct MenuWrapper<'a, Message, Theme = iced::Theme, Renderer = iced::Renderer> {
    x: f32,
    content: Element<'a, Message, Theme, Renderer>,
    on_click_outside: Option<Message>,
    padding: Padding,
    vertical_alignment: alignment::Vertical,
    backdrop: Option<Color>,
    open: bool,
    animated: bool,
    duration: Duration,
}

impl<'a, Message, Theme, Renderer> MenuWrapper<'a, Message, Theme, Renderer>
where
    Renderer: iced::advanced::Renderer,
{
    pub fn new(x: f32, content: Element<'a, Message, Theme, Renderer>) -> Self {
        MenuWrapper {
            x,
            content,
            on_click_outside: None,
            vertical_alignment: alignment::Vertical::Top,
            padding: Padding::ZERO,
            backdrop: None,
            open: true,
            animated: true,
            duration: BASE,
        }
    }

    pub fn padding<P: Into<Padding>>(mut self, padding: P) -> Self {
        self.padding = padding.into();
        self
    }

    pub fn align_y(mut self, alignment: impl Into<alignment::Vertical>) -> Self {
        self.vertical_alignment = alignment.into();
        self
    }

    pub fn on_click_outside(mut self, message: Message) -> Self {
        self.on_click_outside = Some(message);
        self
    }

    pub fn backdrop(mut self, color: Color) -> Self {
        self.backdrop = Some(color);
        self
    }

    pub fn open(mut self, open: bool) -> Self {
        self.open = open;
        self
    }

    pub fn animated(mut self, animated: bool) -> Self {
        self.animated = animated;
        self
    }

    /// Sets the open/close tween duration.
    pub fn duration(mut self, duration: Duration) -> Self {
        self.duration = duration;
        self
    }

    fn slide_from_top(&self) -> bool {
        matches!(self.vertical_alignment, alignment::Vertical::Top)
    }

    /// Direction the content travels from while opening: down from the top
    /// bar, up from the bottom bar.
    fn slide_axis(&self) -> f32 {
        if self.slide_from_top() { 1.0 } else { -1.0 }
    }

    /// How far the content has yet to travel, in pixels.
    ///
    /// `layout` offsets the content by this and `draw` shifts the mask by the
    /// same amount, so the visible edge stays pinned to the bar while the menu
    /// settles underneath.
    fn reveal_offset(&self, progress: f32) -> f32 {
        -self.slide_axis() * REVEAL_DISTANCE * (1.0 - progress)
    }
}

impl<'a, Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for MenuWrapper<'a, Message, Theme, Renderer>
where
    Message: Clone,
    Renderer: iced::advanced::Renderer,
{
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::new(self.x))
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
    }

    fn size(&self) -> Size<Length> {
        Size {
            width: Length::Fill,
            height: Length::Fill,
        }
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let state = tree.state.downcast_mut::<State>();
        let now = Instant::now();

        let display_x = state.animate(self.x, self.open, self.duration, self.animated, now);

        let node = layout::positioned(
            limits,
            Length::Fill,
            Length::Fill,
            self.padding,
            |limits| {
                self.content
                    .as_widget_mut()
                    .layout(&mut tree.children[0], renderer, limits)
            },
            |node, size| {
                let content_size = node.size();
                let x = f32::min(
                    f32::max(display_x - content_size.width / 2.0, 8.),
                    size.width - content_size.width - 8.,
                );
                let node = node.align(
                    iced::Alignment::Center,
                    self.vertical_alignment.into(),
                    size,
                );
                let y = node.bounds().y;
                node.move_to(Point::new(x, y))
            },
        );

        // While opening, the content travels a few pixels into place behind the
        // clip.
        let progress_value = state.progress(self.open, now);
        let slide = self.reveal_offset(progress_value);
        if slide.abs() > 0.01 {
            node.translate(Vector::new(0.0, slide))
        } else {
            node
        }
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
            self.content.as_widget_mut().operate(
                &mut tree.children[0],
                layout.children().next().unwrap(),
                renderer,
                operation,
            );
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
            if state.progress_anim.is_animating(*now) || state.x_anim.is_animating(*now) {
                shell.request_redraw();
                shell.invalidate_layout();
            }
        }

        // Ignore click-outside while the close animation plays — otherwise a
        // late click could re-emit CloseMenu and trigger spurious work.
        if !self.open {
            return;
        }

        if let Some(on_click_outside) = &self.on_click_outside
            && let event::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
            | event::Event::Touch(touch::Event::FingerLifted { .. }) = event
        {
            let bounds = layout.children().next().unwrap().bounds();
            let cursor_over_content = cursor.is_over(bounds);
            if !cursor_over_content {
                shell.publish(on_click_outside.clone());
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
        // This wrapper is the root of the whole menu layer (backdrop included),
        // so it has to delegate to the content: text fields inside a menu keep
        // their I-beam cursor. While the menu is closing it is no longer
        // interactive, so don't forward anything from a surface that is about
        // to be destroyed.
        if !self.open {
            return mouse::Interaction::default();
        }

        self.content.as_widget().mouse_interaction(
            &tree.children[0],
            layout.children().next().unwrap(),
            cursor,
            viewport,
            renderer,
        )
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        renderer_style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_ref::<State>();
        let now = Instant::now();
        let progress = state.progress(self.open, now);

        if let Some(backdrop) = self.backdrop {
            let mut backdrop = backdrop;
            // The backdrop tracks the menu so the two read as one motion.
            backdrop.a *= progress;
            renderer.fill_quad(
                renderer::Quad {
                    bounds: layout.bounds(),
                    border: Border::default(),
                    shadow: Shadow::default(),
                    snap: true,
                },
                Background::Color(backdrop),
            );
        }

        if progress <= 0.0 {
            return;
        }

        let content_layout = layout.children().next().unwrap();
        let content_bounds = content_layout.bounds();

        // The content is drawn at full size and revealed by a growing clip,
        // so the menu rolls out rather than sliding in as a block.
        let full_height = content_bounds.height;
        let visible_height = full_height * progress;
        let edge = content_bounds.y;
        let clip_bounds = if self.slide_from_top() {
            Rectangle {
                x: content_bounds.x,
                y: edge,
                width: content_bounds.width,
                height: visible_height,
            }
        } else {
            Rectangle {
                x: content_bounds.x,
                y: edge + full_height - visible_height,
                width: content_bounds.width,
                height: visible_height,
            }
        };
        // The layer clip is invisible to children; `viewport` is what they read.
        let Some(child_viewport) = clip_bounds.intersection(viewport) else {
            return;
        };
        renderer.with_layer(clip_bounds, |renderer| {
            self.content.as_widget().draw(
                &tree.children[0],
                renderer,
                theme,
                renderer_style,
                content_layout,
                cursor,
                &child_viewport,
            );
        });
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
            layout.children().next().unwrap(),
            renderer,
            viewport,
            translation,
        )
    }
}

impl<'a, Message, Theme, Renderer> From<MenuWrapper<'a, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: 'a + Clone,
    Theme: 'a,
    Renderer: iced::advanced::Renderer + 'a,
{
    fn from(menu_wrapper: MenuWrapper<'a, Message, Theme, Renderer>) -> Self {
        Self::new(menu_wrapper)
    }
}
