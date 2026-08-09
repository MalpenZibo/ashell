use crate::{config::ModuleAppearance, theme::use_theme};
use iced::{Border, Element, widget::container};

/// Wraps content in a container styled from theme
pub fn module_group<'a, Msg: 'static>(
    content: Element<'a, Msg>,
    module_apperance: &ModuleAppearance,
) -> Element<'a, Msg> {
    let (theme_space, theme_radius, module_opacity, module_border, _module_padding) =
        use_theme(|theme| {
            (
                theme.space,
                theme.radius,
                theme.bar.opacity.module,
                theme.bar.module_border,
                theme.space.xxs,
            )
        });

    let radius = module_border.radius.resolve(theme_radius);
    let padding = theme_space.resolve(module_apperance.padding);

    container(content)
        .padding(padding)
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
        .into()
}
