use crate::{reactive_gtk::{label, Align, Node, NodeBuilder}, utils};

pub fn app_launcher() -> impl Into<Node> {
    label()
        .class(vec!["bar-item", "interactive", "app-launcher"])
        .text("󱗼".to_string())
        .vexpand(false)
        .hexpand(false)
        .valign(Align::Center)
        .on_click(Box::new(|| {
            utils::launcher::launch_rofi();
        }))
}
