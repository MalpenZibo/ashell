use crate::{config::BarSurface, theme::use_theme};
use iced::{Border, Color, Element, widget::container};

/// Wraps content with the appropriate bar surface container.
///
/// - `Solid` → pass through as-is (the bar itself carries the background)
/// - `Transparent` → wrap in a container with background color + rounded border
pub fn module_group<'a, Msg: 'static>(content: Element<'a, Msg>) -> Element<'a, Msg> {
    let (bar_surface, opacity, radius) =
        use_theme(|theme| (theme.bar_surface, theme.opacity, theme.radius));

    match bar_surface {
        BarSurface::Solid => content,
        BarSurface::Transparent => container(content)
            .style(move |iced_theme: &iced::Theme| container::Style {
                background: Some(iced_theme.palette().background.scale_alpha(opacity).into()),
                border: Border {
                    width: 0.0,
                    radius: radius.lg.into(),
                    color: Color::TRANSPARENT,
                },
                ..container::Style::default()
            })
            .into(),
    }
}
