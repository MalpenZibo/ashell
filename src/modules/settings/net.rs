use gtk4::Widget;
use leptos::{create_memo, ReadSignal, SignalGet};

use crate::{
    gtk4_wrapper::{label, Component},
    utils::net::ActiveConnection,
};

pub fn net_indicator(active_connection: ReadSignal<Option<ActiveConnection>>) -> Widget {
    let format = create_memo(move |_| {
        let active_connection = active_connection.get();
        format!(
            "{}",
            active_connection.map_or("".to_string(), |c| c.to_icon().into()),
        )
    });
    let visible = create_memo(move |_| active_connection.get().is_some());

    label().text(format).visible(visible).into()
}
