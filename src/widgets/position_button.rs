use iced::{
    Background, Color, Length, Padding, Point, Rectangle, Size, Vector,
    core::{
        Clipboard, Layout, Shell, Widget, event, keyboard, layout, mouse, overlay, renderer, touch,
        widget::{Operation, Tree, tree},
    },
    widget::button::{Catalog, Status, Style, StyleFn},
};

type Element<'a, Message, Theme, Renderer> = iced::core::Element<'a, Message, Theme, Renderer>;

#[derive(Debug, Clone, Copy)]
pub struct ButtonUIRef {
    pub position: Point,
    pub viewport: (f32, f32),
}

enum OnPress<'a, Message> {
    Message(Message),
    MessageWithPosition(Box<dyn Fn(ButtonUIRef) -> Message + 'a>),
}

pub struct PositionButton<'a, Message, Theme = iced::Theme, Renderer = iced::Renderer>
where
    Renderer: iced::core::Renderer,
    Theme: Catalog,
{
    content: Element<'a, Message, Theme, Renderer>,
    on_press: Option<OnPress<'a, Message>>,
    on_right_press: Option<OnPress<'a, Message>>,
    on_scroll_up: Option<OnPress<'a, Message>>,
    on_scroll_down: Option<OnPress<'a, Message>>,
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
            on_press: None,
            on_right_press: None,
            on_scroll_up: None,
            on_scroll_down: None,
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

    /// Sets the message that will be produced when the [`Button`] is pressed.
    ///
    /// Unless `on_press` is called, the [`Button`] will be disabled.
    pub fn on_press(mut self, on_press: Message) -> Self {
        self.on_press = Some(OnPress::Message(on_press));
        self
    }

    pub fn on_press_with_position(
        mut self,
        on_press: impl Fn(ButtonUIRef) -> Message + 'a,
    ) -> Self {
        self.on_press = Some(OnPress::MessageWithPosition(Box::new(on_press)));
        self
    }

    pub fn on_right_press(mut self, on_right_press: Message) -> Self {
        self.on_right_press = Some(OnPress::Message(on_right_press));
        self
    }

    pub fn on_scroll_up(mut self, on_scroll_up: Message) -> Self {
        self.on_scroll_up = Some(OnPress::Message(on_scroll_up));
        self
    }

    pub fn on_scroll_down(mut self, on_scroll_down: Message) -> Self {
        self.on_scroll_down = Some(OnPress::Message(on_scroll_down));
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

    fn publish_on_press(
        &self,
        on_press: &OnPress<'a, Message>,
        layout: Layout<'_>,
        viewport: &Rectangle,
        shell: &mut Shell<'_, Message>,
    ) where
        Message: Clone,
    {
        match on_press {
            OnPress::Message(message) => {
                shell.publish(message.clone());
            }
            OnPress::MessageWithPosition(on_press_with_position) => {
                let ui_data = ButtonUIRef {
                    position: Point::new(
                        layout.bounds().width / 2. + layout.position().x,
                        layout.bounds().height / 2. + layout.position().y,
                    ),
                    viewport: (viewport.width, viewport.height),
                };
                shell.publish(on_press_with_position(ui_data));
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct State {
    is_hovered: bool,
    is_pressed: bool,
    is_right_pressed: bool,
    is_focused: bool,
}

impl<'a, Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for PositionButton<'a, Message, Theme, Renderer>
where
    Message: 'a + Clone,
    Renderer: 'a + iced::core::Renderer,
    Theme: Catalog,
{
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::default())
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

        match event {
            event::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
            | event::Event::Touch(touch::Event::FingerPressed { .. })
                if self.on_press.is_some() =>
            {
                let bounds = layout.bounds();

                if cursor.is_over(bounds) {
                    let state = tree.state.downcast_mut::<State>();

                    state.is_pressed = true;

                    shell.capture_event();
                    return;
                }
            }
            event::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right))
                if self.on_right_press.is_some() =>
            {
                let bounds = layout.bounds();

                if cursor.is_over(bounds) {
                    let state = tree.state.downcast_mut::<State>();
                    state.is_right_pressed = true;
                    shell.capture_event();
                    return;
                }
            }
            event::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
            | event::Event::Touch(touch::Event::FingerLifted { .. }) => {
                if let Some(on_press) = self.on_press.as_ref() {
                    let state = tree.state.downcast_mut::<State>();

                    if state.is_pressed {
                        state.is_pressed = false;

                        let bounds = layout.bounds();

                        if cursor.is_over(bounds) {
                            self.publish_on_press(on_press, layout, viewport, shell);
                        }

                        shell.capture_event();
                        return;
                    }
                }
            }
            event::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Right)) => {
                if let Some(on_right_press) = self.on_right_press.as_ref() {
                    let state = tree.state.downcast_mut::<State>();

                    if state.is_right_pressed {
                        state.is_right_pressed = false;

                        let bounds = layout.bounds();

                        if cursor.is_over(bounds) {
                            self.publish_on_press(on_right_press, layout, viewport, shell);
                        }

                        shell.capture_event();
                        return;
                    }
                }
            }
            event::Event::Mouse(mouse::Event::WheelScrolled { delta }) => {
                let bounds = layout.bounds();
                let y = match delta {
                    mouse::ScrollDelta::Lines { y, .. } => Some(*y),
                    mouse::ScrollDelta::Pixels { y, .. } => Some(*y),
                };
                if cursor.is_over(bounds)
                    && let Some(y) = y
                {
                    let target = if y > 0.0 {
                        self.on_scroll_up.as_ref()
                    } else if y < 0.0 {
                        self.on_scroll_down.as_ref()
                    } else {
                        None
                    };

                    if let Some(on_scroll) = target {
                        self.publish_on_press(on_scroll, layout, viewport, shell);
                        shell.capture_event();
                        return;
                    }
                }
            }
            event::Event::Keyboard(keyboard::Event::KeyPressed { key, .. }) => {
                if let Some(on_press) = self.on_press.as_ref() {
                    let state = tree.state.downcast_mut::<State>();
                    if state.is_focused
                        && matches!(key, keyboard::Key::Named(keyboard::key::Named::Enter))
                    {
                        state.is_pressed = true;
                        match on_press {
                            OnPress::Message(message) => {
                                shell.publish(message.clone());
                            }
                            OnPress::MessageWithPosition(on_press) => {
                                let ui_data = ButtonUIRef {
                                    position: Point::new(
                                        layout.bounds().width / 2. + layout.position().x,
                                        layout.bounds().height / 2. + layout.position().y,
                                    ),
                                    viewport: (viewport.width, viewport.height),
                                };
                                shell.publish(on_press(ui_data));
                            }
                        }
                        shell.capture_event();
                        return;
                    }
                }
            }
            event::Event::Touch(touch::Event::FingerLost { .. })
            | event::Event::Mouse(mouse::Event::CursorLeft) => {
                let state = tree.state.downcast_mut::<State>();
                state.is_hovered = false;
                state.is_pressed = false;
            }
            _ => {}
        }

        // Reactive rendering: track hover state and request redraw on change
        let state = tree.state.downcast_mut::<State>();
        let is_hovered = self.on_press.is_some() && cursor.is_over(layout.bounds());
        if is_hovered != state.is_hovered {
            state.is_hovered = is_hovered;
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
        let state = tree.state.downcast_ref::<State>();

        let status = if self.on_press.is_none() {
            Status::Disabled
        } else if state.is_hovered {
            if state.is_pressed {
                Status::Pressed
            } else {
                Status::Hovered
            }
        } else {
            Status::Active
        };

        let style = theme.style(&self.class, status);

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

        if is_mouse_over && self.on_press.is_some() {
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
