use crate::{
    components::icons::{StaticIcon, icon},
    config::UpdatesModuleConfig,
    menu::MenuSize,
    theme::AshellTheme,
};
use iced::{
    Alignment, Element, Length, Subscription, Task,
    alignment::Horizontal,
    stream::channel,
    widget::{Column, button, column, container, horizontal_rule, row, scrollable, text},
    window::Id,
};
use log::error;
use serde::Deserialize;
use std::{any::TypeId, convert, process::Stdio, time::Duration};
use tokio::{process, time::sleep};

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
    MenuOpened,
    ToggleUpdatesList,
    CheckNow,
    Update(Id),
}

pub enum Action {
    None,
    CheckForUpdates(Task<Message>),
    CloseMenu(Id, Task<Message>),
}

#[derive(Debug, Default, Clone, Eq, PartialEq)]
enum State {
    #[default]
    Checking,
    Ready,
}

#[derive(Debug, Clone)]
pub struct Updates {
    config: UpdatesModuleConfig,
    state: State,
    updates: Vec<Update>,
    is_updates_list_open: bool,
}

impl Updates {
    pub fn new(config: UpdatesModuleConfig) -> Self {
        Self {
            config,
            state: State::default(),
            updates: Vec::new(),
            is_updates_list_open: false,
        }
    }

    pub fn update(&mut self, message: Message) -> Action {
        match message {
            Message::UpdatesCheckCompleted(updates) => {
                self.updates = updates;
                self.state = State::Ready;

                Action::None
            }
            Message::UpdateFinished => {
                // Re-check updates to verify they were actually applied
                let check_command = self.config.check_cmd.clone();

                Action::CheckForUpdates(Task::perform(
                    async move { check_update_now(&check_command).await },
                    Message::UpdatesCheckCompleted,
                ))
            }
            Message::MenuOpened => {
                self.is_updates_list_open = false;

                Action::None
            }
            Message::ToggleUpdatesList => {
                self.is_updates_list_open = !self.is_updates_list_open;

                Action::None
            }
            Message::CheckNow => {
                self.state = State::Checking;
                let check_command = self.config.check_cmd.clone();

                Action::CheckForUpdates(Task::perform(
                    async move { check_update_now(&check_command).await },
                    Message::UpdatesCheckCompleted,
                ))
            }
            Message::Update(id) => {
                let update_command = self.config.update_cmd.clone();

                Action::CloseMenu(
                    id,
                    Task::perform(
                        async move {
                            update(&update_command).await; // Wait for real completion
                        },
                        move |_| Message::UpdateFinished,
                    ),
                )
            }
        }
    }

    pub fn view(&'_ self, theme: &AshellTheme) -> Element<'_, Message> {
        let mut content = row!(container(icon(match self.state {
            State::Checking => StaticIcon::Refresh,
            State::Ready if self.updates.is_empty() => StaticIcon::NoUpdatesAvailable,
            _ => StaticIcon::UpdatesAvailable,
        })))
        .align_y(Alignment::Center)
        .spacing(theme.space.xxs);

        if !self.updates.is_empty() {
            content = content.push(text(self.updates.len()));
        }

        content.into()
    }

    pub fn menu_view<'a>(&'a self, id: Id, theme: &'a AshellTheme) -> Element<'a, Message> {
        column!(
            if self.updates.is_empty() {
                convert::Into::<Element<'_, _, _>>::into(
                    container(text("Up to date ;)")).padding(theme.space.xs),
                )
            } else {
                let mut elements = column!(
                    button(row!(
                        text(format!("{} Updates available", self.updates.len()))
                            .width(Length::Fill),
                        icon(if self.is_updates_list_open {
                            StaticIcon::MenuClosed
                        } else {
                            StaticIcon::MenuOpen
                        })
                    ))
                    .style(theme.ghost_button_style())
                    .padding(theme.space.xs)
                    .on_press(Message::ToggleUpdatesList)
                    .width(Length::Fill),
                )
                .spacing(theme.space.xs);

                if self.is_updates_list_open {
                    elements = elements.push(
                        container(scrollable(
                            Column::with_children(
                                self.updates
                                    .iter()
                                    .map(|update| {
                                        column!(
                                            text(update.package.clone())
                                                .size(theme.font_size.xs)
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
                                            .size(theme.font_size.xs)
                                        )
                                        .into()
                                    })
                                    .collect::<Vec<Element<'_, _, _>>>(),
                            )
                            .spacing(theme.space.xs)
                            .padding([
                                0,
                                theme.space.md,
                                0,
                                theme.space.xs,
                            ]),
                        ))
                        .max_height(300),
                    );
                }
                elements.into()
            },
            horizontal_rule(1),
            column!(
                button("Update")
                    .style(theme.ghost_button_style())
                    .padding(theme.space.xs)
                    .on_press(Message::Update(id))
                    .width(Length::Fill),
                button({
                    let mut content = row!(text("Check now").width(Length::Fill),);

                    if self.state == State::Checking {
                        content = content.push(icon(StaticIcon::Refresh));
                    }

                    content
                })
                .style(theme.ghost_button_style())
                .padding(theme.space.xs)
                .on_press(Message::CheckNow)
                .width(Length::Fill)
            ),
        )
        .spacing(theme.space.xs)
        .max_width(MenuSize::Small)
        .into()
    }

    pub fn subscription(&self) -> Subscription<Message> {
        let check_cmd = self.config.check_cmd.clone();
        let interval = Duration::from_secs(self.config.interval.max(60));
        let id = TypeId::of::<Self>();

        Subscription::run_with_id(
            (id, check_cmd.clone()),
            channel(10, async move |mut output| {
                loop {
                    let updates = check_update_now(&check_cmd).await;

                    let _ = output.try_send(Message::UpdatesCheckCompleted(updates));

                    sleep(interval).await;
                }
            }),
        )
    }
}
