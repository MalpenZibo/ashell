//! Rotated text widget using canvas for vertical bar support.
//!
//! This widget renders text rotated 90° clockwise or counter-clockwise,
//! using iced's canvas API. The glyph-outline fallback path is triggered
//! when the transform is not a simple scale+translation.

use iced::{
    Color, Font, Length, Pixels, Rectangle, Theme, Vector,
    widget::{
        canvas::{self, Cache, Geometry, Program, Text},
        text::{Alignment, Shaping},
    },
};

/// Direction of rotation for the text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RotationDirection {
    /// 90° clockwise (text reads top-to-bottom when bar is on left)
    Clockwise,
    /// 90° counter-clockwise (text reads bottom-to-top when bar is on right)
    CounterClockwise,
}

/// A widget that renders text rotated 90° using canvas.
pub struct RotatedText {
    content: String,
    size: f32,
    color: Option<Color>,
    font: Font,
    direction: RotationDirection,
}

impl RotatedText {
    /// Creates a new rotated text widget.
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            size: 16.0,
            color: None,
            font: Font::DEFAULT,
            direction: RotationDirection::Clockwise,
        }
    }

    /// Sets the font size.
    pub fn size(mut self, size: impl Into<Pixels>) -> Self {
        self.size = size.into().0;
        self
    }

    /// Sets the text color.
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    /// Sets the font.
    pub fn font(mut self, font: Font) -> Self {
        self.font = font;
        self
    }

    /// Sets the rotation direction.
    pub fn direction(mut self, direction: RotationDirection) -> Self {
        self.direction = direction;
        self
    }
}

struct RotatedTextProgram {
    content: String,
    size: f32,
    color: Color,
    font: Font,
    direction: RotationDirection,
    cache: Cache,
}

impl<Message> Program<Message> for RotatedTextProgram {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &iced::Renderer,
        theme: &Theme,
        bounds: Rectangle,
        _cursor: iced::mouse::Cursor,
    ) -> Vec<Geometry> {
        let geometry = self.cache.draw(renderer, bounds.size(), |frame| {
            let color = if self.color == Color::TRANSPARENT {
                theme.palette().text
            } else {
                self.color
            };

            // Use with_save to automatically restore transform after drawing
            frame.with_save(|frame| {
                // Translate to center of frame
                let center = frame.center();
                frame.translate(Vector::new(center.x, center.y));

                // Rotate around the center
                match self.direction {
                    RotationDirection::Clockwise => {
                        frame.rotate(std::f32::consts::FRAC_PI_2);
                    }
                    RotationDirection::CounterClockwise => {
                        frame.rotate(-std::f32::consts::FRAC_PI_2);
                    }
                }

                // Draw text centered at origin (which is now the center of the frame)
                let text = Text {
                    content: self.content.clone(),
                    position: iced::Point::ORIGIN,
                    color,
                    size: Pixels(self.size),
                    font: self.font,
                    align_x: Alignment::Center,
                    align_y: iced::alignment::Vertical::Center,
                    shaping: Shaping::Advanced,
                    line_height: iced::widget::text::LineHeight::default(),
                    max_width: f32::INFINITY,
                };

                frame.fill_text(text);
            });
        });

        vec![geometry]
    }
}

impl<Message: 'static> From<RotatedText> for iced::Element<'_, Message> {
    fn from(widget: RotatedText) -> Self {
        let color = widget.color.unwrap_or(Color::TRANSPARENT);
        let program = RotatedTextProgram {
            content: widget.content,
            size: widget.size,
            color,
            font: widget.font,
            direction: widget.direction,
            cache: Cache::new(),
        };

        canvas::Canvas::new(program)
            .width(Length::Shrink)
            .height(Length::Shrink)
            .into()
    }
}
