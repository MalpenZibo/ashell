pub mod bar_indicator;
pub mod buttons;
pub mod center_box;
pub mod expandable_panel;
pub mod icons;
pub mod module_group;
pub mod module_item;
pub mod quick_setting;
pub mod selectable_item;
pub mod slider;
pub mod toggle_button;

pub use bar_indicator::bar_indicator;
pub use buttons::{ButtonHierarchy, ButtonKind, ButtonSize, button};
pub use center_box::center_box;
pub use expandable_panel::expandable_panel;
pub use icons::{IconKind, StaticIcon, icon};
pub use module_group::module_group;
pub use module_item::module_item;
pub use quick_setting::quick_setting;
pub use selectable_item::selectable_item;
pub use slider::slider;
pub use toggle_button::toggle_button;

pub fn divider() -> impl guido::prelude::Widget {
    use guido::prelude::*;
    container()
        .width(fill())
        .height(1)
        .background(Color::rgba(1.0, 1.0, 1.0, 0.15))
}
