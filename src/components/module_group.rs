use crate::{config::{AppearanceStyle, ModuleName}, theme::use_theme};
use iced::{Border, Color, Element, widget::container};

/// Wraps content with the appropriate bar style container.
///
/// - `Solid | Gradient` → pass through as-is
/// - `Islands` → wrap in a container with background color + rounded border
///
/// Per-module appearance overrides are applied when available.
pub fn module_group<'a, Msg: 'static>(content: Element<'a, Msg>, module_name: Option<&ModuleName>) -> Element<'a, Msg> {
    let (bar_style, opacity, radius, text_color) =
        use_theme(|theme| {
            let base_opacity = match module_name {
                Some(name) => theme.module_opacity(name),
                None => theme.opacity,
            };
            let base_radius = match module_name {
                Some(name) => theme.module_border_radius(name).unwrap_or(theme.radius.lg),
                None => theme.radius.lg,
            };
            let text_color = module_name
                .and_then(|name| theme.module_text_color(name))
                .map(|c| c.get_base());
            (theme.bar_style, base_opacity, base_radius, text_color)
        });

    match bar_style {
        AppearanceStyle::Solid | AppearanceStyle::Gradient => {
            // Even in Solid/Gradient mode, apply per-module text color if set
            match text_color {
                Some(tc) => container(content)
                    .style(move |_iced_theme: &iced::Theme| container::Style {
                        text_color: Some(tc),
                        ..container::Style::default()
                    })
                    .into(),
                None => content,
            }
        }
        AppearanceStyle::Islands => {
            // Check if this module has a custom background color
            let custom_bg = module_name.and_then(|name| {
                use_theme(|theme| {
                    theme.module_background_color(name).cloned()
                })
            });

            container(content)
                .style(move |iced_theme: &iced::Theme| container::Style {
                    background: Some(
                        if let Some(ref custom_bg) = custom_bg {
                            custom_bg.get_base().scale_alpha(opacity)
                        } else {
                            iced_theme.palette().background.scale_alpha(opacity)
                        }
                        .into()
                    ),
                    text_color,
                    border: Border {
                        width: 0.0,
                        radius: radius.into(),
                        color: Color::TRANSPARENT,
                    },
                    ..container::Style::default()
                })
                .into()
        }
    }
}
