use crate::{
    config::{BarSurface, ModuleAppearance},
    theme::use_theme,
};
use iced::{Border, Color, Element, widget::container};

/// Wraps content with the appropriate bar surface container.
///
/// - `Solid` → pass through as-is (the bar itself carries the background)
/// - `Transparent` → wrap in a container with background color + rounded border
/// - `Panel` → wrap in a container with customisation from config
pub fn module_group<'a, Msg: 'static>(
    content: Element<'a, Msg>,
    module_apperance: Option<&ModuleAppearance>,
) -> Element<'a, Msg> {
    let (bar_surface, theme_space, theme_radius, module_opacity, module_border, _module_padding) =
        use_theme(|theme| {
            (
                theme.bar.surface,
                theme.space,
                theme.radius,
                theme.bar.opacity.module,
                theme.bar.module_border,
                theme.space.xxs,
            )
        });

    let radius = module_border.radius.resolve(theme_radius);

    match bar_surface {
        BarSurface::Solid => content,
        BarSurface::Transparent => container(content)
            .padding(
                module_apperance.map_or(0., |appearance| theme_space.resolve(appearance.padding)),
            )
            .style(move |iced_theme: &iced::Theme| container::Style {
                background: Some(
                    iced_theme
                        .palette()
                        .background
                        .scale_alpha(module_opacity)
                        .into(),
                ),
                border: Border {
                    width: 0.0,
                    radius,
                    color: Color::TRANSPARENT,
                },
                ..container::Style::default()
            })
            .into(),
        BarSurface::Panel => container(content)
            .padding(
                module_apperance.map_or(0., |appearance| theme_space.resolve(appearance.padding)),
            )
            .style(move |iced_theme: &iced::Theme| container::Style {
                background: Some(
                    iced_theme
                        .palette()
                        .background
                        .scale_alpha(module_opacity)
                        .into(),
                ),
                border: Border {
                    width: module_border.width,
                    radius,
                    color: module_border.color.get_base(),
                },
                ..container::Style::default()
            })
            .into(),
    }
}
