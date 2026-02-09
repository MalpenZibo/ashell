use guido::prelude::*;

use crate::theme;

#[component]
pub struct ModuleGroup {
    #[prop(default = "Color::rgba(theme::BASE.r, theme::BASE.g, theme::BASE.b, 0.85)")]
    background: Color,
    #[prop(default = "16.0")]
    corner_radius: f32,
    #[prop(default = "12.0")]
    padding_x: f32,
    #[prop(callback)]
    on_click: (),
    #[prop(children)]
    children: (),
}

impl ModuleGroup {
    fn render(&self) -> impl Widget + use<> {
        container()
            .height(fill())
            .padding_xy(self.padding_x.get(), 0.0)
            .background(self.background.clone())
            .corner_radius(self.corner_radius.get())
            .layout(
                Flex::row()
                    .spacing(12.0)
                    .cross_axis_alignment(CrossAxisAlignment::Center),
            )
            .on_click_option(self.on_click.clone())
            .children_source(self.take_children())
    }
}
