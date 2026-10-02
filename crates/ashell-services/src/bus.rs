use futures::{Stream, StreamExt};
use zbus::{fdo::DBusProxy, names::BusName};

/// Whether `name` is owned on the bus. Asked to the bus itself, so it never
/// triggers activation of the service.
pub(crate) async fn has_owner(conn: &zbus::Connection, name: &'static str) -> anyhow::Result<bool> {
    Ok(DBusProxy::new(conn)
        .await?
        .name_has_owner(BusName::try_from(name)?)
        .await?)
}

/// Resolves once `name` is owned on the bus.
pub(crate) async fn owner_acquired(
    conn: &zbus::Connection,
    name: &'static str,
) -> anyhow::Result<()> {
    let dbus = DBusProxy::new(conn).await?;
    let mut owner_changes = dbus
        .receive_name_owner_changed_with_args(&[(0, name)])
        .await?;

    // The owner may have appeared before the subscription was in place
    if dbus.name_has_owner(BusName::try_from(name)?).await? {
        return Ok(());
    }

    while let Some(signal) = owner_changes.next().await {
        if signal.args()?.new_owner().is_some() {
            return Ok(());
        }
    }

    Ok(())
}

/// Yields whenever `name` gains or loses its owner.
pub(crate) async fn owner_changes(
    conn: &zbus::Connection,
    name: &'static str,
) -> anyhow::Result<impl Stream<Item = ()> + Send + 'static> {
    Ok(DBusProxy::new(conn)
        .await?
        .receive_name_owner_changed_with_args(&[(0, name)])
        .await?
        .map(|_| {}))
}
