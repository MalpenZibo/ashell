use iced::advanced::layout::{self, Layout};
use iced::advanced::renderer;
use iced::advanced::widget::{Operation, Tree};
use iced::advanced::{Clipboard, Shell, Widget, mouse};
use iced::core::widget::tree;
use iced::id::Id;
use iced::{
    Background, Border, Color, Element, Event, Length, Padding, Point, Rectangle, Shadow, Size,
    Vector, alignment, event, overlay, touch,
};

#[allow(missing_debug_implementations)]
pub struct MenuWrapper<'a, Message, Theme = iced::Theme, Renderer = iced::Renderer> {
    id: Id,
    x: f32,
    content: Element<'a, Message, Theme, Renderer>,
    on_click_outside: Option<Message>,
    padding: Padding,
    vertical_alignment: alignment::Vertical,
    backdrop: Option<Color>,
}

impl<'a, Message, Theme, Renderer> MenuWrapper<'a, Message, Theme, Renderer>
where
    Renderer: iced::advanced::Renderer,
{
    pub fn new(x: f32, content: Element<'a, Message, Theme, Renderer>) -> Self {
        MenuWrapper {
            id: Id::unique(),
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
    Renderer: iced::advanced::Renderer,
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

    fn diff(&mut self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_mut(&mut self.content));
    }

    fn size(&self) -> Size<Length> {
        Size {
            width: Length::Fill,
            height: Length::Fill,
        }
    }

    fn layout(
        &self,
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
                    .as_widget()
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
        )
    }

    fn operate(
        &self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        operation.container(None, layout.bounds(), &mut |operation| {
            self.content.as_widget().operate(
                &mut tree.children[0],
                layout.children().next().unwrap(),
                renderer,
                operation,
            );
        });
    }

    fn on_event(
        &mut self,
        tree: &mut Tree,
        event: Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) -> event::Status {
        if let event::Status::Captured = self.content.as_widget_mut().on_event(
            &mut tree.children[0],
            event.clone(),
            layout.children().next().unwrap(),
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        ) {
            return event::Status::Captured;
        }

        if let Some(on_click_outside) = &self.on_click_outside
            && let Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
            | Event::Touch(touch::Event::FingerLifted { .. }) = event
        {
            let bounds = layout.children().next().unwrap().bounds();
            let cursor_over_scrollable = cursor.is_over(bounds);
            if !cursor_over_scrollable {
                shell.publish(on_click_outside.clone());

                return event::Status::Captured;
            }
        }

        event::Status::Ignored
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
        layout: Layout<'_>,
        renderer: &Renderer,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        self.content.as_widget_mut().overlay(
            &mut tree.children[0],
            layout.children().next().unwrap(),
            renderer,
            translation,
        )
    }

    fn id(&self) -> Option<Id> {
        Some(self.id.clone())
    }

    fn set_id(&mut self, id: Id) {
        self.id = id;
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
