//! Distribute content horizontally.
use iced::advanced::layout::{self, Layout, Limits, Node};
use iced::advanced::overlay;
use iced::advanced::renderer;
use iced::advanced::widget::{Operation, Tree};
use iced::advanced::{Clipboard, Shell, Widget, mouse};
use iced::core::clipboard::DndDestinationRectangles;
use iced::core::widget::tree;
use iced::id::Id;
use iced::{
    Alignment, Background, Border, Color, Element, Event, Length, Padding, Pixels, Point,
    Rectangle, Shadow, Size, Vector, event,
};

/// A container that distributes its contents horizontally.
#[allow(missing_debug_implementations)]
pub struct MenuWrapper<'a, Message, Theme = iced::Theme, Renderer = iced::Renderer> {
    x: f32,
    content: Element<'a, Message, Theme, Renderer>,
}

impl<'a, Message, Theme, Renderer> MenuWrapper<'a, Message, Theme, Renderer>
where
    Renderer: iced::advanced::Renderer,
{
    pub fn new(x: f32, content: Element<'a, Message, Theme, Renderer>) -> Self {
        MenuWrapper { x, content }
    }
}

impl<'a, Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for MenuWrapper<'a, Message, Theme, Renderer>
where
    Renderer: iced::advanced::Renderer,
{
    fn tag(&self) -> tree::Tag {
        self.content.as_widget().tag()
    }

    fn state(&self) -> tree::State {
        self.content.as_widget().state()
    }

    fn children(&self) -> Vec<Tree> {
        self.content.as_widget().children()
    }

    fn diff(&mut self, tree: &mut Tree) {
        self.content.as_widget_mut().diff(tree);
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
        let mut layout = self.content.as_widget().layout(tree, renderer, limits);
        let size = layout.size();

        let viewport_width = limits.max().width;

        let x = f32::min(
            f32::max(self.x - size.width / 2.0, 8.),
            viewport_width - size.width - 8.,
        );
        println!(
            "size {size:?}, viewport_width {viewport_width}, x {}, x position {x}",
            self.x
        );

        layout.move_to_mut(Point::new(x, 0.));

        Node::with_children(size, vec![layout])
    }

    fn operate(
        &self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        operation.container(
            self.content.as_widget().id().as_ref(),
            layout.bounds(),
            &mut |operation| {
                self.content.as_widget().operate(
                    tree,
                    layout
                        .children()
                        .next()
                        .unwrap()
                        .with_virtual_offset(layout.virtual_offset()),
                    renderer,
                    operation,
                );
            },
        );
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
            layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        ) {
            println!("click inside");
            return event::Status::Captured;
        }
        println!("click outside");
        return event::Status::Captured;
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
            tree,
            layout
                .children()
                .next()
                .unwrap()
                .with_virtual_offset(layout.virtual_offset()),
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
        renderer.fill_quad(
            renderer::Quad {
                bounds: layout.bounds(),
                border: Border::default(),
                shadow: Shadow::default(),
            },
            Background::Color(Color::from_rgb(0., 1., 0.)),
        );

        self.content.as_widget().draw(
            tree,
            renderer,
            theme,
            &renderer::Style {
                icon_color: renderer_style.icon_color,
                text_color: renderer_style.text_color,
                scale_factor: renderer_style.scale_factor,
            },
            layout
                .children()
                .next()
                .unwrap()
                .with_virtual_offset(layout.virtual_offset()),
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
            tree,
            layout
                .children()
                .next()
                .unwrap()
                .with_virtual_offset(layout.virtual_offset()),
            renderer,
            translation,
        )
    }

    #[cfg(feature = "a11y")]
    /// get the a11y nodes for the widget
    fn a11y_nodes(
        &self,
        layout: Layout<'_>,
        state: &Tree,
        cursor: mouse::Cursor,
    ) -> iced_accessibility::A11yTree {
        let c_layout = layout.children().next().unwrap();

        self.content.as_widget().a11y_nodes(
            c_layout.with_virtual_offset(layout.virtual_offset()),
            state,
            cursor,
        )
    }

    fn drag_destinations(
        &self,
        state: &Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        dnd_rectangles: &mut DndDestinationRectangles,
    ) {
        if let Some(l) = layout.children().next() {
            self.content.as_widget().drag_destinations(
                state,
                l.with_virtual_offset(layout.virtual_offset()),
                renderer,
                dnd_rectangles,
            );
        }
    }

    fn id(&self) -> Option<Id> {
        self.content.as_widget().id().clone()
    }

    fn set_id(&mut self, id: Id) {
        self.content.as_widget_mut().set_id(id);
    }
}

impl<'a, Message, Theme, Renderer> From<MenuWrapper<'a, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: 'a,
    Theme: 'a,
    Renderer: iced::advanced::Renderer + 'a,
{
    fn from(menu_wrapper: MenuWrapper<'a, Message, Theme, Renderer>) -> Self {
        Self::new(menu_wrapper)
    }
}
