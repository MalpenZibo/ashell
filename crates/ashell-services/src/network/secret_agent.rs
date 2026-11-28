//! The NetworkManager secret agent: NetworkManager asks it for the
//! passphrase of a Wi-Fi network it can't join, and the activation waits for
//! the answer. Its registration also withdraws the requests of a daemon that
//! stops.

use super::{
    IWD, NETWORK_MANAGER, Redacted,
    dbus::{ConnectionSettingsMap, connection_ssid},
    password::{Asker, PasswordReply, PasswordRequests},
    property,
};
use crate::{
    bus,
    retry::{RETRY_DELAY, failed},
};
use futures::{
    FutureExt, StreamExt,
    future::{self, Either, select},
    stream,
};
use log::{debug, error, info, warn};
use std::{
    collections::HashMap,
    pin::pin,
    sync::{Arc, Mutex, MutexGuard, PoisonError},
    time::Duration,
};
use zbus::{
    DBusError, OwnedObjectPath, OwnedVariant, Variant,
    fdo::DBusProxy,
    interface,
    names::{OwnedUniqueName, WellKnownName},
    proxy,
};

/// NetworkManager calls every agent at this path.
const AGENT_PATH: &str = "/org/freedesktop/NetworkManager/SecretAgent";
const IDENTIFIER: &str = "org.ashell.SecretAgent";

const WIRELESS_SECURITY: &str = "802-11-wireless-security";

/// `NM_SECRET_AGENT_GET_SECRETS_FLAG_ALLOW_INTERACTION`: the user may be asked.
const ALLOW_INTERACTION: u32 = 0x1;

const FIRST_RETRY_DELAY: Duration = Duration::from_millis(100);

#[derive(Debug, DBusError)]
#[zbus(prefix = "org.freedesktop.NetworkManager.SecretAgent")]
enum AgentError {
    #[zbus(error)]
    ZBus(zbus::Error),
    /// NetworkManager asks the next agent.
    NoSecrets(String),
    /// NetworkManager asks no other agent, and the activation fails.
    UserCanceled(String),
}

type Secrets = HashMap<String, HashMap<String, OwnedVariant>>;

struct SecretAgent {
    requests: PasswordRequests,
}

#[interface(name = "org.freedesktop.NetworkManager.SecretAgent")]
impl SecretAgent {
    async fn get_secrets(
        &self,
        connection: ConnectionSettingsMap,
        connection_path: OwnedObjectPath,
        setting_name: String,
        hints: Vec<String>,
        flags: u32,
    ) -> Result<Secrets, AgentError> {
        let ssid = match wifi_passphrase_request(&connection, &setting_name, &hints, flags) {
            Ok(ssid) => ssid,
            Err(reason) => {
                debug!(
                    "Leaving the {setting_name} secrets of {connection_path} to other agents: {reason}"
                );
                return Err(AgentError::NoSecrets(reason.to_owned()));
            }
        };

        debug!("NetworkManager asks for the passphrase of {ssid}");
        let asker = Asker::NetworkManager {
            connection: connection_path,
            setting: setting_name,
        };
        match self.requests.ask(ssid, asker).await {
            PasswordReply::Password(password) => Ok(passphrase_secrets(password)?),
            PasswordReply::Cancel => Err(AgentError::UserCanceled(
                "The user canceled the request".to_owned(),
            )),
            PasswordReply::Decline => Err(AgentError::NoSecrets("No answer".to_owned())),
        }
    }

    fn cancel_get_secrets(&self, connection_path: OwnedObjectPath, setting_name: String) {
        debug!("NetworkManager cancels the {setting_name} request of {connection_path}");
        self.requests.withdraw(|asker| {
            matches!(asker, Asker::NetworkManager { connection, setting }
                if *connection == connection_path && *setting == setting_name)
        });
    }

    /// ashell keeps no secrets: NetworkManager stores the ones it owns, and
    /// those flagged as agent-owned are asked again next time.
    fn save_secrets(&self, _connection: ConnectionSettingsMap, _connection_path: OwnedObjectPath) {}

    fn delete_secrets(
        &self,
        _connection: ConnectionSettingsMap,
        _connection_path: OwnedObjectPath,
    ) {
    }
}

/// The SSID whose passphrase to ask for, or why the request is left to other
/// agents: ashell asks only for the passphrase of WPA, WPA2 and WPA3 Personal
/// networks, and keeps no secrets to answer without asking.
fn wifi_passphrase_request(
    connection: &ConnectionSettingsMap,
    setting_name: &str,
    hints: &[String],
    flags: u32,
) -> Result<String, &'static str> {
    if flags & ALLOW_INTERACTION == 0 {
        return Err("the user can't be asked");
    }
    if setting_name != WIRELESS_SECURITY {
        return Err("not a Wi-Fi passphrase");
    }
    if hints.iter().any(|hint| hint != "psk") {
        return Err("asks for more than the passphrase");
    }
    let key_mgmt = connection
        .get(WIRELESS_SECURITY)
        .and_then(|security| property::<&str>(security, "key-mgmt"));
    if !matches!(key_mgmt, Some("wpa-psk" | "sae")) {
        return Err("not a WPA or WPA3 Personal network");
    }

    connection_ssid(connection)
        .filter(|ssid| !ssid.is_empty())
        .ok_or("no SSID")
}

fn passphrase_secrets(password: Redacted<String>) -> zbus::Result<Secrets> {
    let psk = OwnedVariant::try_from(Variant::from(password.into_inner()))?;

    Ok(HashMap::from([(
        WIRELESS_SECURITY.to_owned(),
        HashMap::from([("psk".to_owned(), psk)]),
    )]))
}

#[proxy(
    interface = "org.freedesktop.NetworkManager.AgentManager",
    default_service = "org.freedesktop.NetworkManager",
    default_path = "/org/freedesktop/NetworkManager/AgentManager"
)]
trait AgentManager {
    fn register(&self, identifier: &str) -> zbus::Result<()>;
}

/// Which NetworkManager instance the agent of a handle is registered with.
#[derive(Debug, Default)]
pub(super) struct Registration {
    /// Held by the one stream of the handle that keeps the agent registered.
    turn: tokio::sync::Mutex<()>,
    /// The instance `Register` was sent to, and whether it replied. A stream
    /// that went away while waiting leaves it unreplied: that instance then
    /// refusing a new `Register` means it holds the one sent before.
    sent: Mutex<Option<(OwnedUniqueName, bool)>>,
}

impl Registration {
    fn sent(&self) -> MutexGuard<'_, Option<(OwnedUniqueName, bool)>> {
        self.sent.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

enum Owner {
    NetworkManager { owned: bool, changed: bool },
    IwdChanged,
}

/// Keeps the secret agent of `conn` registered with the NetworkManager
/// running, and withdraws the requests of a daemon that stops or restarts.
/// The streams of a handle take turns: one serves at a time.
pub(super) async fn serve(
    conn: zbus::Connection,
    requests: PasswordRequests,
    registration: Arc<Registration>,
) {
    let _turn = registration.turn.lock().await;

    let agent = SecretAgent {
        requests: requests.clone(),
    };
    if let Err(err) = conn.object_server().at(AGENT_PATH, agent).await {
        error!("Failed to serve the NetworkManager secret agent: {err}");
        return;
    }

    // The first owner of each is the current one: only the next ones are changes
    let owners = async {
        let network_manager = bus::owner_watch(&conn, NETWORK_MANAGER)
            .await?
            .enumerate()
            .map(|(index, owned)| Owner::NetworkManager {
                owned,
                changed: index > 0,
            });
        let iwd = bus::owner_watch(&conn, IWD)
            .await?
            .skip(1)
            .map(|_| Owner::IwdChanged);
        anyhow::Ok(stream::select(network_manager, iwd))
    };
    let mut owners = match owners.await {
        Ok(owners) => pin!(owners),
        Err(err) => {
            error!("Failed to watch for network backends on the system bus: {err}");
            return;
        }
    };

    let mut registering = future::pending().boxed();
    loop {
        let owner = match select(&mut registering, owners.next()).await {
            Either::Left(((), _)) => None,
            Either::Right((owner, _)) => Some(owner),
        };
        match owner {
            None => registering = future::pending().boxed(),
            Some(None) => return,
            Some(Some(Owner::NetworkManager { owned, changed })) => {
                if changed {
                    requests.withdraw(Asker::is_network_manager);
                    *registration.sent() = None;
                }
                registering = if owned {
                    keep_registered(conn.clone(), registration.clone()).boxed()
                } else {
                    future::pending().boxed()
                };
            }
            Some(Some(Owner::IwdChanged)) => requests.withdraw(Asker::is_iwd),
        }
    }
}

/// Registers the agent with the NetworkManager running, retrying until it
/// succeeds: NetworkManager owns its name before it serves the AgentManager.
/// Gives up when another agent of this user has the identifier.
async fn keep_registered(conn: zbus::Connection, registration: Arc<Registration>) {
    let mut delay = FIRST_RETRY_DELAY;
    let mut failing = false;
    loop {
        match register(&conn, &registration).await {
            Ok(()) => return,
            Err(err) if refused(&err) => {
                warn!("Failed to register as a NetworkManager secret agent: {err}");
                return;
            }
            Err(err) if delay < RETRY_DELAY => {
                debug!("Failed to register as a NetworkManager secret agent, retrying: {err}");
            }
            Err(err) => failed!(
                &mut failing,
                "Failed to register as a NetworkManager secret agent, retrying: {err}"
            ),
        }
        tokio::time::sleep(delay).await;
        delay = (delay * 2).min(RETRY_DELAY);
    }
}

/// Registers the agent with the NetworkManager running now, which forgets it
/// when it stops, unless it is registered with it already.
async fn register(conn: &zbus::Connection, registration: &Registration) -> zbus::Result<()> {
    let owner = DBusProxy::new(conn)
        .await?
        .get_name_owner(WellKnownName::from_static_str_unchecked(NETWORK_MANAGER).into())
        .await?;
    let sent_before = match &*registration.sent() {
        Some((sent, replied)) if *sent == owner => {
            if *replied {
                return Ok(());
            }
            true
        }
        _ => false,
    };

    let manager = AgentManagerProxy::builder(conn)
        .cache_properties(zbus::proxy::CacheProperties::No)
        .build()
        .await?;
    *registration.sent() = Some((owner.clone(), false));
    match manager.register(IDENTIFIER).await {
        Ok(()) => {}
        Err(err) if sent_before && refused(&err) => {}
        Err(err) => {
            *registration.sent() = None;
            return Err(err);
        }
    }
    *registration.sent() = Some((owner, true));

    info!("Registered as a NetworkManager secret agent");
    Ok(())
}

/// NetworkManager accepts one agent per identifier and user.
fn refused(err: &zbus::Error) -> bool {
    matches!(
        err,
        zbus::Error::MethodError(name, _, _)
            if name.as_str() == "org.freedesktop.NetworkManager.AgentManager.PermissionDenied"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const REQUEST_NEW: u32 = 0x2;

    fn variant<'a>(value: impl Into<Variant<'a>>) -> OwnedVariant {
        OwnedVariant::try_from(value.into()).unwrap()
    }

    fn wifi(key_mgmt: Option<&str>) -> ConnectionSettingsMap {
        let mut connection = HashMap::from([(
            "802-11-wireless".to_string(),
            HashMap::from([("ssid".to_string(), variant(b"Home".to_vec()))]),
        )]);
        if let Some(key_mgmt) = key_mgmt {
            connection.insert(
                WIRELESS_SECURITY.to_string(),
                HashMap::from([("key-mgmt".to_string(), variant(key_mgmt))]),
            );
        }
        connection
    }

    fn request(
        connection: &ConnectionSettingsMap,
        setting_name: &str,
        hints: &[&str],
        flags: u32,
    ) -> Result<String, &'static str> {
        let hints: Vec<_> = hints.iter().map(|hint| hint.to_string()).collect();
        wifi_passphrase_request(connection, setting_name, &hints, flags)
    }

    #[test]
    fn wpa_and_sae_passphrases_are_asked_for() {
        for key_mgmt in ["wpa-psk", "sae"] {
            let connection = wifi(Some(key_mgmt));
            for flags in [ALLOW_INTERACTION, ALLOW_INTERACTION | REQUEST_NEW] {
                assert_eq!(
                    request(&connection, WIRELESS_SECURITY, &[], flags).as_deref(),
                    Ok("Home")
                );
            }
            assert_eq!(
                request(&connection, WIRELESS_SECURITY, &["psk"], ALLOW_INTERACTION).as_deref(),
                Ok("Home")
            );
        }
    }

    #[test]
    fn nothing_is_answered_without_interaction() {
        let connection = wifi(Some("wpa-psk"));

        for flags in [0, REQUEST_NEW] {
            assert!(request(&connection, WIRELESS_SECURITY, &[], flags).is_err());
        }
    }

    #[test]
    fn other_secrets_are_left_to_other_agents() {
        let wpa = wifi(Some("wpa-psk"));
        assert!(request(&wpa, "802-1x", &[], ALLOW_INTERACTION).is_err());
        assert!(request(&wpa, "vpn", &[], ALLOW_INTERACTION).is_err());
        assert!(request(&wpa, WIRELESS_SECURITY, &["wep-key0"], ALLOW_INTERACTION).is_err());

        for key_mgmt in [Some("wpa-eap"), Some("none"), Some("owe"), None] {
            assert!(request(&wifi(key_mgmt), WIRELESS_SECURITY, &[], ALLOW_INTERACTION).is_err());
        }

        let mut no_ssid = wpa.clone();
        no_ssid.remove("802-11-wireless");
        assert!(request(&no_ssid, WIRELESS_SECURITY, &[], ALLOW_INTERACTION).is_err());
    }

    #[test]
    fn the_answer_is_the_psk() {
        let secrets = passphrase_secrets(Redacted::new("hunter22".to_string())).unwrap();

        let psk = secrets
            .get(WIRELESS_SECURITY)
            .and_then(|security| property::<&str>(security, "psk"));
        assert_eq!(psk, Some("hunter22"));
    }
}
