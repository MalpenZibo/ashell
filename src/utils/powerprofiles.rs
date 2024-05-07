use crate::modules::settings::powerprofiles::{PowerProfilesMessage, Profiles};
use iced::{
    futures::{FutureExt, SinkExt, StreamExt},
    Subscription,
};
use zbus::{proxy, Result};

#[proxy(
    default_service = "org.freedesktop.UPower.PowerProfiles",
    default_path = "/org/freedesktop/UPower/PowerProfiles",
    interface = "org.freedesktop.UPower.PowerProfiles"
)]
trait PowerProfiles {
    #[zbus(property)]
    fn active_profile(&self) -> Result<String>;

    #[zbus(property)]
    fn set_active_profile(&self, profile: &str) -> Result<()>;
}

pub enum PowerProfilesCommand {
    Toggle,
}

const POWER_PROFILES: [Profiles; 3] = [
    Profiles::PowerSaver,
    Profiles::Balanced,
    Profiles::Performance,
];

pub fn subscription(
    rx: Option<tokio::sync::mpsc::UnboundedReceiver<PowerProfilesCommand>>,
) -> Subscription<PowerProfilesMessage> {
    iced::subscription::channel(
        "powerprofiles-dbus-connection-listener",
        100,
        |mut output| async move {
            let mut rx = rx.expect("Failed to get commander receiver");
            let conn = zbus::Connection::system()
                .await
                .expect("Failed to connect to system bus");

            let powerprofiles = PowerProfilesProxy::new(&conn)
                .await
                .expect("Failed to create PowerProfilesProxy");

            let active_profile = powerprofiles.active_profile().await.unwrap();
            let mut current_profile_index = POWER_PROFILES
                .iter()
                .position(|profile| Into::<String>::into(*profile) == active_profile)
                .unwrap();

            let _ = output
                .send(PowerProfilesMessage::Active(
                    POWER_PROFILES[current_profile_index],
                ))
                .await;

            let mut active_profile_signal = powerprofiles.receive_active_profile_changed().await;

            loop {
                iced::futures::select! {
                    v = rx.recv().fuse() => {
                        if let Some(v) = v {
                            match v {
                                PowerProfilesCommand::Toggle => {
                                    current_profile_index = (current_profile_index + 1) % POWER_PROFILES.len();
                                    let new_profile = Into::<String>::into(POWER_PROFILES[current_profile_index]);
                                    let _ = powerprofiles.set_active_profile(&new_profile).await;
                                }
                            }
                        }
                    },
                    v = active_profile_signal.next().fuse() => {
                        if let Some(new_profile) = v {
                            if let Ok(new_profile) = new_profile.get().await {
                                current_profile_index = POWER_PROFILES
                                    .iter()
                                    .position(
                                        |profile| Into::<String>::into(*profile) == new_profile
                                    )
                                    .unwrap();
                                let _ = output
                                    .send(PowerProfilesMessage::Active(
                                        POWER_PROFILES[current_profile_index]
                                    )).await;
                            }
                        }
                    }
                }
            }
        },
    )
}
