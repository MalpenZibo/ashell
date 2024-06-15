use iced::Element;

use crate::{
    components::icons::{icon, Icons},
    style::{GREEN, RED},
    utils::{powerprofiles::PowerProfilesCommand, Commander},
};

use super::{quick_setting_button, Message};

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

pub struct PowerProfiles {
    commander: Commander<PowerProfilesCommand>,
    profiles: Option<Profiles>,
}

impl PowerProfiles {
    pub fn new() -> Self {
        Self {
            commander: Commander::new(),
            profiles: None,
        }
    }

    pub fn update(&mut self, msg: PowerProfilesMessage) -> iced::Command<Message> {
        match msg {
            PowerProfilesMessage::Active(state) => {
                self.profiles = Some(state);

                iced::Command::none()
            }
            PowerProfilesMessage::Toggle => {
                let _ = self.commander.send(PowerProfilesCommand::Toggle);

                iced::Command::none()
            }
        }
    }

    pub fn indicator(&self) -> Option<Element<Message>> {
        self.profiles.and_then(|v| match v {
            Profiles::Balanced => None,
            Profiles::Performance => Some(icon(Icons::Performance).style(RED).into()),
            Profiles::PowerSaver => Some(icon(Icons::PowerSaver).style(GREEN).into()),
        })
    }

    pub fn get_quick_setting_button(&self) -> Option<(Element<Message>, Option<Element<Message>>)> {
        self.profiles.map(|state| {
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

    pub fn subscription(&self) -> iced::Subscription<PowerProfilesMessage> {
        crate::utils::powerprofiles::subscription(self.commander.give_receiver())
    }
}
