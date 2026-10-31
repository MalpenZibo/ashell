use iced::{
    Length, Radians, Rectangle, Size, Vector,
    core::{
        Clipboard, Layout, Shell, Widget, event, layout, mouse, overlay, renderer,
        widget::{Operation, Tree},
    },
    widget::canvas::{self, Frame, Path},
};
use std::{
    cell::Cell,
    f32::consts::PI,
    time::{Duration, Instant},
};

type Element<'a, Message, Theme, Renderer> = iced::core::Element<'a, Message, Theme, Renderer>;

const SPIN_SPEED: f32 = PI * 2.0;

/// Minimum spacing between redraws of the spinner.
///
/// The canvas is a couple of dozen pixels wide and one revolution takes a
/// second, so a full-frame refresh adds nothing the eye can see while keeping
/// the compositor busy on a surface that is otherwise idle. Capping here makes
/// the spinner cost proportional to what it actually renders.
const FRAME_INTERVAL: Duration = Duration::from_millis(1000 / 30);

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

/// Rotation origin, set by the canvas program.
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

/// Redraw throttle, set by the widget wrapping the canvas.
///
/// Frames are counted in [`Widget::draw`], which runs on every drawn frame,
/// rather than waiting for a `RedrawRequested` event: a layer surface that
/// stays idle is only updated when it has events of its own or the app
/// produced messages, so an event-driven throttle stops requesting frames
/// after the first one and the spinner freezes. Counting drawn frames keeps
/// the chain alive and still spaces redraws by at least [`FRAME_INTERVAL`].
struct Throttle {
    last_frame: Instant,
    /// Frames drawn since the last redraw request. A [`Cell`] because
    /// [`Widget::draw`] only gets a shared [`Tree`].
    drawn_frames: Cell<u32>,
}

impl Throttle {
    fn new(now: Instant) -> Self {
        Self {
            last_frame: now,
            drawn_frames: Cell::new(0),
        }
    }
}

/// Records that a frame was drawn.
fn drawn(throttle: &Throttle) {
    throttle.drawn_frames.set(throttle.drawn_frames.get() + 1);
}

/// Returns `true` at most once per [`FRAME_INTERVAL`], and only after a frame
/// has been drawn since the previous request.
fn redraw_due(throttle: &mut Throttle, now: Instant) -> bool {
    let elapsed = now.saturating_duration_since(throttle.last_frame);
    if throttle.drawn_frames.get() > 0 && elapsed >= FRAME_INTERVAL {
        throttle.drawn_frames.set(0);
        throttle.last_frame = now;
        true
    } else {
        false
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

/// Requests a redraw every [`FRAME_INTERVAL`] so the inner canvas keeps
/// spinning without keeping the compositor busy on every frame.
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
    fn tag(&self) -> iced::core::widget::tree::Tag {
        iced::core::widget::tree::Tag::of::<Throttle>()
    }

    fn state(&self) -> iced::core::widget::tree::State {
        iced::core::widget::tree::State::new(Throttle::new(Instant::now()))
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

        if redraw_due(tree.state.downcast_mut::<Throttle>(), Instant::now()) {
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

        drawn(tree.state.downcast_ref::<Throttle>());
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

#[cfg(test)]
mod tests {
    use super::{FRAME_INTERVAL, Throttle, drawn, redraw_due};
    use std::time::Instant;

    #[test]
    fn requests_once_per_frame_interval_while_frames_keep_being_drawn() {
        let start = Instant::now();
        let mut throttle = Throttle::new(start);

        // Nothing has been drawn yet: an event must not start a redraw loop.
        assert!(!redraw_due(&mut throttle, start + FRAME_INTERVAL));
        drawn(&throttle);

        // Too early: wait, but keep the pending frame counted.
        assert!(!redraw_due(&mut throttle, start + FRAME_INTERVAL / 2));
        assert!(redraw_due(&mut throttle, start + FRAME_INTERVAL));

        // The request is consumed until the next frame is drawn.
        assert!(!redraw_due(
            &mut throttle,
            start + FRAME_INTERVAL + FRAME_INTERVAL
        ));
        drawn(&throttle);
        assert!(redraw_due(
            &mut throttle,
            start + FRAME_INTERVAL + FRAME_INTERVAL
        ));
    }
}
