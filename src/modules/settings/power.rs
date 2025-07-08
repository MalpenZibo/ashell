use crate::{
    components::icons::{Icons, icon},
    config::SettingsModuleConfig,
    style::ghost_button_style,
    utils,
};
use iced::{
    Element, Length,
    widget::{button, column, horizontal_rule, row, text},
};

#[derive(Debug, Clone)]
pub enum PowerMessage {
    Suspend(String),
    Reboot(String),
    Shutdown(String),
    Logout(String),
}

impl PowerMessage {
    pub fn update(self) {
        match self {
            PowerMessage::Suspend(cmd) => {
                utils::launcher::suspend(cmd);
            }
            PowerMessage::Reboot(cmd) => {
                utils::launcher::reboot(cmd);
            }
            PowerMessage::Shutdown(cmd) => {
                utils::launcher::shutdown(cmd);
            }
            PowerMessage::Logout(cmd) => {
                utils::launcher::logout(cmd);
            }
        }
    }
}

pub fn power_menu<'a>(opacity: f32, config: &SettingsModuleConfig) -> Element<'a, PowerMessage> {
    column!(
        button(row!(icon(Icons::Suspend), text("Suspend")).spacing(16))
            .padding([4, 12])
            .on_press(PowerMessage::Suspend(config.suspend_cmd.clone()))
            .width(Length::Fill)
            .style(ghost_button_style(opacity)),
        button(row!(icon(Icons::Reboot), text("Reboot")).spacing(16))
            .padding([4, 12])
            .on_press(PowerMessage::Reboot(config.reboot_cmd.clone()))
            .width(Length::Fill)
            .style(ghost_button_style(opacity)),
        button(row!(icon(Icons::Power), text("Shutdown")).spacing(16))
            .padding([4, 12])
            .on_press(PowerMessage::Shutdown(config.shutdown_cmd.clone()))
            .width(Length::Fill)
            .style(ghost_button_style(opacity)),
        horizontal_rule(1),
        button(row!(icon(Icons::Logout), text("Logout")).spacing(16))
            .padding([4, 12])
            .on_press(PowerMessage::Logout(config.logout_cmd.clone()))
            .width(Length::Fill)
            .style(ghost_button_style(opacity)),
    )
    .padding(8)
    .width(Length::Fill)
    .spacing(8)
    .into()
}
