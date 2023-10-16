use crate::{
    bar::{MenuAction, MenuType},
    gtk4_wrapper::{
        container, label, scrolled_window, separator, spawn, Align, Component, Orientation,
        PolicyType, TextAlign,
    },
};
use gtk4::Widget;
use leptos::{create_memo, create_signal, ReadSignal, SignalGet, SignalSet};
use serde::Deserialize;
use std::{process::Stdio, rc::Rc, time::Duration};
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

pub fn updates(toggle_menu: Rc<dyn Fn(MenuType, MenuAction)>) -> Widget {
    let (updates, set_updates) = create_signal::<Vec<Update>>(vec![]);

    spawn(async move {
        loop {
            set_updates.set(check_update_now().await);
            sleep(Duration::from_secs(600)).await;
        }
    });

    let update_not_present = create_memo(move |_| updates.get().is_empty());
    let update_present = create_memo(move |_| !updates.get().is_empty());
    let updates_number = create_memo(move |_| updates.get().len().to_string());

    container()
        .class(vec!["header-button", "icon-position-fix"])
        .spacing(8)
        .vexpand(false)
        .valign(Align::Center)
        .on_click(move || {
            toggle_menu(
                MenuType::Updates,
                MenuAction::Open(Box::new({
                    let toggle_menu = toggle_menu.clone();
                    move || {
                        (
                            update_menu(
                                updates,
                                move || {
                                    spawn(async move {
                                        set_updates.set(vec![]);
                                        set_updates.set(check_update_now().await);
                                    });
                                },
                                {
                                    let toggle_menu = toggle_menu.clone();
                                    move || {
                                        tokio::spawn(async move {
                                            update().await;
                                        });
                                        toggle_menu(MenuType::Updates, MenuAction::Close);
                                    }
                                },
                            ),
                            Align::Start,
                        )
                    }
                })),
            )
        })
        .children(vec![
            label()
                .text("󰗠")
                .text_halign(TextAlign::Center)
                .text_valign(TextAlign::Center)
                .visible(update_not_present)
                .into(),
            label()
                .text("󰳛")
                .text_halign(TextAlign::Center)
                .text_valign(TextAlign::Center)
                .visible(update_present)
                .into(),
            label().text(updates_number).visible(update_present).into(),
        ])
        .into()
}

fn update_menu(
    updates: ReadSignal<Vec<Update>>,
    check_updates: impl Fn() + 'static,
    update: impl Fn() + 'static,
) -> Widget {
    let (menu_open, set_menu_open) = create_signal(false);
    let update_present = create_memo(move |_| !updates.get().is_empty());

    let scroll_size = create_memo(move |_| {
        if updates.get().len() < 10 {
            (-1, updates.get().len() as i32 * 16)
        } else {
            (-1, 320)
        }
    });
    let number_of_updates = create_memo(move |_| {
        if updates.get().is_empty() {
            "Up to date ;)".to_string()
        } else {
            format!("{} updates availabe", updates.get().len())
        }
    });

    let menu_expander_icon =
        create_memo(move |_| if menu_open.get() { "" } else { "" }.to_string());

    let updates_list = create_memo(move |_| {
        updates
            .get()
            .iter()
            .map(|update| {
                container()
                    .orientation(Orientation::Horizontal)
                    .spacing(4)
                    .children(vec![
                        label()
                            .hexpand(true)
                            .class(vec!["small-text"])
                            .text_halign(TextAlign::Start)
                            .text(update.package.clone())
                            .into(),
                        label()
                            .class(vec!["small-text"])
                            .text_halign(TextAlign::End)
                            .text(format!("{} -> {}", update.from, update.to))
                            .into(),
                    ])
                    .into()
            })
            .collect::<Vec<Widget>>()
    });

    container()
        .orientation(Orientation::Vertical)
        .hexpand(true)
        .class(vec!["updates-menu"])
        .spacing(4)
        .children(vec![
            container()
                .class(vec!["menu-voice"])
                .hexpand(true)
                .on_click(move || {
                    set_menu_open.set(!menu_open.get());
                })
                .spacing(16)
                .children(vec![
                    label()
                        .hexpand(true)
                        .halign(Align::Start)
                        .text(number_of_updates)
                        .into(),
                    label()
                        .text(menu_expander_icon)
                        .visible(update_present)
                        .into(),
                ])
                .into(),
            scrolled_window()
                .size(scroll_size)
                .child(Into::<Widget>::into(
                    container()
                        .class(vec!["updates-list"])
                        .orientation(Orientation::Vertical)
                        .spacing(4)
                        .children(updates_list),
                ))
                .hscrollbar_policy(PolicyType::Never)
                .visible(menu_open)
                .into(),
            separator().orientation(Orientation::Horizontal).into(),
            label()
                .class(vec!["menu-voice"])
                .hexpand(true)
                .on_click(update)
                .text_halign(TextAlign::Start)
                .text("Update")
                .into(),
            label()
                .class(vec!["menu-voice"])
                .hexpand(true)
                .on_click(check_updates)
                .text_halign(TextAlign::Start)
                .text("Check now!")
                .into(),
        ])
        .into()
}
