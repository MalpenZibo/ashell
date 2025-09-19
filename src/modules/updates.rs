use crate::{
    app::{self},
    components::icons::{Icons, icon},
    config::UpdatesModuleConfig,
    menu::MenuType,
    outputs::Outputs,
    style::ghost_button_style,
};
use iced::{
    Alignment, Element, Length, Padding, Subscription, Task,
    alignment::Horizontal,
    stream::channel,
    widget::{Column, button, column, container, horizontal_rule, row, scrollable, text},
    window::Id,
};
use log::error;
use serde::Deserialize;
use std::{any::TypeId, convert, process::Stdio, time::Duration};
use tokio::{process, spawn, time::sleep};

use super::{Module, OnModulePress};

#[derive(Deserialize, Debug, Clone)]
pub struct Update {
    pub package: String,
    pub from: String,
    pub to: String,
}

async fn check_update_now(check_cmd: &str) -> Vec<Update> {
    let check_update_cmd = process::Command::new("bash")
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
            error!("Error: {e:?}");
            vec![]
        }
    }
}

async fn update(update_cmd: &str) {
    let _ = process::Command::new("bash")
        .arg("-c")
        .arg(update_cmd)
        .output()
        .await;
}

#[derive(Debug, Clone)]
pub enum Message {
    UpdatesCheckCompleted(Vec<Update>),
    UpdateFinished,
    ToggleUpdatesList,
    CheckNow,
    Update(Id),
}

#[derive(Debug, Default, Clone, Eq, PartialEq)]
enum State {
    #[default]
    Checking,
    Ready,
}

#[derive(Debug, Default, Clone)]
pub struct Updates {
    state: State,
    pub updates: Vec<Update>,
    pub is_updates_list_open: bool,
}

impl Updates {
    pub fn update(
        &mut self,
        message: Message,
        config: &UpdatesModuleConfig,
        outputs: &mut Outputs,
        main_config: &crate::config::Config,
    ) -> Task<crate::app::Message> {
        match message {
            Message::UpdatesCheckCompleted(updates) => {
                self.updates = updates;
                self.state = State::Ready;

                Task::none()
            }
            Message::UpdateFinished => {
                self.updates.clear();
                self.state = State::Ready;

                Task::none()
            }
            Message::ToggleUpdatesList => {
                self.is_updates_list_open = !self.is_updates_list_open;

                Task::none()
            }
            Message::CheckNow => {
                self.state = State::Checking;
                let check_command = config.check_cmd.clone();
                Task::perform(
                    async move { check_update_now(&check_command).await },
                    move |updates| app::Message::Updates(Message::UpdatesCheckCompleted(updates)),
                )
            }
            Message::Update(id) => {
                let update_command = config.update_cmd.clone();
                let mut cmds = vec![Task::perform(
                    async move {
                        spawn({
                            async move {
                                update(&update_command).await;
                            }
                        })
                        .await
                    },
                    move |_| app::Message::Updates(Message::UpdateFinished),
                )];

                cmds.push(outputs.close_menu_if(id, MenuType::Updates, main_config));

                Task::batch(cmds)
            }
        }
    }

    pub fn menu_view(&self, id: Id, opacity: f32) -> Element<Message> {
        column!(
            if self.updates.is_empty() {
                convert::Into::<Element<'_, _, _>>::into(
                    container(text("Up to date ;)")).padding([8, 8]),
                )
            } else {
                let mut elements = column!(
                    button(row!(
                        text(format!("{} Updates available", self.updates.len()))
                            .width(Length::Fill),
                        icon(if self.is_updates_list_open {
                            Icons::MenuClosed
                        } else {
                            Icons::MenuOpen
                        })
                    ))
                    .style(ghost_button_style(opacity))
                    .padding([8, 8])
                    .on_press(Message::ToggleUpdatesList)
                    .width(Length::Fill),
                );

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
                                            .align_x(Horizontal::Right)
                                            .size(10)
                                        )
                                        .into()
                                    })
                                    .collect::<Vec<Element<'_, _, _>>>(),
                            )
                            .padding(Padding::ZERO.right(16))
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
                .style(ghost_button_style(opacity))
                .padding([8, 8])
                .on_press(Message::Update(id))
                .width(Length::Fill),
            button({
                let mut content = row!(text("Check now").width(Length::Fill),);

                if self.state == State::Checking {
                    content = content.push(icon(Icons::Refresh));
                }

                content
            })
            .style(ghost_button_style(opacity))
            .padding([8, 8])
            .on_press(Message::CheckNow)
            .width(Length::Fill),
        )
        .spacing(4)
        .into()
    }
}

impl Module for Updates {
    type ViewData<'a> = &'a Option<UpdatesModuleConfig>;
    type SubscriptionData<'a> = &'a UpdatesModuleConfig;

    fn view(
        &self,
        config: Self::ViewData<'_>,
    ) -> Option<(Element<app::Message>, Option<OnModulePress>)> {
        if config.is_some() {
            let mut content = row!(container(icon(match self.state {
                State::Checking => Icons::Refresh,
                State::Ready if self.updates.is_empty() => Icons::NoUpdatesAvailable,
                _ => Icons::UpdatesAvailable,
            })))
            .align_y(Alignment::Center)
            .spacing(4);

            if !self.updates.is_empty() {
                content = content.push(text(self.updates.len()));
            }

            Some((
                content.into(),
                Some(OnModulePress::ToggleMenu(MenuType::Updates)),
            ))
        } else {
            None
        }
    }

    fn subscription(
        &self,
        config: Self::SubscriptionData<'_>,
    ) -> Option<Subscription<app::Message>> {
        let check_cmd = config.check_cmd.clone();
        let id = TypeId::of::<Self>();

        Some(
            Subscription::run_with_id(
                id,
                channel(10, async move |mut output| {
                    loop {
                        let updates = check_update_now(&check_cmd).await;

                        let _ = output.try_send(Message::UpdatesCheckCompleted(updates));

                        sleep(Duration::from_secs(3600)).await;
                    }
                }),
            )
            .map(app::Message::Updates),
        )
    }
}
