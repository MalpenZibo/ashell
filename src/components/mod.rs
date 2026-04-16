pub mod button;
mod centerbox;
mod format_indicator;
pub mod icons;
pub mod menu;
mod menu_wrapper;
mod module_group;
mod module_item;
pub mod password_dialog;
mod position_button;
mod quick_setting_button;
mod slider_control;
mod sub_menu_wrapper;

pub use button::*;
pub use centerbox::*;
pub use format_indicator::*;
pub use menu_wrapper::*;
pub use module_group::*;
pub use module_item::*;
pub use position_button::*;
pub use quick_setting_button::*;
pub use slider_control::*;
pub use sub_menu_wrapper::*;

use iced::{Element, widget::rule};

pub fn divider<'a, Msg: 'static>() -> Element<'a, Msg> {
    rule::horizontal(1).into()
}
