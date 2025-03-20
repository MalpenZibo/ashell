use crate::{
    components::icons::{Icons, icon},
    style::GhostButtonStyle,
    utils,
};
use iced::{
    Element, Length,
    widget::{button, column, horizontal_rule, row, text},
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
                utils::launcher::suspend();
            }
            PowerMessage::Reboot => {
                utils::launcher::reboot();
            }
            PowerMessage::Shutdown => {
                utils::launcher::shutdown();
            }
            PowerMessage::Logout => {
                utils::launcher::logout();
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
            .style(GhostButtonStyle.into_style()),
        button(row!(icon(Icons::Reboot), text("Reboot")).spacing(16))
            .padding([4, 12])
            .on_press(PowerMessage::Reboot)
            .width(Length::Fill)
            .style(GhostButtonStyle.into_style()),
        button(row!(icon(Icons::Power), text("Shutdown")).spacing(16))
            .padding([4, 12])
            .on_press(PowerMessage::Shutdown)
            .width(Length::Fill)
            .style(GhostButtonStyle.into_style()),
        horizontal_rule(1),
        button(row!(icon(Icons::Logout), text("Logout")).spacing(16))
            .padding([4, 12])
            .on_press(PowerMessage::Logout)
            .width(Length::Fill)
            .style(GhostButtonStyle.into_style()),
    )
    .padding(8)
    .width(Length::Fill)
    .spacing(8)
    .into()
}
