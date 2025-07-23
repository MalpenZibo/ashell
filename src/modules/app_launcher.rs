use crate::components::icons::{Icons, icon};
use iced::Element;

#[derive(Debug, Clone)]
pub enum Message {}

#[derive(Debug, Clone)]
pub struct AppLauncher {
    command: String,
}

impl AppLauncher {
    pub fn new(command: String) -> Self {
        Self { command }
    }

    pub fn view(&self) -> Element<'_, Message> {
        icon(Icons::AppLauncher).into()
    }
}
