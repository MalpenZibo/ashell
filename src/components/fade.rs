//! Fades content in and out.
//!
//! iced paints a frame only when a widget asks for the next one, so a fade
//! cannot be driven from application state alone. [`Fade`] keeps its tween in
//! tree state and requests frames until it settles.
//!
//! iced has no alpha layer, so only the content's default text colour fades.
//! Anything that paints its own colours keeps them.

use crate::components::motion::{EASE_IN, EASE_OUT};
use iced::{
    Animation, Border, Color, Length, Rectangle, Shadow, Size, Vector,
    core::{
        Clipboard, Layout, Shell, Widget, event, layout, mouse, overlay, renderer,
        widget::{Operation, Tree, tree},
    },
};
use std::time::Instant;

type Element<'a, Message, Theme, Renderer> = iced::core::Element<'a, Message, Theme, Renderer>;

struct State {
    fade: Animation<f32>,
    last_visible: bool,
    initialized: bool,
}

impl State {
    fn opacity(&self, visible: bool, now: Instant) -> f32 {
        let target = if visible { 1.0 } else { 0.0 };
        let value = self.fade.interpolate_with(|v| v, now);
        // Snap near the target so rounding cannot leave a ghost on screen.
        if (value - target).abs() < 0.001 {
            target
        } else {
            value
        }
    }
}

pub struct Fade<'a, Message, Theme = iced::Theme, Renderer = iced::Renderer>
where
    Renderer: iced::core::Renderer,
{
    content: Element<'a, Message, Theme, Renderer>,
    visible: bool,
    duration: std::time::Duration,
}

impl<'a, Message, Theme, Renderer> Fade<'a, Message, Theme, Renderer>
where
    Renderer: iced::core::Renderer,
{
    pub fn duration(mut self, duration: std::time::Duration) -> Self {
        self.duration = duration;
        self
    }
}

impl<'a, Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for Fade<'a, Message, Theme, Renderer>
where
    Message: 'a,
    Theme: 'a,
    Renderer: iced::core::Renderer + 'a,
{
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State {
            fade: Animation::new(if self.visible { 1.0 } else { 0.0 })
                .duration(self.duration)
                .easing(EASE_OUT),
            last_visible: self.visible,
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
        self.content.as_widget().size()
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let node = self
            .content
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits);

        let state = tree.state.downcast_mut::<State>();
        let now = Instant::now();

        if !state.initialized {
            state.fade = Animation::new(if self.visible { 0.0 } else { 1.0 })
                .duration(self.duration)
                .easing(EASE_OUT);
            state.last_visible = self.visible;
            state.initialized = true;
            if self.visible {
                state.fade.go_mut(1.0, now);
            }
        } else if self.visible != state.last_visible {
            state.last_visible = self.visible;
            let current = state.opacity(self.visible, now);
            state.fade = Animation::new(current)
                .duration(self.duration)
                .easing(if self.visible { EASE_OUT } else { EASE_IN });
            state.fade.go_mut(if self.visible { 1.0 } else { 0.0 }, now);
        }

        layout::Node::with_children(node.size(), vec![node])
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

        if let event::Event::Window(iced::core::window::Event::RedrawRequested(now)) = event
            && tree.state.downcast_mut::<State>().fade.is_animating(*now)
        {
            shell.request_redraw();
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
        let opacity = tree
            .state
            .downcast_ref::<State>()
            .opacity(self.visible, Instant::now());
        if opacity <= 0.0 {
            return;
        }

        let Some(child_layout) = layout.children().next() else {
            return;
        };
        let Some(viewport) = layout.bounds().intersection(viewport) else {
            return;
        };

        self.content.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            &renderer::Style {
                text_color: style.text_color.scale_alpha(opacity),
            },
            child_layout,
            cursor,
            &viewport,
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

impl<'a, Message, Theme, Renderer> From<Fade<'a, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: 'a,
    Theme: 'a,
    Renderer: iced::core::Renderer + 'a,
{
    fn from(widget: Fade<'a, Message, Theme, Renderer>) -> Self {
        Self::new(widget)
    }
}

/// A panel that fades as one piece: background, border and text.
pub struct Panel<'a, Message, Theme = iced::Theme, Renderer = iced::Renderer>
where
    Renderer: iced::core::Renderer,
{
    inner: Fade<'a, Message, Theme, Renderer>,
    background: Option<Color>,
    border: Border,
}

impl<'a, Message, Theme, Renderer> Panel<'a, Message, Theme, Renderer>
where
    Renderer: iced::core::Renderer,
{
    pub fn background(mut self, background: Color) -> Self {
        self.background = Some(background);
        self
    }

    pub fn border(mut self, border: Border) -> Self {
        self.border = border;
        self
    }
}

impl<'a, Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for Panel<'a, Message, Theme, Renderer>
where
    Message: 'a,
    Theme: 'a,
    Renderer: iced::core::Renderer + 'a,
{
    fn tag(&self) -> tree::Tag {
        self.inner.tag()
    }

    fn state(&self) -> tree::State {
        self.inner.state()
    }

    fn children(&self) -> Vec<Tree> {
        self.inner.children()
    }

    fn diff(&self, tree: &mut Tree) {
        self.inner.diff(tree);
    }

    fn size(&self) -> Size<Length> {
        self.inner.size()
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.inner.layout(tree, renderer, limits)
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
        self.inner.update(
            tree, event, layout, cursor, renderer, clipboard, shell, viewport,
        );
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
        let opacity = tree
            .state
            .downcast_ref::<State>()
            .opacity(self.inner.visible, Instant::now());
        if opacity <= 0.0 {
            return;
        }

        // The background and border are the panel itself, so they fade with it
        // instead of appearing at full strength under fading content.
        if self.background.is_some() || self.border.width > 0.0 {
            renderer.fill_quad(
                renderer::Quad {
                    bounds: layout.bounds(),
                    border: Border {
                        color: self.border.color.scale_alpha(opacity),
                        ..self.border
                    },
                    shadow: Shadow::default(),
                    snap: true,
                },
                self.background
                    .unwrap_or(Color::TRANSPARENT)
                    .scale_alpha(opacity),
            );
        }

        self.inner
            .draw(tree, renderer, theme, style, layout, cursor, viewport);
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        self.inner.operate(tree, layout, renderer, operation);
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.inner
            .mouse_interaction(tree, layout, cursor, viewport, renderer)
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        self.inner
            .overlay(tree, layout, renderer, viewport, translation)
    }
}

impl<'a, Message, Theme, Renderer> From<Panel<'a, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: 'a,
    Theme: 'a,
    Renderer: iced::core::Renderer + 'a,
{
    fn from(widget: Panel<'a, Message, Theme, Renderer>) -> Self {
        Self::new(widget)
    }
}

/// Fades `content` towards `visible`.
pub fn fade<'a, Message, Theme, Renderer>(
    visible: bool,
    content: impl Into<Element<'a, Message, Theme, Renderer>>,
) -> Fade<'a, Message, Theme, Renderer>
where
    Renderer: iced::core::Renderer,
{
    Fade {
        content: content.into(),
        visible,
        duration: crate::components::motion::BASE,
    }
}

/// A fading panel around `content`.
pub fn panel<'a, Message, Theme, Renderer>(
    content: impl Into<Element<'a, Message, Theme, Renderer>>,
) -> Panel<'a, Message, Theme, Renderer>
where
    Renderer: iced::core::Renderer,
{
    Panel {
        inner: fade(true, content),
        background: None,
        border: Border::default(),
    }
}
