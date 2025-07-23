use crate::{
    components::icons::{Icons, icon},
    utils::launcher::execute_command,
};
use iced::Element;

#[derive(Debug, Clone)]
pub enum Message {
    Launch,
}

#[derive(Debug, Clone)]
pub struct Clipboard {
    command: String,
}

impl Clipboard {
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

    pub fn view(&self) -> Element<Message> {
        icon(Icons::Clipboard).into()
    }
}
