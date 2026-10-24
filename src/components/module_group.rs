use crate::{
    config::{BarSurface, Surface},
    theme::{IslandStyle, Paint, use_theme},
};
use iced::{
    Border, Element,
    widget::{blur_container, container},
};

/// Wraps content in its island, styled by `island_style`.
///
/// - `Solid` → pass through as-is (the bar itself carries the background),
///   unless the island sets its own border or background
/// - `Transparent` → wrap in a container with background color + rounded border,
///   using `blur_container` when compositor blur is enabled
pub fn module_group<'a, Msg: 'static>(
    content: Element<'a, Msg>,
    island_style: IslandStyle,
) -> Element<'a, Msg> {
    let (bar_surface, blur) =
        use_theme(|theme| (theme.bar_surface, theme.surface(Surface::Bar).blur));

    let style = move |iced_theme: &iced::Theme| container::Style {
        background: match island_style.background {
            Some(color) => Some(Paint::surface(iced_theme, color.color(iced_theme)).into()),
            // A solid bar already paints the default background.
            None if bar_surface == BarSurface::Solid => None,
            None => Some(iced_theme.palette().background.into()),
        },
        border: Border {
            radius: island_style.radius,
            ..island_style
                .border
                .map_or(Border::default(), |border| Border {
                    width: border.width,
                    color: border.color.color(iced_theme),
                    ..Border::default()
                })
        },
        ..container::Style::default()
    };

    match bar_surface {
        BarSurface::Solid if island_style.border.is_none() && island_style.background.is_none() => {
            content
        }
        // The bar itself carries the blur.
        BarSurface::Solid => container(content)
            .padding(island_style.padding)
            .style(style)
            .into(),
        BarSurface::Transparent => {
            if blur {
                blur_container(content)
                    .padding(island_style.padding)
                    .style(style)
                    .into()
            } else {
                container(content)
                    .padding(island_style.padding)
                    .style(style)
                    .into()
            }
        }
    }
}
