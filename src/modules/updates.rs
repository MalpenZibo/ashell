use crate::{
    components::icons::{icon, Icons},
    style::HeaderButtonStyle,
};
use iced::{
    widget::{button, row, text},
    Element,
};
use serde::Deserialize;
use std::{process::Stdio, time::Duration};
use tokio::{process::Command, time::sleep};

#[derive(Deserialize, Debug, Clone)]
pub struct Update {
    pub package: String,
    pub from: String,
    pub to: String,
}

async fn check_update_now() -> Vec<Update> {
    let check_update_cmd = Command::new("bash")
        .arg("-c")
        .arg("checkupdates; paru -Qua ")
        .stdout(Stdio::piped())
        .output()
        .await;

    match check_update_cmd {
        Ok(check_update_cmd) => {
            let cmd_output = String::from_utf8_lossy(&check_update_cmd.stdout);
            let mut new_updates: Vec<Update> = Vec::new();
            for update in cmd_output.split('\n') {
                if update.is_empty() {
                    continue;
                }

                let data = update.split(' ').collect::<Vec<&str>>();
                if data.len() < 4 {
                    continue;
                }
                new_updates.push(Update {
                    package: data[0].to_string(),
                    from: data[1].to_string(),
                    to: data[3].to_string(),
                });
            }

            new_updates
        }
        Err(e) => {
            println!("Error: {:?}", e);
            vec![]
        }
    }
}

async fn update() {
    let _ = Command::new("bash")
            .arg("-c")
            .arg("alacritty -e bash -c \"paru; flatpak update; echo Done - Press enter to exit; read\" &")
            .output().await;
}

#[derive(Debug, Clone)]
pub enum Message {
    ToggleMenu,
    InternalMessage(InternalMessage),
}

#[derive(Debug, Clone)]
pub enum InternalMessage {
    UpdatesCheckCompleted(Vec<Update>),
}

enum State {
    Checking,
    Ready,
}
pub struct Updates {
    state: State,
    updates: Vec<Update>,
}

impl Updates {
    pub fn new() -> Self {
        Self {
            state: State::Checking,
            updates: vec![],
        }
    }

    pub fn update(&mut self, message: InternalMessage) {
        match message {   
            InternalMessage::UpdatesCheckCompleted(updates) => {
                self.updates = updates;
                self.state = State::Ready;
            }  
        }  
    }  
  
    pub fn view(&self) -> Element<Message> {
        let mut content = row!(icon(match self.state {
            State::Checking => Icons::Refresh,
            State::Ready if self.updates.is_empty() => Icons::NoUpdatesAvailable,
            _ => Icons::UpdatesAvailable,
        }))  
        .spacing(4);   
     
        if !self.updates.is_empty() {
            content = content.push(text(self.updates.len()));
        }   
  
        button(content)  
            .style(iced::theme::Button::custom(HeaderButtonStyle))
            .on_press(Message::ToggleMenu)
            .into()  
    }  

    pub fn subscription(&self) -> iced::Subscription<InternalMessage> {
        iced::subscription::channel("update-checker", 10, |mut output| async move {
            let updates = check_update_now().await;

            let _ = output.try_send(InternalMessage::UpdatesCheckCompleted(updates));

            loop {
                sleep(Duration::from_secs(60 * 30)).await;

                let updates = check_update_now().await;
                let _ = output.try_send(InternalMessage::UpdatesCheckCompleted(updates));
            }
        })
    }
}

#[derive(Debug, Clone)]
pub enum UpdateMenuMessage {}

#[derive(Debug)]
pub struct UpdateMenu {}

pub trait MenuItem {
    type Message;

    fn update(&mut self, message: Self::Message);

    fn view(&self) -> Element<Self::Message>;
}

impl MenuItem for UpdateMenu {
    type Message = UpdateMenuMessage;

    fn update(&mut self, message: Self::Message) {}

    fn view(&self) -> Element<Self::Message> {
        text("Hello from update menu").into()
    }
}
