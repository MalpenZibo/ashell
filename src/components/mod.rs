pub mod center_box;
pub mod expandable_panel;
pub mod icons;
pub mod module_group;
pub mod module_item;
pub mod quick_setting;
pub mod slider;
pub mod toggle_button;

pub use center_box::center_box;
pub use expandable_panel::expandable_panel;
pub use icons::{StaticIcon, icon};
pub use module_group::module_group;
pub use module_item::module_item;
pub use quick_setting::quick_setting;
pub use slider::slider;
pub use toggle_button::toggle_button;

/// Reusable hover menu button (label + optional click)
pub fn menu_button(
    label: &'static str,
    on_click: impl Fn() + 'static,
) -> impl guido::widgets::Widget {
    use guido::prelude::*;
    use crate::theme::ThemeColors;
    let theme = expect_context::<ThemeColors>();
    let hovered = create_signal(false);
    container()
        .width(fill())
        .padding([6.0, 8.0])
        .corner_radius(8.0)
        .on_click(move || on_click())
        .on_hover(move |h| hovered.set(h))
        .background(move || {
            if hovered.get() {
                Color::rgba(1.0, 1.0, 1.0, 0.1)
            } else {
                Color::TRANSPARENT
            }
        })
        .child(
            text(label)
                .color(theme.text)
                .font_size(14.0),
        )
}
