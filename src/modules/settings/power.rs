use crate::{
    components::icons::{icon, Icons},
    style::GhostButtonStyle,
};
use iced::{
    theme::Button,
    widget::{button, column, horizontal_rule, row, text},
    Element, Length,
};

#[derive(Debug, Clone)]
pub enum PowerMessage {
    Suspend,
    Reboot,
    Shutdown,
    Logout,
}

impl PowerMessage {
    pub fn update(self) {
        match self {
            PowerMessage::Suspend => {
                crate::utils::launcher::suspend();
            }
            PowerMessage::Reboot => {
                crate::utils::launcher::reboot();
            }
            PowerMessage::Shutdown => {
                crate::utils::launcher::shutdown();
            }
            PowerMessage::Logout => {
                crate::utils::launcher::logout();
            }
        }
    }
}

pub fn power_menu<'a>() -> Element<'a, PowerMessage> {
    column!(
        button(row!(icon(Icons::Suspend), text("Suspend")).spacing(16))
            .padding([4, 12])
            .on_press(PowerMessage::Suspend)
            .width(Length::Fill)
            .style(Button::custom(GhostButtonStyle)),
        button(row!(icon(Icons::Reboot), text("Reboot")).spacing(16))
            .padding([4, 12])
            .on_press(PowerMessage::Reboot)
            .width(Length::Fill)
            .style(Button::custom(GhostButtonStyle)),
        button(row!(icon(Icons::Power), text("Shutdown")).spacing(16))
            .padding([4, 12])
            .on_press(PowerMessage::Shutdown)
            .width(Length::Fill)
            .style(Button::custom(GhostButtonStyle)),
        horizontal_rule(1),
        button(row!(icon(Icons::Logout), text("Logout")).spacing(16))
            .padding([4, 12])
            .on_press(PowerMessage::Logout)
            .width(Length::Fill)
            .style(Button::custom(GhostButtonStyle)),
    )
    .padding(8)
    .width(Length::Fill)
    .spacing(8)
    .into()
}
