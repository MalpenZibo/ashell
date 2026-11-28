use futures::{
    Stream, StreamExt,
    future::ready,
    stream::{self},
};
#[cfg(any(feature = "bluetooth", feature = "network"))]
use zbus::{
    MatchRule, MessageStream,
    fdo::{PropertiesChanged, PropertiesChangedArgs},
    match_rule::Builder,
    message::Type,
};
use zbus::{fdo::DBusProxy, names::BusName};

/// Whether `name` is owned on the bus right now. Asked to the bus itself, so
/// it never triggers activation of the service.
#[cfg(any(feature = "network", feature = "upower"))]
pub(crate) async fn has_owner(conn: &zbus::Connection, name: &str) -> anyhow::Result<bool> {
    Ok(DBusProxy::new(conn)
        .await?
        .name_has_owner(BusName::try_from(name)?)
        .await?)
}

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

/// Every `PropertiesChanged` that `sender` emits for the objects under
/// `path_namespace`: one match rule instead of a stream per object.
#[cfg(feature = "bluetooth")]
pub(crate) async fn properties_changed(
    conn: &zbus::Connection,
    sender: &str,
    path_namespace: &str,
) -> anyhow::Result<impl Stream<Item = PropertiesChanged> + Send + 'static> {
    subscribe(conn, properties_changed_rule(sender, path_namespace)).await
}

/// The same for the properties of `interfaces` only, one rule each: the bus
/// doesn't send the others at all, instead of the stream dropping them.
#[cfg(feature = "network")]
pub(crate) async fn interface_properties_changed(
    conn: &zbus::Connection,
    sender: &str,
    path_namespace: &str,
    interfaces: &[&str],
) -> anyhow::Result<impl Stream<Item = PropertiesChanged> + Send + 'static> {
    let subscriptions = interfaces.iter().map(|interface| {
        subscribe(
            conn,
            properties_changed_rule(sender, path_namespace).arg(0, *interface),
        )
    });

    Ok(stream::select_all(
        futures::future::try_join_all(subscriptions)
            .await?
            .into_iter()
            .map(StreamExt::boxed),
    ))
}

#[cfg(any(feature = "bluetooth", feature = "network"))]
fn properties_changed_rule<'m>(sender: &'m str, path_namespace: &'m str) -> Builder<'m> {
    MatchRule::builder()
        .msg_type(Type::Signal)
        .sender(sender)
        .interface("org.freedesktop.DBus.Properties")
        .member("PropertiesChanged")
        .path_namespace(path_namespace)
}

#[cfg(any(feature = "bluetooth", feature = "network"))]
async fn subscribe(
    conn: &zbus::Connection,
    rule: Builder<'_>,
) -> anyhow::Result<impl Stream<Item = PropertiesChanged> + Send + use<>> {
    Ok(MessageStream::for_match_rule(rule.build()?, conn, None)
        .await?
        .filter_map(|message| ready(message.ok().and_then(PropertiesChanged::from_message))))
}

/// Whether the change sets or invalidates any of `names`.
#[cfg(any(feature = "bluetooth", feature = "network"))]
pub(crate) fn changes_any(args: &PropertiesChangedArgs<'_>, names: &[&str]) -> bool {
    args.changed_properties()
        .keys()
        .chain(args.invalidated_properties().iter())
        .any(|name| names.contains(name))
}
