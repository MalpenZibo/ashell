use crate::{
    nodes,
    reactive_gtk::{container, label, Dynamic, Node, NodeBuilder},
    utils::net::{ActiveConnection, Vpn},
};
use futures_signals::signal::Mutable;

pub fn net_indicator(
    active_connection: Mutable<Option<ActiveConnection>>,
    vpn_list: Mutable<Vec<Vpn>>,
) -> impl Into<Node> {
    container().spacing(4).children(nodes!(
        connection_indicator(active_connection),
        vpn_indicator(vpn_list)
    ))
}

pub fn vpn_indicator(vpn_list: Mutable<Vec<Vpn>>) -> impl Into<Node> {
    let visible = vpn_list.signal_ref(|vpn_list| !vpn_list.is_empty());

    label()
        .class(vec!["vpn"])
        .text("󰖂")
        .visible(Dynamic(visible))
}

pub fn connection_indicator(
    active_connection: Mutable<Option<ActiveConnection>>,
) -> impl Into<Node> {
    let format = active_connection.signal_ref(|active_connection| {
        active_connection
            .as_ref()
            .map_or("", |c| c.to_icon())
            .to_string()
    });
    let visible = active_connection.signal_ref(|active_connection| active_connection.is_some());

    label()
        .class(Dynamic(active_connection.signal_ref(|active_connection| {
            [
                active_connection
                    .as_ref()
                    .map_or(vec![], |c| c.get_classes()),
                vec!["connection"],
            ]
            .concat()
        })))
        .text::<String>(Dynamic(format))
        .visible(Dynamic(visible))
}
