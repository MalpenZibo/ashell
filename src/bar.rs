use crate::{app::AppCtx, reactive_gtk::{centerbox, Node}};

pub fn bar(app: AppCtx) -> impl Into<Node> {
    centerbox()
}
