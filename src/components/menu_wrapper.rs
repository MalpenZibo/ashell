use iced::advanced::layout::{self, Layout};
use iced::advanced::renderer;
use iced::advanced::widget::{Operation, Tree};
use iced::advanced::{Clipboard, Shell, Widget, mouse};
use iced::animation::Easing;
use iced::core::widget::tree;
use iced::{
    Alignment, Animation, Background, Border, Color, Length, Padding, Rectangle, Shadow, Size,
    Vector, event, overlay, touch,
};
use std::time::Instant;

use crate::components::menu::ANIMATION_DURATION;

use super::{Axis, BarEdge};

type Element<'a, Message, Theme, Renderer> = iced::core::Element<'a, Message, Theme, Renderer>;

struct State {
    progress_anim: Animation<f32>,
    last_open: bool,
    initialized: bool,
}

#[allow(missing_debug_implementations)]
pub struct MenuWrapper<'a, Message, Theme = iced::Theme, Renderer = iced::Renderer> {
    cross: f32,
    anchor: BarEdge,
    content: Element<'a, Message, Theme, Renderer>,
    on_click_outside: Option<Message>,
    padding: Padding,
    backdrop: Option<Color>,
    open: bool,
    animated: bool,
}

impl<'a, Message, Theme, Renderer> MenuWrapper<'a, Message, Theme, Renderer>
where
    Renderer: iced::advanced::Renderer,
{
    pub fn new(cross: f32, content: Element<'a, Message, Theme, Renderer>) -> Self {
        MenuWrapper {
            cross,
            anchor: BarEdge::Top,
            content,
            on_click_outside: None,
            padding: Padding::ZERO,
            backdrop: None,
            open: true,
            animated: true,
        }
    }

    pub fn padding<P: Into<Padding>>(mut self, padding: P) -> Self {
        self.padding = padding.into();
        self
    }

    pub fn anchor(mut self, anchor: BarEdge) -> Self {
        self.anchor = anchor;
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
        tree::State::new(State {
            progress_anim: Animation::new(0.0),
            last_open: false,
            initialized: false,
        })
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
                let axis = self.anchor.axis();

                // `self.cross` is the button's centre along the clamp axis;
                // clamp the content to that axis with an 8px inset.
                let cross_value = f32::min(
                    f32::max(self.cross - axis.main(content_size) / 2.0, 8.),
                    axis.main(size) - axis.main(content_size) - 8.,
                );

                // Pin along the anchored axis. `align` is additive (the node
                // starts at the padding origin), so the anchor inset travels
                // through to the final position exactly like before.
                let (align_x, align_y) =
                    axis.align(Alignment::Center, self.anchor.anchor_alignment());
                let aligned = node.align(align_x, align_y, size);
                let aligned_pos = aligned.bounds().position();
                let anchor_value = match axis {
                    Axis::Horizontal => aligned_pos.y,
                    Axis::Vertical => aligned_pos.x,
                };

                aligned.move_to(axis.point(cross_value, anchor_value))
            },
        );

        let state = tree.state.downcast_mut::<State>();
        let now = Instant::now();

        if !self.animated {
            let target = if self.open { 1.0 } else { 0.0 };
            state.progress_anim = Animation::new(target);
            state.last_open = self.open;
            state.initialized = true;
        } else if !state.initialized {
            let initial = if self.open { 0.0 } else { 1.0 };
            state.progress_anim = Animation::new(initial)
                .duration(ANIMATION_DURATION)
                .easing(Easing::EaseOutCubic);
            state.last_open = self.open;
            state.initialized = true;
            if self.open {
                state.progress_anim.go_mut(1.0, now);
            }
        } else if self.open != state.last_open {
            state.last_open = self.open;
            let target = if self.open { 1.0 } else { 0.0 };
            state.progress_anim.go_mut(target, now);
        }

        node
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
            if state.progress_anim.is_animating(*now) {
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
            let cursor_over_scrollable = cursor.is_over(bounds);
            if !cursor_over_scrollable {
                shell.publish(on_click_outside.clone());
            }
        }
    }

    fn mouse_interaction(
        &self,
        _: &Tree,
        _: Layout<'_>,
        _: mouse::Cursor,
        _: &Rectangle,
        _: &Renderer,
    ) -> mouse::Interaction {
        mouse::Interaction::default()
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
        let progress = if state.progress_anim.is_animating(now) {
            state.progress_anim.interpolate_with(|v| v, now)
        } else if self.open {
            1.0
        } else {
            0.0
        };

        if let Some(backdrop) = self.backdrop {
            let mut backdrop = backdrop;
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

        if progress < 0.01 {
            return;
        }

        let content_layout = layout.children().next().unwrap();
        let content_bounds = content_layout.bounds();

        // Clip-reveal: content is drawn at full size, but a growing clip rect
        // hides everything past `progress * length`. Anchored to the bar edge
        // so the menu "rolls out" from there.
        let axis = self.anchor.axis();
        let is_start = self.anchor.is_start();
        let full_length = match axis {
            Axis::Horizontal => content_bounds.height,
            Axis::Vertical => content_bounds.width,
        };
        let visible_length = full_length * progress;

        let (clip_width, clip_height) = match axis {
            Axis::Horizontal => (content_bounds.width, visible_length),
            Axis::Vertical => (visible_length, content_bounds.height),
        };
        let (clip_x, clip_y) = match (axis, is_start) {
            (Axis::Horizontal, true) => (content_bounds.x, content_bounds.y),
            (Axis::Horizontal, false) => (
                content_bounds.x,
                content_bounds.y + full_length - visible_length,
            ),
            (Axis::Vertical, true) => (content_bounds.x, content_bounds.y),
            (Axis::Vertical, false) => (
                content_bounds.x + full_length - visible_length,
                content_bounds.y,
            ),
        };
        let clip_bounds = Rectangle {
            x: clip_x,
            y: clip_y,
            width: clip_width,
            height: clip_height,
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
