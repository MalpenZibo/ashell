use crate::{
    app::AppCtx,
    modules::{app_launcher, system_info, title, workspaces, settings},
    reactive_gtk::{centerbox, container, Align, Node, NodeBuilder}, nodes,
};

pub fn bar(app: AppCtx) -> impl Into<Node> {
    centerbox()
        .class(vec!["bar"])
        .valign(Align::Center)
        .vexpand(false)
        .start(Some(
            container()
                .spacing(4)
                .vexpand(false)
                .valign(Align::Center)
                .children(nodes![app_launcher(), workspaces()]),
        ))
        .center(Some(
            container()
                .vexpand(false)
                .valign(Align::Center)
                .children(nodes![title()]),
        ))
        .end(Some(
            container()
                .spacing(4)
                .vexpand(false)
                .valign(Align::Center)
                .children(nodes![system_info(), settings()]),
        ))
}
