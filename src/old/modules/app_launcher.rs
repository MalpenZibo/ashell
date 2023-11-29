use crate::{
    nodes,
    reactive_gtk::{container, Node, NodeBuilder},
    utils, components::icons::{icon, Icons},
};

pub fn app_launcher() -> impl Into<Node> {
    container()
        .class(vec!["bar-item", "interactive", "app-launcher"])
        .children(nodes!(icon(Icons::Launcher)))
        .vexpand(true)
        .hexpand(false)
        .on_click(Box::new(|| {
            utils::launcher::launch_rofi();
        }))
}
