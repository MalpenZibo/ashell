use guido::prelude::*;

use crate::theme::ThemeColors;

/// Pill-shaped on/off toggle button.
pub fn toggle_button(
    active: impl Fn() -> bool + 'static + Clone,
    on_toggle: impl Fn() + 'static,
) -> impl Widget {
    let theme = expect_context::<ThemeColors>();
    let hovered = create_signal(false);
    let active2 = active.clone();
    let active3 = active.clone();
    let active4 = active.clone();

    container()
        .width(36.0)
        .height(20.0)
        .corner_radius(10.0)
        .on_hover(move |h| hovered.set(h))
        .on_click(move || on_toggle())
        .background(move || {
            if active() {
                theme.primary
            } else {
                Color::rgba(1.0, 1.0, 1.0, 0.2)
            }
        })
        // Knob
        .child(
            container()
                .width(16.0)
                .height(16.0)
                .corner_radius(8.0)
                .translate(
                    move || if active2() { 18.0 } else { 2.0 },
                    2.0,
                )
                .background(move || {
                    if active3() {
                        theme.background
                    } else {
                        theme.text
                    }
                }),
        )
}
