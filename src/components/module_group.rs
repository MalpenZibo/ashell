use crate::{config::BarSurface, theme::use_theme};
use iced::{
    Border, Color, Element,
    widget::{blur_container, container},
};

/// Wraps content with the appropriate bar surface container.
///
/// - `Solid` → pass through as-is (the bar itself carries the background)
/// - `Transparent` → wrap in a container with background color + rounded border,
///   using `blur_container` when compositor blur is enabled
pub fn module_group<'a, Msg: 'static>(content: Element<'a, Msg>) -> Element<'a, Msg> {
    let (bar_surface, opacity, radius, blur) =
        use_theme(|theme| (theme.bar_surface, theme.opacity, theme.radius, theme.blur));

    match bar_surface {
        BarSurface::Solid => content,
        BarSurface::Transparent => {
            let style = move |iced_theme: &iced::Theme| container::Style {
                background: Some(iced_theme.palette().background.scale_alpha(opacity).into()),
                border: Border {
                    width: 0.0,
                    radius: radius.lg.into(),
                    color: Color::TRANSPARENT,
                },
                ..container::Style::default()
            };
            if blur {
                blur_container(content).style(style).into()
            } else {
                container(content).style(style).into()
            }
        }
    }
}
