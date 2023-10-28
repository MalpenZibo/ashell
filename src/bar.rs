use crate::{
    app::AppCtx,
    modules::{app_launcher, system_info, title},
    reactive_gtk::{centerbox, container, Align, Node, NodeBuilder},
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
                .children(vec![app_launcher().into()])
                .into(),
        ))
        .center(Some(
            container()
                .vexpand(false)
                .valign(Align::Center)
                .children(vec![title().into()])
                .into(),
        ))
        .end(Some(
            container()
                .spacing(4)
                .vexpand(false)
                .valign(Align::Center)
                .children(vec![system_info().into()])
                .into(),
        ))
}
