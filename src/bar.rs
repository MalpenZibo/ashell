use crate::{
    app::AppCtx,
    modules::{title, app_launcher},
    reactive_gtk::{centerbox, Node, NodeBuilder},
};

pub fn bar(app: AppCtx) -> impl Into<Node> {
    centerbox()
        .class(vec!("bar"))
        .start(Some(app_launcher().into()))
        .center(Some(title().into()))
}
