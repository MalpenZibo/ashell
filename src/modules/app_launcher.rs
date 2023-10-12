use gtk4::Widget;

use crate::{
    gtk4_wrapper::{label, Align, Component},
    utils,
};

pub fn app_launcher() -> Widget {
    label()
        .class(vec!["header-button"])
        .text("󱗼")
        .vexpand(false)
        .valign(Align::Center)
        .visible(true)
        .on_click(Box::new(|| {
            utils::launcher::launch_rofi();
        }))
        .into()
}
