use iced_layershell::advanced::layout::{self, Layout};
use iced_layershell::advanced::renderer;
use iced_layershell::advanced::widget::{Operation, Tree};
use iced_layershell::advanced::{Clipboard, Shell, Widget, mouse};
use iced_layershell::core::widget::tree;
use iced_layershell::{
    Background, Border, Color, Length, Padding, Point, Rectangle, Shadow, Size,
    Vector, alignment, event, overlay, touch,
};

type Element<'a, Message, Theme, Renderer> =
    iced_layershell::core::Element<'a, Message, Theme, Renderer>;

#[allow(missing_debug_implementations)]
pub struct MenuWrapper<'a, Message, Theme = iced_layershell::Theme, Renderer = iced_layershell::Renderer> {
    x: f32,
    content: Element<'a, Message, Theme, Renderer>,
    on_click_outside: Option<Message>,
    padding: Padding,
    vertical_alignment: alignment::Vertical,
    backdrop: Option<Color>,
}

impl<'a, Message, Theme, Renderer> MenuWrapper<'a, Message, Theme, Renderer>
where
    Renderer: iced_layershell::advanced::Renderer,
{
    pub fn new(x: f32, content: Element<'a, Message, Theme, Renderer>) -> Self {
        MenuWrapper {
            x,
            content,
            on_click_outside: None,
            vertical_alignment: alignment::Vertical::Top,
            padding: Padding::ZERO,
            backdrop: None,
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
}

impl<'a, Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for MenuWrapper<'a, Message, Theme, Renderer>
where
    Message: Clone,
    Renderer: iced_layershell::advanced::Renderer,
{
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<()>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(())
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
        layout::positioned(
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
                    iced_layershell::Alignment::Center,
                    self.vertical_alignment.into(),
                    size,
                );
                let y = node.bounds().y;
                node.move_to(Point::new(x, y))
            },
        )
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
        if let Some(backdrop) = self.backdrop {
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

        let content_layout = layout.children().next().unwrap();
        self.content.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            renderer_style,
            content_layout,
            cursor,
            viewport,
        );
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
    Renderer: iced_layershell::advanced::Renderer + 'a,
{
    fn from(menu_wrapper: MenuWrapper<'a, Message, Theme, Renderer>) -> Self {
        Self::new(menu_wrapper)
    }
}
