use guido::prelude::*;

use crate::theme::ThemeColors;

/// Pill-shaped on/off toggle button.
#[component]
pub fn toggle_button(
    active: bool,
    #[prop(callback)]
    on_toggle: (),
) -> impl Widget {
    let theme = expect_context::<ThemeColors>();
    let active1 = active.clone();
    let active2 = active.clone();
    let active3 = active.clone();

    container()
        .width(36)
        .height(20)
        .corner_radius(10)
        .on_click_option(on_toggle.clone())
        .background(move || {
            if active1.get() {
                theme.primary
            } else {
                Color::rgba(1.0, 1.0, 1.0, 0.2)
            }
        })
        // Knob
        .child(
            container()
                .width(16)
                .height(16)
                .corner_radius(8)
                .translate(
                    move || if active2.get() { 18 } else { 2 },
                    2,
                )
                .background(move || {
                    if active3.get() {
                        theme.background
                    } else {
                        theme.text
                    }
                }),
        )
}
