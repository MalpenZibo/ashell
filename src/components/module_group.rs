use guido::prelude::*;

use crate::config::{BarSurface, Config};
use crate::theme::ThemeColors;

#[component]
pub fn module_group(
    #[prop(
        default = "{ let t = expect_context::<ThemeColors>(); let (op, solid) = with_context::<Config, _>(|c| (c.appearance.bar.opacity, c.appearance.bar.surface == BarSurface::Solid)).unwrap_or((1.0, false)); if solid { Color::TRANSPARENT } else { Color::rgba(t.background.r, t.background.g, t.background.b, op) } }"
    )]
    background: Color,
    #[prop(default = "16.0")] corner_radius: f32,
    #[prop(callback)] on_click: (),
    #[prop(children)] children: (),
) -> impl Widget {
    // Solid bar surface: the bar carries the background, islands pass through
    let (opacity, solid, blur) = with_context::<Config, _>(|c| {
        (
            c.appearance.bar.opacity,
            c.appearance.bar.surface == BarSurface::Solid,
            c.appearance.blur,
        )
    })
    .unwrap_or((1.0, false, crate::config::BlurMode::Never));

    let mut c = container()
        .height(fill())
        .background(background)
        .corner_radius(if solid { 0.0 } else { corner_radius.get() })
        // No inner spacing: items carry their own horizontal padding
        // (upstream Row default)
        .layout(Flex::row().cross_alignment(CrossAlignment::Center))
        .on_click_option(on_click);
    if !solid && blur.enabled(opacity) {
        c = c.background_blur();
    }
    c.children_source(children)
}
