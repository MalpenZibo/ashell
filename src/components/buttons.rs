use super::icons::{IconKind, icon};
use crate::theme::ThemeColors;
use guido::prelude::*;

#[derive(Copy, Clone)]
pub enum ButtonKind {
    Solid,
    Transparent,
}

/// Reusable hover menu button (label + optional click)
#[component]
pub struct Button {
    #[prop(default = "None")]
    icon: Option<IconKind>,
    #[prop]
    label: String,
    #[prop(default = "ButtonKind::Transparent")]
    kind: ButtonKind,
    #[prop(callback)]
    on_click: (),
}

impl Button {
    fn render(&self) -> impl Widget + use<> {
        let theme = expect_context::<ThemeColors>();

        container()
            .height(32.)
            .width(at_least(32.))
            .padding(4.)
            .corner_radius(32.0)
            .on_click_option(self.on_click.clone())
            .hover_state(|c| c.background(Color::rgba(1.0, 1.0, 1.0, 0.1)))
            .background(Color::TRANSPARENT)
            .layout(
                Flex::row()
                    .main_alignment(MainAlignment::Center)
                    .cross_alignment(CrossAlignment::Center),
            )
            .maybe_child(
                self.icon
                    .get()
                    .map(|ik| icon().ic(ik).mono(true).color(theme.text)),
            )
            .child(text(self.label.clone()).color(theme.text).font_size(14.0))
    }
}
