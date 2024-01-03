use crate::{
    app::MenuRequest,
    components::icons::{icon, Icons},
    menu::MenuOutput,
    style::{GhostButtonStyle, HeaderButtonStyle},
};
use iced::{
    widget::{button, column, container, horizontal_rule, row, scrollable, text, Column},
    Element, Length,
};
use serde::Deserialize;
use std::{process::Stdio, time::Duration};
use tokio::{process::Command, sync::mpsc::UnboundedSender, time::sleep};

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
    UpdatesCheckInit,
    UpdatesCheckCompleted(Vec<Update>),
    UpdateFinished,
    UpdatesRefreshFromMenu(Vec<Update>),
}

#[derive(Debug, Clone, Eq, PartialEq)]
enum State {
    Checking,
    Ready,
}
pub struct Updates {
    state: State,
    pub updates: Vec<Update>,
}

impl Updates {
    pub fn new() -> Self {
        Self {
            state: State::Checking,
            updates: vec![],
        }
    }

    pub fn update(&mut self, message: Message) -> Option<MenuRequest> {
        match message {
            Message::UpdatesCheckCompleted(updates) => {
                self.updates = updates;
                self.state = State::Ready;

                Some(MenuRequest::NotifyNewUpdates(&self.updates))
            }
            Message::ToggleMenu => Some(MenuRequest::Updates(&self.updates)),
            Message::UpdatesCheckInit => {
                self.state = State::Checking;
                None
            }
            Message::UpdateFinished => {
                self.updates.clear();
                self.state = State::Ready;
                None
            }
            Message::UpdatesRefreshFromMenu(updates) => {
                self.updates = updates;
                self.state = State::Ready;

                None
            }
        }
    }

    pub fn view(&self) -> Element<Message> {
        let mut content = row!(container(icon(match self.state {
            State::Checking => Icons::Refresh,
            State::Ready if self.updates.is_empty() => Icons::NoUpdatesAvailable,
            _ => Icons::UpdatesAvailable,
        }))
        .padding([0, 1]))
        .align_items(iced::Alignment::Center)
        .spacing(4);

        if !self.updates.is_empty() {
            content = content.push(text(self.updates.len()));
        }

        button(content)
            .style(iced::theme::Button::custom(HeaderButtonStyle::Full))
            .on_press(Message::ToggleMenu)
            .into()
    }

    pub fn subscription(&self) -> iced::Subscription<Message> {
        iced::subscription::channel("update-checker", 10, |mut output| async move {
            loop {
                let updates = check_update_now().await;

                let _ = output.try_send(Message::UpdatesCheckCompleted(updates));

                sleep(Duration::from_secs(10)).await;
            }
        })
    }
}

pub enum UpdateMenuOutput {
    UpdateFinished,
    UpdatesCheckInit,
    UpdatesCheckCompleted(Vec<Update>),
}

#[derive(Debug, Clone)]
pub enum UpdateMenuMessage {
    UpdatesCheckCompleted(Vec<Update>),
    ToggleUpdatesList,
    CheckNow,
    Update,
    UpdateFinished,
}

#[derive(Debug)]
pub struct UpdateMenu {
    output_tx: UnboundedSender<MenuOutput>,
    state: State,
    updates: Vec<Update>,
    is_updates_list_open: bool,
}

impl UpdateMenu {
    pub fn new(output_tx: UnboundedSender<MenuOutput>, updates: Vec<Update>) -> Self {
        Self {
            output_tx,
            state: State::Ready,
            updates,
            is_updates_list_open: false,
        }
    }

    pub fn update(&mut self, message: UpdateMenuMessage) -> iced::Command<UpdateMenuMessage> {
        match message {
        UpdateMenuMessage::UpdatesCheckCompleted(updates) => {
                self.state = State::Ready;

                let _ = self.output_tx.send(MenuOutput::MessageFromUpdates(
                    UpdateMenuOutput::UpdatesCheckCompleted(updates.clone()),
                ));
                self.updates = updates;
                iced::Command::none()
            }
            UpdateMenuMessage::ToggleUpdatesList => {
                self.is_updates_list_open = !self.is_updates_list_open;
                iced::Command::none()
            }
            UpdateMenuMessage::Update => iced::Command::perform(
                async move {
                    tokio::spawn({
                        async move {
                            update().await;
                        }
                    })
                    .await
                },
                {
                    let output_tx = self.output_tx.clone();
                    move |_| {
                        let _ = output_tx.send(MenuOutput::MessageFromUpdates(
                            UpdateMenuOutput::UpdateFinished,
                        ));
                        UpdateMenuMessage::UpdateFinished
                    }
                },
            ),
            UpdateMenuMessage::CheckNow | UpdateMenuMessage::UpdateFinished => {
                self.state = State::Checking;
                let _ = self.output_tx.send(MenuOutput::MessageFromUpdates(
                    UpdateMenuOutput::UpdatesCheckInit,
                ));
                iced::Command::perform(
                    async move { check_update_now().await },
                    UpdateMenuMessage::UpdatesCheckCompleted,
                )
            }
        }
    }

    pub fn view(&self) -> Element<UpdateMenuMessage> {
        column!(
            if self.updates.is_empty() {
                std::convert::Into::<Element<'_, _, _>>::into(
                    container(text("Up to date ;)")).padding([8, 8]),
                )
            } else {
                let mut elements = column!(button(row!(
                    text(format!("{} Updates available", self.updates.len()))
                        .width(iced::Length::Fill),
                    icon(if self.is_updates_list_open {
                        Icons::MenuClosed
                    } else {
                        Icons::MenuOpen
                    })
                ))
                .style(iced::theme::Button::custom(GhostButtonStyle))
                .padding([8, 8])
                .on_press(UpdateMenuMessage::ToggleUpdatesList)
                .width(Length::Fill),);

                if self.is_updates_list_open {
                    elements = elements.push(
                        container(scrollable(
                            Column::with_children(
                                self.updates
                                    .iter()
                                    .map(|update| {
                                        column!(
                                            text(update.package.clone())
                                                .size(10)
                                                .width(iced::Length::Fill),
                                            text(format!(
                                                "{} -> {}",
                                                {
                                                    let mut res = update.from.clone();
                                                    res.truncate(18);

                                                    res
                                                },
                                                {
                                                    let mut res = update.to.clone();
                                                    res.truncate(18);

                                                    res
                                                },
                                            ))
                                            .width(iced::Length::Fill)
                                            .horizontal_alignment(
                                                iced::alignment::Horizontal::Right
                                            )
                                            .size(10)
                                        )
                                        .into()
                                    })
                                    .collect::<Vec<Element<'_, _, _>>>(),
                            )
                            .spacing(4),
                        ))
                        .padding([8, 0])
                        .max_height(300),
                    );
                }
                elements.into()
            },
            horizontal_rule(1).width(Length::Fill),
            button("Update")
                .style(iced::theme::Button::custom(GhostButtonStyle))
                .padding([8, 8])
                .on_press(UpdateMenuMessage::Update)
                .width(Length::Fill),
            button({
                let mut content = row!(text("Check now").width(Length::Fill),);

                if self.state == State::Checking {
                    content = content.push(icon(Icons::Refresh));
                }

                content
            })
            .style(iced::theme::Button::custom(GhostButtonStyle))
            .padding([8, 8])
            .on_press(UpdateMenuMessage::CheckNow)
            .width(Length::Fill),
        )
        .spacing(4)
        .padding(16)
        .max_width(250)
        .into()
    }
}
