use iced::{
    Background, Border, Color, Length, Rectangle, Size,
    core::{
        Clipboard, Layout, Shell, Widget, event, layout, mouse, renderer, widget::Tree, window,
    },
};
use std::time::{Duration, Instant};

type Element<'a, Message, Theme, Renderer> = iced::core::Element<'a, Message, Theme, Renderer>;

#[derive(Debug, Clone, Copy)]
pub enum Countdown {
    Running {
        deadline: Instant,
        total: Duration,
    },
    Paused {
        remaining: Duration,
        total: Duration,
    },
}

impl Countdown {
    fn fraction(&self, now: Instant) -> f32 {
        let (remaining, total) = match *self {
            Self::Running { deadline, total } => (deadline.saturating_duration_since(now), total),
            Self::Paused { remaining, total } => (remaining, total),
        };

        if total.is_zero() {
            return 0.0;
        }

        (remaining.as_secs_f32() / total.as_secs_f32()).clamp(0.0, 1.0)
    }
}

/// A bar that shrinks as a countdown runs out. It requests its own redraws,
/// so no subscription has to tick the app.
pub struct CountdownBar {
    countdown: Countdown,
    height: f32,
    color: Color,
}

pub fn countdown_bar(countdown: Countdown, height: f32, color: Color) -> CountdownBar {
    CountdownBar {
        countdown,
        height,
        color,
    }
}

impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer> for CountdownBar
where
    Renderer: iced::core::Renderer,
{
    fn size(&self) -> Size<Length> {
        Size::new(Length::Fill, Length::Fixed(self.height))
    }

    fn layout(
        &mut self,
        _tree: &mut Tree,
        _renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        layout::atomic(limits, Length::Fill, Length::Fixed(self.height))
    }

    fn update(
        &mut self,
        _tree: &mut Tree,
        event: &event::Event,
        _layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _renderer: &Renderer,
        _clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        _viewport: &Rectangle,
    ) {
        let event::Event::Window(window::Event::RedrawRequested(now)) = event else {
            return;
        };

        if !matches!(self.countdown, Countdown::Running { .. })
            || self.countdown.fraction(*now) <= 0.0
        {
            return;
        }

        shell.request_redraw();
    }

    fn draw(
        &self,
        _tree: &Tree,
        renderer: &mut Renderer,
        _theme: &Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        if viewport.intersection(&bounds).is_none() {
            return;
        }

        let fraction = self.countdown.fraction(Instant::now());
        let width = bounds.width * fraction;
        if width <= 0.0 {
            return;
        }

        renderer.fill_quad(
            renderer::Quad {
                bounds: Rectangle { width, ..bounds },
                border: Border::default().rounded(bounds.height / 2.0),
                ..Default::default()
            },
            Background::Color(self.color),
        );
    }
}

impl<'a, Message, Theme, Renderer> From<CountdownBar> for Element<'a, Message, Theme, Renderer>
where
    Message: 'a,
    Theme: 'a,
    Renderer: iced::core::Renderer + 'a,
{
    fn from(bar: CountdownBar) -> Self {
        Self::new(bar)
    }
}

#[cfg(test)]
mod tests {
    use super::Countdown;
    use std::time::{Duration, Instant};

    const TOTAL: Duration = Duration::from_secs(4);

    #[test]
    fn running_fraction_follows_the_deadline() {
        let now = Instant::now();
        let countdown = Countdown::Running {
            deadline: now + Duration::from_secs(3),
            total: TOTAL,
        };

        assert_eq!(countdown.fraction(now), 0.75);
        assert_eq!(countdown.fraction(now + Duration::from_secs(5)), 0.0);
    }

    #[test]
    fn paused_fraction_ignores_the_clock() {
        let countdown = Countdown::Paused {
            remaining: Duration::from_secs(1),
            total: TOTAL,
        };

        assert_eq!(countdown.fraction(Instant::now()), 0.25);
        assert_eq!(
            countdown.fraction(Instant::now() + Duration::from_secs(60)),
            0.25
        );
    }

    #[test]
    fn fraction_is_clamped_and_safe_on_zero_total() {
        let over = Countdown::Paused {
            remaining: Duration::from_secs(8),
            total: TOTAL,
        };
        let zero = Countdown::Paused {
            remaining: Duration::ZERO,
            total: Duration::ZERO,
        };

        assert_eq!(over.fraction(Instant::now()), 1.0);
        assert_eq!(zero.fraction(Instant::now()), 0.0);
    }
}
