use guido::prelude::*;

use crate::theme::ThemeColors;

#[component]
pub fn module_group(
    #[prop(
        default = "{ let t = expect_context::<ThemeColors>(); Color::rgba(t.background.r, t.background.g, t.background.b, 1.0) }"
    )]
    background: Color,
    #[prop(default = "16.0")] corner_radius: f32,
    #[prop(callback)] on_click: (),
    #[prop(children)] children: (),
) -> impl Widget {
    container()
        .height(fill())
        .background(background)
        .corner_radius(corner_radius.get())
        .layout(
            Flex::row()
                .spacing(8)
                .cross_alignment(CrossAlignment::Center),
        )
        .on_click_option(on_click.clone())
        .children_source(children)
}
