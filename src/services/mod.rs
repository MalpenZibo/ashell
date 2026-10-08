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
