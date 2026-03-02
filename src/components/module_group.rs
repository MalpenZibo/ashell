use guido::prelude::*;

use crate::theme::ThemeColors;

#[component]
pub struct ModuleGroup {
    #[prop(
        default = "{ let t = expect_context::<ThemeColors>(); Color::rgba(t.background.r, t.background.g, t.background.b, 1.0) }"
    )]
    background: Color,
    #[prop(default = "16.0")]
    corner_radius: f32,
    #[prop(callback)]
    on_click: (),
    #[prop(children)]
    children: (),
}

impl ModuleGroup {
    fn render(&self) -> impl Widget + use<> {
        container()
            .height(fill())
            .background(self.background.clone())
            .corner_radius(self.corner_radius.get())
            .layout(
                Flex::row()
                    .spacing(8.0)
                    .cross_alignment(CrossAlignment::Center),
            )
            .on_click_option(self.on_click.clone())
            .children_source(self.take_children())
    }
}
