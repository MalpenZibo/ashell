use iced::Element;

use crate::{components::icons::Icons, utils::powerprofiles::PowerProfilesCommand};

use super::{quick_setting_button, Message, Settings};

#[derive(Debug, Copy, Clone)]
pub enum Profiles {
    Balanced,
    Performance,
    PowerSaver,
}

impl From<Profiles> for Icons {
    fn from(profile: Profiles) -> Self {
        match profile {
            Profiles::Balanced => Icons::Balanced,
            Profiles::Performance => Icons::Performance,
            Profiles::PowerSaver => Icons::PowerSaver,
        }
    }
}

impl From<Profiles> for String {
    fn from(profile: Profiles) -> Self {
        match profile {
            Profiles::Balanced => "balanced".to_string(),
            Profiles::Performance => "performance".to_string(),
            Profiles::PowerSaver => "power-saver".to_string(),
        }
    }
}
#[derive(Debug, Clone)]
pub enum PowerProfilesMessage {
    Active(Profiles),
    Toggle,
}

impl PowerProfilesMessage {
    pub fn update(self, settings: &mut Settings) -> iced::Command<Message> {
        match self {
            PowerProfilesMessage::Active(state) => {
                settings.powerprofiles = Some(state);

                iced::Command::none()
            }
            PowerProfilesMessage::Toggle => {
                let _ = settings
                    .powerprofiles_commander
                    .send(PowerProfilesCommand::Toggle);

                iced::Command::none()
            }
        }
    }
}

pub fn get_powerprofiles_quick_setting_button<'a>(
    settings: &Settings,
) -> Option<(Element<'a, Message>, Option<Element<'a, Message>>)> {
    settings.powerprofiles.map(|state| {
        (
            quick_setting_button(
                state.into(),
                match state {
                    Profiles::Balanced => "Balanced",
                    Profiles::Performance => "Performance",
                    Profiles::PowerSaver => "Power Saver",
                }
                .to_string(),
                None,
                true,
                Message::PowerProfiles(PowerProfilesMessage::Toggle),
                None,
            ),
            None,
        )
    })
}
