use crate::{
    reactive_gtk::{label, Dynamic, Node, NodeBuilder},
    utils::net::ActiveConnection,
};
use futures_signals::signal::Mutable;

pub fn net_indicator(active_connection: Mutable<Option<ActiveConnection>>) -> impl Into<Node> {
    let format = active_connection.signal_ref(|active_connection| {
        active_connection.as_ref().map_or("", |c| c.to_icon()).to_string()
    });
    let visible = active_connection.signal_ref(|active_connection| active_connection.is_some());

    label().text(Dynamic(format)).visible(Dynamic(visible))
}
