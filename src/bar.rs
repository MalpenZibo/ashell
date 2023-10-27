use crate::{
    app::AppCtx,
    modules::title,
    reactive_gtk::{centerbox, Node},
};

pub fn bar(app: AppCtx) -> impl Into<Node> {
    centerbox().center(Some(title().into()))
}
