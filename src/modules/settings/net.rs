use crate::{
    components::icons::{icon_with_class, Icons},
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

    icon_with_class(Icons::Vpn, vec!["vpn"])
        .into()
        .visible(Dynamic(visible))
}

pub fn connection_indicator(
    active_connection: Mutable<Option<ActiveConnection>>,
) -> impl Into<Node> {
    let icon_type = active_connection.signal_ref(|active_connection| {
        active_connection
            .as_ref()
            .map(|c| c.to_icon())
            .unwrap_or_default()
    });
    let visible = active_connection.signal_ref(|active_connection| active_connection.is_some());

    icon_with_class(
        Dynamic(icon_type),
        Dynamic(active_connection.signal_ref(|active_connection| {
            [
                active_connection
                    .as_ref()
                    .map_or(vec![], |c| c.get_classes()),
                vec!["connection"],
            ]
            .concat()
        })),
    )
    .into()
    .visible(Dynamic(visible))
}
