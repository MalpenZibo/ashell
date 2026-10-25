use crate::components::motion::{EASE_OUT, INSTANT};
use iced::{
    Animation, Background, Color, Length, Padding, Point, Rectangle, Size, Vector,
    core::{
        Clipboard, Layout, Shell, Widget, event, keyboard, layout, mouse, overlay, renderer, touch,
        widget::{Operation, Tree, tree},
    },
    widget::button::{Catalog, Status, Style, StyleFn},
};
use std::time::Instant;

type Element<'a, Message, Theme, Renderer> = iced::core::Element<'a, Message, Theme, Renderer>;

#[derive(Debug, Clone, Copy)]
pub struct ButtonUIRef {
    pub position: Point,
    pub viewport: (f32, f32),
}

/// Which mouse button a [`PressHandler`] is bound to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

impl MouseButton {
    pub const ALL: [Self; 3] = [Self::Left, Self::Right, Self::Middle];

    pub const fn index(self) -> usize {
        match self {
            Self::Left => 0,
            Self::Right => 1,
            Self::Middle => 2,
        }
    }
}

/// What a [`PositionButton`] does for an input event: publish a plain message,
/// or build one from the button's on-screen position (needed to place a popup
/// under it).
pub enum PressHandler<'a, Message> {
    Message(Message),
    WithPosition(Box<dyn Fn(ButtonUIRef) -> Message + 'a>),
}

impl<'a, Message> PressHandler<'a, Message> {
    pub fn message(message: Message) -> Self {
        Self::Message(message)
    }

    pub fn with_position(handler: impl Fn(ButtonUIRef) -> Message + 'a) -> Self {
        Self::WithPosition(Box::new(handler))
    }
}

pub struct PositionButton<'a, Message, Theme = iced::Theme, Renderer = iced::Renderer>
where
    Renderer: iced::core::Renderer,
    Theme: Catalog,
{
    content: Element<'a, Message, Theme, Renderer>,
    handlers: [Option<PressHandler<'a, Message>>; 3],
    on_scroll_up: Option<PressHandler<'a, Message>>,
    on_scroll_down: Option<PressHandler<'a, Message>>,
    on_hover: Option<PressHandler<'a, Message>>,
    on_unhover: Option<PressHandler<'a, Message>>,
    width: Length,
    height: Length,
    padding: Padding,
    clip: bool,
    class: Theme::Class<'a>,
}

impl<'a, Message, Theme, Renderer> PositionButton<'a, Message, Theme, Renderer>
where
    Renderer: iced::core::Renderer,
    Theme: Catalog,
{
    pub fn new(content: impl Into<Element<'a, Message, Theme, Renderer>>) -> Self {
        let content = content.into();
        let size = content.as_widget().size_hint();

        PositionButton {
            content,
            handlers: [None, None, None],
            on_scroll_up: None,
            on_scroll_down: None,
            on_hover: None,
            on_unhover: None,
            width: size.width.fluid(),
            height: size.height.fluid(),
            padding: DEFAULT_PADDING,
            clip: false,
            class: Theme::default(),
        }
    }

    /// Sets the width of the [`Button`].
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    /// Sets the height of the [`Button`].
    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();
        self
    }

    /// Sets the [`Padding`] of the [`Button`].
    pub fn padding<P: Into<Padding>>(mut self, padding: P) -> Self {
        self.padding = padding.into();
        self
    }

    /// Binds `handler` to `button`.
    pub fn on(mut self, button: MouseButton, handler: PressHandler<'a, Message>) -> Self {
        self.handlers[button.index()] = Some(handler);
        self
    }

    pub fn on_scroll_up(mut self, on_scroll_up: Message) -> Self {
        self.on_scroll_up = Some(PressHandler::message(on_scroll_up));
        self
    }

    pub fn on_scroll_down(mut self, on_scroll_down: Message) -> Self {
        self.on_scroll_down = Some(PressHandler::message(on_scroll_down));
        self
    }

    pub fn on_hover(mut self, on_hover: PressHandler<'a, Message>) -> Self {
        self.on_hover = Some(on_hover);
        self
    }

    pub fn on_unhover(mut self, on_unhover: Message) -> Self {
        self.on_unhover = Some(PressHandler::message(on_unhover));
        self
    }

    /// Sets whether the contents of the [`Button`] should be clipped on
    /// overflow.
    pub fn clip(mut self, clip: bool) -> Self {
        self.clip = clip;
        self
    }

    /// Sets the style of the [`Button`].
    #[must_use]
    pub fn style(mut self, style: impl Fn(&Theme, Status) -> Style + 'a) -> Self
    where
        Theme::Class<'a>: From<StyleFn<'a, Theme>>,
    {
        self.class = (Box::new(style) as StyleFn<'a, Theme>).into();
        self
    }

    fn publish_handler(
        handler: &PressHandler<'a, Message>,
        layout: Layout<'_>,
        viewport: &Rectangle,
        shell: &mut Shell<'_, Message>,
    ) where
        Message: Clone,
    {
        match handler {
            PressHandler::Message(message) => {
                shell.publish(message.clone());
            }
            PressHandler::WithPosition(handler) => {
                let ui_data = ButtonUIRef {
                    position: Point::new(
                        layout.bounds().width / 2. + layout.position().x,
                        layout.bounds().height / 2. + layout.position().y,
                    ),
                    viewport: (viewport.width, viewport.height),
                };
                shell.publish(handler(ui_data));
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct State {
    is_hovered: bool,
    pressed: [bool; 3],
    is_focused: bool,
}

/// `0.0` at rest to `1.0` hovered, so the highlight follows the pointer instead
/// of appearing in one frame.
struct Highlight(Animation<f32>);

impl Highlight {
    fn new() -> Self {
        Self(Animation::new(0.0).duration(INSTANT).easing(EASE_OUT))
    }
}

/// Blends two button styles. Theme styles only paint solid backgrounds, so a
/// gradient falls back to the target style.
fn fade_style(from: Style, to: Style, t: f32) -> Style {
    let lerp = crate::theme::lerp_color;
    Style {
        background: match (from.background, to.background) {
            (Some(Background::Color(a)), Some(Background::Color(b))) => {
                Some(Background::Color(lerp(a, b, t)))
            }
            (None, Some(Background::Color(b))) => Some(Background::Color(b.scale_alpha(t))),
            (_, to) => to,
        },
        text_color: lerp(from.text_color, to.text_color, t),
        border: to.border,
        shadow: to.shadow,
        snap: to.snap,
    }
}

fn pressed_button(event: &event::Event) -> Option<MouseButton> {
    match event {
        event::Event::Mouse(mouse::Event::ButtonPressed(button)) => match *button {
            mouse::Button::Left => Some(MouseButton::Left),
            mouse::Button::Right => Some(MouseButton::Right),
            mouse::Button::Middle => Some(MouseButton::Middle),
            _ => None,
        },
        event::Event::Touch(touch::Event::FingerPressed { .. }) => Some(MouseButton::Left),
        _ => None,
    }
}

fn released_button(event: &event::Event) -> Option<MouseButton> {
    match event {
        event::Event::Mouse(mouse::Event::ButtonReleased(button)) => match *button {
            mouse::Button::Left => Some(MouseButton::Left),
            mouse::Button::Right => Some(MouseButton::Right),
            mouse::Button::Middle => Some(MouseButton::Middle),
            _ => None,
        },
        event::Event::Touch(touch::Event::FingerLifted { .. }) => Some(MouseButton::Left),
        _ => None,
    }
}

impl<'a, Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for PositionButton<'a, Message, Theme, Renderer>
where
    Message: 'a + Clone,
    Renderer: 'a + iced::core::Renderer,
    Theme: Catalog,
{
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<(State, Highlight)>()
    }

    fn state(&self) -> tree::State {
        tree::State::new((State::default(), Highlight::new()))
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
    }

    fn size(&self) -> Size<Length> {
        Size {
            width: self.width,
            height: self.height,
        }
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        layout::padded(limits, self.width, self.height, self.padding, |limits| {
            self.content
                .as_widget_mut()
                .layout(&mut tree.children[0], renderer, limits)
        })
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

        if shell.is_event_captured() {
            return;
        }

        let is_over = cursor.is_over(layout.bounds());

        if let Some(button) = pressed_button(event) {
            let index = button.index();
            if self.handlers[index].is_some() && is_over {
                let (state, highlight) = tree.state.downcast_mut::<(State, Highlight)>();
                state.pressed[index] = true;
                highlight.0.go_mut(1.0, Instant::now());
                shell.request_redraw();
                shell.capture_event();
                return;
            }
        }

        if let Some(button) = released_button(event) {
            let index = button.index();
            if let Some(handler) = self.handlers[index].as_ref() {
                let state = tree.state.downcast_mut::<State>();
                if state.pressed[index] {
                    state.pressed[index] = false;
                    shell.request_redraw();
                    if is_over {
                        Self::publish_handler(handler, layout, viewport, shell);
                    }
                    shell.capture_event();
                    return;
                }
            }
        }

        match event {
            event::Event::Mouse(mouse::Event::WheelScrolled { delta }) => {
                let y = match delta {
                    mouse::ScrollDelta::Lines { y, .. } => Some(*y),
                    mouse::ScrollDelta::Pixels { y, .. } => Some(*y),
                };
                if is_over && let Some(y) = y {
                    let target = if y > 0.0 {
                        self.on_scroll_up.as_ref()
                    } else if y < 0.0 {
                        self.on_scroll_down.as_ref()
                    } else {
                        None
                    };

                    if let Some(handler) = target {
                        Self::publish_handler(handler, layout, viewport, shell);
                        shell.capture_event();
                        return;
                    }
                }
            }
            event::Event::Keyboard(keyboard::Event::KeyPressed { key, .. }) => {
                if let Some(handler) = self.handlers[MouseButton::Left.index()].as_ref() {
                    let state = tree.state.downcast_mut::<State>();
                    if state.is_focused
                        && matches!(key, keyboard::Key::Named(keyboard::key::Named::Enter))
                    {
                        state.pressed[MouseButton::Left.index()] = true;
                        Self::publish_handler(handler, layout, viewport, shell);
                        shell.capture_event();
                        return;
                    }
                }
            }
            event::Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                let state = tree.state.downcast_mut::<State>();
                if is_over {
                    if !state.is_hovered
                        && let Some(on_hover) = self.on_hover.as_ref()
                    {
                        state.is_hovered = true;
                        Self::publish_handler(on_hover, layout, viewport, shell);
                    }
                } else if state.is_hovered {
                    state.is_hovered = false;
                    if let Some(on_unhover) = self.on_unhover.as_ref() {
                        Self::publish_handler(on_unhover, layout, viewport, shell);
                    }
                }
            }
            event::Event::Touch(touch::Event::FingerLost { .. })
            | event::Event::Mouse(mouse::Event::CursorLeft) => {
                let state = tree.state.downcast_mut::<State>();
                if state.is_hovered {
                    state.is_hovered = false;
                    if let Some(on_unhover) = self.on_unhover.as_ref() {
                        Self::publish_handler(on_unhover, layout, viewport, shell);
                    }
                }
                state.pressed = [false; 3];
            }
            _ => {}
        }

        let actionable = self.handlers.iter().any(Option::is_some) || self.on_hover.is_some();
        let is_hovered = actionable && is_over;
        let (state, highlight) = tree.state.downcast_mut::<(State, Highlight)>();
        let was_hovered = state.is_hovered;
        state.is_hovered = is_hovered;

        if is_hovered != was_hovered {
            highlight
                .0
                .go_mut(if is_hovered { 1.0 } else { 0.0 }, Instant::now());
            shell.request_redraw();
        }

        if let event::Event::Window(iced::core::window::Event::RedrawRequested(now)) = event
            && highlight.0.is_animating(*now)
        {
            shell.request_redraw();
        }
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        _renderer_style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        let content_layout = layout.children().next().unwrap();
        let (state, highlight) = tree.state.downcast_ref::<(State, Highlight)>();
        let highlight = highlight.0.interpolate_with(|v| v, Instant::now());

        let disabled = !self.handlers.iter().any(Option::is_some) && self.on_hover.is_none();
        let resting = if state.is_hovered {
            if state.pressed[MouseButton::Left.index()] {
                Status::Pressed
            } else {
                Status::Hovered
            }
        } else {
            Status::Active
        };

        let style = if disabled {
            theme.style(&self.class, Status::Disabled)
        } else if highlight <= 0.001 || resting == Status::Active {
            theme.style(&self.class, Status::Active)
        } else {
            fade_style(
                theme.style(&self.class, Status::Active),
                theme.style(&self.class, resting),
                highlight,
            )
        };

        if style.background.is_some() || style.border.width > 0.0 || style.shadow.color.a > 0.0 {
            renderer.fill_quad(
                renderer::Quad {
                    bounds,
                    border: style.border,
                    shadow: style.shadow,
                    snap: true,
                },
                style
                    .background
                    .unwrap_or(Background::Color(Color::TRANSPARENT)),
            );
        }

        let viewport = if self.clip {
            bounds.intersection(viewport).unwrap_or(*viewport)
        } else {
            *viewport
        };

        self.content.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            &renderer::Style {
                text_color: style.text_color,
            },
            content_layout,
            cursor,
            &viewport,
        );
    }

    fn mouse_interaction(
        &self,
        _tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
        _renderer: &Renderer,
    ) -> mouse::Interaction {
        let is_mouse_over = cursor.is_over(layout.bounds());

        if is_mouse_over && self.handlers.iter().any(Option::is_some) {
            mouse::Interaction::Pointer
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

impl<'a, Message, Theme, Renderer> From<PositionButton<'a, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: Clone + 'a,
    Theme: Catalog + 'a,
    Renderer: iced::core::Renderer + 'a,
{
    #[inline]
    fn from(button: PositionButton<'a, Message, Theme, Renderer>) -> Self {
        Self::new(button)
    }
}

pub fn position_button<'a, Message, Theme, Renderer>(
    content: impl Into<Element<'a, Message, Theme, Renderer>>,
) -> PositionButton<'a, Message, Theme, Renderer>
where
    Theme: Catalog + 'a,
    Renderer: iced::core::Renderer,
{
    PositionButton::new(content)
}

/// The default [`Padding`] of a [`Button`].
pub(crate) const DEFAULT_PADDING: Padding = Padding {
    top: 5.0,
    bottom: 5.0,
    right: 10.0,
    left: 10.0,
};
