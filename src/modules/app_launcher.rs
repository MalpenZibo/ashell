use crate::{
    components::icons::{StaticIcon, icon},
    utils::launcher::execute_command,
};
use iced::Element;

#[derive(Debug, Clone)]
pub enum Message {
    Launch,
}

#[derive(Debug, Clone)]
pub struct AppLauncher {
    command: String,
}

impl AppLauncher {
    pub fn new(command: String) -> Self {
        Self { command }
    }

    pub fn update(&mut self, message: Message) {
        match message {
            Message::Launch => {
                execute_command(self.command.clone());
            }
        }
    }

    pub fn view(&'_ self) -> Element<'_, Message> {
        icon(StaticIcon::AppLauncher).into()
    }
}
