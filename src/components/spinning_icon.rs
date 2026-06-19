use iced::{
    Length, Radians, Rectangle, Size, Vector,
    core::{
        Clipboard, Layout, Shell, Widget, event, layout, mouse, overlay, renderer,
        widget::{Operation, Tree},
    },
    widget::canvas::{self, Frame, Path},
};
use std::{f32::consts::PI, time::Instant};

type Element<'a, Message, Theme, Renderer> = iced::core::Element<'a, Message, Theme, Renderer>;

const SPIN_SPEED: f32 = PI * 2.0;

const DOTS: &[(f32, f32, f32)] = &[
    (12.0, 1.0, 0.9),
    (19.2, 4.8, 1.1),
    (22.5, 12.0, 1.3),
    (19.2, 19.2, 1.5),
    (12.0, 22.5, 1.7),
    (4.8, 19.2, 1.9),
    (1.5, 12.0, 2.1),
    (4.8, 4.8, 2.3),
];

struct SpinnerState {
    start: Instant,
}

impl Default for SpinnerState {
    fn default() -> Self {
        Self {
            start: Instant::now(),
        }
    }
}

struct SpinnerProgram {
    size: f32,
}

impl<Message> canvas::Program<Message> for SpinnerProgram {
    type State = SpinnerState;

    fn draw(
        &self,
        state: &Self::State,
        renderer: &iced::Renderer,
        theme: &iced::Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let color = theme.palette().text;
        let scale = self.size / 24.0;
        let center_x = bounds.width / 2.0;
        let center_y = bounds.height / 2.0;

        let mut frame = Frame::new(renderer, bounds.size());

        let elapsed = state.start.elapsed().as_secs_f32();
        let angle = elapsed * SPIN_SPEED;

        frame.with_save(|frame| {
            frame.translate(Vector::new(center_x, center_y));
            frame.rotate(Radians(angle));
            frame.translate(Vector::new(-center_x, -center_y));

            for &(cx, cy, r) in DOTS {
                frame.fill(
                    &Path::circle(iced::Point::new(cx * scale, cy * scale), r * scale),
                    color,
                );
            }
        });

        vec![frame.into_geometry()]
    }
}

/// Drives continuous redraws so the inner canvas keeps spinning.
pub struct SpinningIconWidget<'a, Message, Theme = iced::Theme, Renderer = iced::Renderer>
where
    Renderer: iced::core::Renderer,
{
    content: Element<'a, Message, Theme, Renderer>,
}

impl<'a, Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for SpinningIconWidget<'a, Message, Theme, Renderer>
where
    Theme: 'a,
    Renderer: iced::core::Renderer + 'a,
{
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
        self.content
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits)
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
            layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );

        if let event::Event::Window(iced::core::window::Event::RedrawRequested(_)) = event {
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
        self.content.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            layout,
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
        self.content
            .as_widget_mut()
            .operate(&mut tree.children[0], layout, renderer, operation);
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
            &tree.children[0],
            layout,
            cursor,
            viewport,
            renderer,
        )
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
            layout,
            renderer,
            viewport,
            translation,
        )
    }
}

impl<'a, Message, Theme, Renderer> From<SpinningIconWidget<'a, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: 'a,
    Theme: 'a,
    Renderer: iced::core::Renderer + 'a,
{
    fn from(widget: SpinningIconWidget<'a, Message, Theme, Renderer>) -> Self {
        Self::new(widget)
    }
}

pub fn spinning_icon<Message: 'static>(
    size: f32,
    animated: bool,
) -> iced::Element<'static, Message> {
    if !animated {
        return iced::widget::container(
            crate::components::icons::icon(crate::components::icons::StaticIcon::Refresh)
                .size(size)
                .line_height(1.0),
        )
        .width(Length::Fixed(size))
        .height(Length::Fixed(size))
        .align_x(iced::alignment::Horizontal::Center)
        .align_y(iced::alignment::Vertical::Center)
        .into();
    }

    let canvas_widget = iced::widget::canvas(SpinnerProgram { size })
        .width(Length::Fixed(size))
        .height(Length::Fixed(size));

    SpinningIconWidget {
        content: canvas_widget.into(),
    }
    .into()
}
