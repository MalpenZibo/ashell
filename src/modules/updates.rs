use crate::{
    components::icons::{icon, Icons},
    config::{Orientation, UpdatesModuleConfig},
    menu::{Menu, MenuType},
    style::{GhostButtonStyle, HeaderButtonStyle},
};
use iced::{
    alignment::Horizontal,
    subscription,
    theme::{self, Button},
    widget::{button, column, container, horizontal_rule, row, scrollable, text, Column, Row},
    Alignment, Command, Element, Length, Subscription,
};
use log::error;
use serde::Deserialize;
use std::{process::Stdio, time::Duration};
use tokio::time::sleep;

#[derive(Deserialize, Debug, Clone)]
pub struct Update {
    pub package: String,
    pub from: String,
    pub to: String,
}

async fn check_update_now(check_cmd: &str) -> Vec<Update> {
    let check_update_cmd = tokio::process::Command::new("bash")
        .arg("-c")
        .arg(check_cmd)
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
            error!("Error: {:?}", e);
            vec![]
        }
    }
}

async fn update(update_cmd: &str) {
    let _ = tokio::process::Command::new("bash")
        .arg("-c")
        .arg(update_cmd)
        .output()
        .await;
}

#[derive(Debug, Clone)]
pub enum Message {
    ToggleMenu,
    UpdatesCheckCompleted(Vec<Update>),
    UpdateFinished,
    ToggleUpdatesList,
    CheckNow,
    Update,
}

#[derive(Debug, Clone, Eq, PartialEq)]
enum State {
    Checking,
    Ready,
}
pub struct Updates {
    state: State,
    pub updates: Vec<Update>,
    is_updates_list_open: bool,
}

impl Updates {
    pub fn new() -> Self {
        Self {
            state: State::Checking,
            updates: vec![],
            is_updates_list_open: false,
        }
    }

    pub fn update(
        &mut self,
        message: Message,
        config: &UpdatesModuleConfig,
        menu: &mut Menu<crate::app::Message>,
    ) -> Command<crate::app::Message> {
        match message {
            Message::UpdatesCheckCompleted(updates) => {
                self.updates = updates;
                self.state = State::Ready;

                Command::none()
            }
            Message::ToggleMenu => {
                self.is_updates_list_open = false;
                menu.toggle(MenuType::Updates)
            }
            Message::UpdateFinished => {
                self.updates.clear();
                self.state = State::Ready;

                Command::none()
            }
            Message::ToggleUpdatesList => {
                self.is_updates_list_open = !self.is_updates_list_open;

                Command::none()
            }
            Message::CheckNow => {
                self.state = State::Checking;
                let check_command = config.check_cmd.clone();
                Command::perform(
                    async move { check_update_now(&check_command).await },
                    move |updates| {
                        crate::app::Message::Updates(Message::UpdatesCheckCompleted(updates))
                    },
                )
            }
            Message::Update => {
                let update_command = config.update_cmd.clone();
                let mut cmds = vec![Command::perform(
                    async move {
                        tokio::spawn({
                            async move {
                                update(&update_command).await;
                            }
                        })
                        .await
                    },
                    move |_| crate::app::Message::Updates(Message::UpdateFinished),
                )];

                cmds.push(menu.close_if(MenuType::Updates));

                Command::batch(cmds)
            }
        }
    }

    pub fn view(&self, orientation: Orientation) -> Element<Message> {
        let icon = Some(
            icon(match self.state {
                State::Checking => Icons::Refresh,
                State::Ready if self.updates.is_empty() => Icons::NoUpdatesAvailable,
                _ => Icons::UpdatesAvailable,
            })
            .into(),
        );

        let label = (!self.updates.is_empty()).then(|| text(self.updates.len()).into());

        let content = vec![icon, label].into_iter().flatten().collect::<Vec<_>>();
        let content: Element<Message> = match orientation {
            Orientation::Horizontal => Row::with_children(content)
                .align_items(Alignment::Center)
                .spacing(4)
                .into(),
            Orientation::Vertical => Column::with_children(content)
                .align_items(Alignment::Center)
                .spacing(4)
                .into(),
        };

        button(content)
            .padding([2, 7])
            .style(theme::Button::custom(HeaderButtonStyle::Full))
            .on_press(Message::ToggleMenu)
            .into()
    }

    pub fn menu_view(&self) -> Element<Message> {
        column!(
            if self.updates.is_empty() {
                std::convert::Into::<Element<'_, _, _>>::into(
                    container(text("Up to date ;)")).padding([8, 8]),
                )
            } else {
                let mut elements = column!(button(row!(
                    text(format!("{} Updates available", self.updates.len())).width(Length::Fill),
                    icon(if self.is_updates_list_open {
                        Icons::MenuClosed
                    } else {
                        Icons::MenuOpen
                    })
                ))
                .style(Button::custom(GhostButtonStyle))
                .padding([8, 8])
                .on_press(Message::ToggleUpdatesList)
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
                                                .width(Length::Fill),
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
                                            .width(Length::Fill)
                                            .horizontal_alignment(Horizontal::Right)
                                            .size(10)
                                        )
                                        .into()
                                    })
                                    .collect::<Vec<Element<'_, _, _>>>(),
                            )
                            .padding([0, 16, 0, 0])
                            .spacing(4),
                        ))
                        .padding([8, 0])
                        .max_height(300),
                    );
                }
                elements.into()
            },
            horizontal_rule(1),
            button("Update")
                .style(Button::custom(GhostButtonStyle))
                .padding([8, 8])
                .on_press(Message::Update)
                .width(Length::Fill),
            button({
                let mut content = row!(text("Check now").width(Length::Fill),);

                if self.state == State::Checking {
                    content = content.push(icon(Icons::Refresh));
                }

                content
            })
            .style(Button::custom(GhostButtonStyle))
            .padding([8, 8])
            .on_press(Message::CheckNow)
            .width(Length::Fill),
        )
        .spacing(4)
        .padding(16)
        .width(250)
        .into()
    }

    pub fn subscription(&self, config: &UpdatesModuleConfig) -> Subscription<Message> {
        let check_cmd = config.check_cmd.clone();
        subscription::channel("update-checker", 10, |mut output| async move {
            loop {
                let updates = check_update_now(&check_cmd).await;

                let _ = output.try_send(Message::UpdatesCheckCompleted(updates));

                sleep(Duration::from_secs(10)).await;
            }
        })
    }
}
