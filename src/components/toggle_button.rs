use guido::prelude::*;

use crate::theme::ThemeColors;

/// Pill-shaped on/off toggle button.
#[component]
pub struct ToggleButton {
    #[prop]
    active: bool,
    #[prop(callback)]
    on_toggle: (),
}

impl ToggleButton {
    fn render(&self) -> impl Widget + use<> {
        let theme = expect_context::<ThemeColors>();
        let active = self.active.clone();
        let active2 = self.active.clone();
        let active3 = self.active.clone();

        container()
            .width(36.0)
            .height(20.0)
            .corner_radius(10.0)
            .on_click_option(self.on_toggle.clone())
            .background(move || {
                if active.get() {
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
                        move || if active2.get() { 18.0 } else { 2.0 },
                        2.0,
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
}
