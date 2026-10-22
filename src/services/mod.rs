use iced::{Subscription, Task};

pub mod audio;
pub mod bluetooth;
pub mod brightness;
pub mod compositor;
pub mod idle_inhibitor;
pub mod logind;
pub mod mpris;
pub mod network;
pub mod notifications;
pub mod privacy;
mod throttle;
pub mod tray;
pub mod upower;
pub mod xdg_icons;

/// The pid of the process behind the D-Bus connection `name`.
pub async fn connection_pid(conn: &zbus::Connection, name: &str) -> anyhow::Result<u32> {
    let dbus = zbus::fdo::DBusProxy::new(conn).await?;

    Ok(dbus
        .get_connection_unix_process_id(name.try_into()?)
        .await?)
}

/// Whether a service error is worth retrying.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetryPolicy {
    /// The backend may come back: a daemon restarting, or a service that
    /// starts after ashell at session startup.
    Transient,
    /// Retrying cannot help in this environment, e.g. missing hardware.
    Permanent,
}

/// A service failure together with the policy that decides how to react.
#[derive(Debug)]
pub struct ServiceError {
    policy: RetryPolicy,
    error: anyhow::Error,
}

impl ServiceError {
    pub fn transient(error: impl Into<anyhow::Error>) -> Self {
        Self {
            policy: RetryPolicy::Transient,
            error: error.into(),
        }
    }

    pub fn permanent(error: impl Into<anyhow::Error>) -> Self {
        Self {
            policy: RetryPolicy::Permanent,
            error: error.into(),
        }
    }

    pub fn policy(&self) -> RetryPolicy {
        self.policy
    }
}

impl std::fmt::Display for ServiceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.error.fmt(f)
    }
}

/// How long a service waits before retrying a transient failure.
const RETRY_DELAY: std::time::Duration = std::time::Duration::from_secs(5);

/// Wait before the next attempt, or park forever when retrying cannot help.
pub async fn wait_for_retry(error: &ServiceError) {
    match error.policy() {
        RetryPolicy::Transient => tokio::time::sleep(RETRY_DELAY).await,
        RetryPolicy::Permanent => std::future::pending().await,
    }
}

#[derive(Debug, Clone)]
pub enum ServiceEvent<S: ReadOnlyService> {
    Init(S),
    Update(S::UpdateEvent),
    Error(S::Error),
}

pub trait Service: ReadOnlyService {
    type Command;

    fn command(&mut self, command: Self::Command) -> Task<ServiceEvent<Self>>;
}

pub trait ReadOnlyService: Sized {
    type UpdateEvent;
    type Error: Clone;

    fn update(&mut self, event: Self::UpdateEvent);

    fn subscribe() -> Subscription<ServiceEvent<Self>>;
}
