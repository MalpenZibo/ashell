use guido::prelude::*;

use crate::theme::ThemeColors;

/// Pill-shaped on/off toggle button.
#[component]
pub fn toggle_button(active: bool, #[prop(callback)] on_toggle: ()) -> impl Widget {
    let theme = expect_context::<ThemeColors>();

    let on_toggle = on_toggle.clone();
    let hovered = create_signal(false);

    container()
        .width(42)
        .height(24)
        .corner_radius(24)
        .border(
            move || if active.get() { 0 } else { 1 },
            move || {
                if hovered.get() {
                    theme.background.lighter(0.2)
                } else {
                    theme.background.lighter(0.1)
                }
            },
        )
        .on_click_option(on_toggle)
        .background(move || {
            let base = if active.get() {
                theme.primary
            } else {
                Color::TRANSPARENT
            };

            if hovered.get() {
                base.lighter(0.2)
            } else {
                base
            }
        })
        .on_hover(move |inside| hovered.set(inside))
        .child(
            container()
                .width(16)
                .height(16)
                .corner_radius(16)
                .translate(move || if active.get() { 20 } else { 4 }, 4)
                .background(move || {
                    let base = if active.get() {
                        theme.background
                    } else {
                        theme.primary
                    };

                    if hovered.get() {
                        base.lighter(0.2)
                    } else {
                        base
                    }
                })
                .animate_transform(Transition::spring(SpringConfig::BOUNCY)),
        )
}
