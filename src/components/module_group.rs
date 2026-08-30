use crate::{
    config::{BorderAppearance, ModuleAppearance},
    theme::use_theme,
};
use iced::{
    Border, Element,
    widget::{blur_container, container},
};

/// Wraps content in a container styled from theme
pub fn module_group<'a, Msg: 'static>(
    content: Element<'a, Msg>,
    module_apperance: ModuleAppearance,
) -> Element<'a, Msg> {
    let (theme_space, theme_radius, module_opacity, module_border, blur) = use_theme(|theme| {
        (
            theme.space,
            theme.radius,
            theme.bar.opacity.module,
            theme.bar.module_border,
            theme.blur,
        )
    });

    let border = module_apperance.border.map_or_else(
        || Border {
            width: module_border.width,
            color: module_border.color.get_base(),
            radius: module_border.radius.resolve(theme_radius),
        },
        |BorderAppearance {
             width,
             radius,
             color,
         }| {
            Border {
                width,
                radius: radius.resolve(theme_radius),
                color: color.get_base(),
            }
        },
    );
    let opacity = module_apperance.opacity.unwrap_or(module_opacity);

    let padding = theme_space.resolve(module_apperance.padding);
    let style = move |iced_theme: &iced::Theme| {
        let background = module_apperance.background.map_or_else(
            || iced_theme.palette().background.scale_alpha(opacity),
            |b| b.get_base().scale_alpha(opacity),
        );

        container::Style {
            background: Some(background.into()),
            border,
            ..container::Style::default()
        }
    };

    if blur {
        blur_container(content).padding(padding).style(style).into()
    } else {
        container(content).padding(padding).style(style).into()
    }
}
