use super::icons::IconKind;
use crate::theme::ThemeColors;
use guido::prelude::*;

#[derive(Copy, Clone)]
pub enum ButtonKind {
    Solid,
    Transparent,
}

/// Reusable hover menu button (label + optional click)
#[component]
pub fn button(
    #[prop(default = "None")]
    icon: Option<IconKind>,
    #[prop(slot)]
    content: (),
    #[prop(default = "ButtonKind::Transparent")]
    kind: ButtonKind,
    #[prop(callback)]
    on_click: (),
) -> impl Widget {
    let theme = expect_context::<ThemeColors>();

    container()
        .height(32)
        .width(at_least(32))
        .padding([0, 8])
        .corner_radius(32)
        .on_click_option(on_click.clone())
        .hover_state(|c| c.background(Color::rgba(1.0, 1.0, 1.0, 0.1)))
        .background(Color::TRANSPARENT)
        .layout(
            Flex::row()
                .main_alignment(MainAlignment::Center)
                .cross_alignment(CrossAlignment::Center),
        )
        .maybe_child(
            icon
                .get()
                .map(|ik| super::icons::icon().ic(ik).mono(true).color(theme.text)),
        )
        .maybe_child(content)
}
