use iced::advanced::layout::{self, Layout};
use iced::advanced::renderer;
use iced::advanced::widget::{Operation, Tree};
use iced::advanced::{Clipboard, Shell, Widget, mouse};
use iced::animation::Easing;
use iced::core::widget::tree;
use iced::{
    Animation, Background, Border, Color, Length, Padding, Point, Rectangle, Shadow, Size, Vector,
    alignment, event, overlay, touch,
};
use std::time::Instant;

use crate::components::menu::ANIMATION_DURATION;

type Element<'a, Message, Theme, Renderer> = iced::core::Element<'a, Message, Theme, Renderer>;

struct State {
    progress_anim: Animation<f32>,
    last_open: bool,
    initialized: bool,
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

    fn slide_from_top(&self) -> bool {
        matches!(self.vertical_alignment, alignment::Vertical::Top)
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
                let x = f32::min(
                    f32::max(self.x - content_size.width / 2.0, 8.),
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
        // hides everything past `progress * height`. Anchored to the bar edge
        // so the menu "rolls out" from there.
        let full_height = content_bounds.height;
        let visible_height = full_height * progress;
        let clip_bounds = if self.slide_from_top() {
            Rectangle {
                x: content_bounds.x,
                y: content_bounds.y,
                width: content_bounds.width,
                height: visible_height,
            }
        } else {
            Rectangle {
                x: content_bounds.x,
                y: content_bounds.y + full_height - visible_height,
                width: content_bounds.width,
                height: visible_height,
            }
        };

        renderer.with_layer(clip_bounds, |renderer| {
            self.content.as_widget().draw(
                &tree.children[0],
                renderer,
                theme,
                renderer_style,
                content_layout,
                cursor,
                viewport,
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
