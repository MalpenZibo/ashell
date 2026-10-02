use futures::{
    Stream, StreamExt,
    future::ready,
    stream::{self},
};
use zbus::{fdo::DBusProxy, names::BusName};

/// Whether `name` is owned on the bus: the current value, then one per change.
/// Asked to the bus itself, so it never triggers activation of the service.
pub(crate) async fn owner_watch(
    conn: &zbus::Connection,
    name: &'static str,
) -> anyhow::Result<impl Stream<Item = bool> + Send + 'static> {
    let dbus = DBusProxy::new(conn).await?;

    // Subscribe before asking, so a change in between isn't lost
    let changes = dbus
        .receive_name_owner_changed_with_args(&[(0, name)])
        .await?;
    let owned = dbus.name_has_owner(BusName::try_from(name)?).await?;

    Ok(stream::once(ready(owned)).chain(
        changes
            .filter_map(|signal| ready(signal.args().ok().map(|args| args.new_owner().is_some()))),
    ))
}
