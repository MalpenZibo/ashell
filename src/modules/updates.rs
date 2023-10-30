use crate::{
    bar::{MenuAction, MenuType},
    nodes,
    reactive_gtk::{
        container, label, scrolled_window, separator, Align, Dynamic, Node, NodeBuilder,
        Orientation, PolicyType, TextAlign,
    },
};
use futures_signals::signal::Mutable;
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

pub fn updates(toggle_menu: Rc<dyn Fn(MenuType, MenuAction)>) -> impl Into<Node> {
    let updates = Mutable::new(vec![]);

    tokio::spawn({
        let updates = updates.clone();
        async move {
            loop {
                // updates.replace(check_update_now().await);
                sleep(Duration::from_secs(600)).await;
            }
        }
    });

    container()
        .class(vec!["bar-item", "interactive", "icon-position-fix"])
        .spacing(8)
        .vexpand(false)
        .valign(Align::Center)
        .on_click({
            let updates = updates.clone();
            move || {
                toggle_menu(
                    MenuType::Updates,
                    MenuAction::Open(Box::new({
                        let toggle_menu = toggle_menu.clone();
                        let updates = updates.clone();
                        move || {
                            (
                                update_menu(toggle_menu.clone(), updates.clone()).into(),
                                Align::Start,
                            )
                        }
                    })),
                )
            }
        })
        .children(nodes![
            label()
                .text("󰗠".to_string())
                .text_halign(TextAlign::Center)
                .text_valign(TextAlign::Center)
                .visible(Dynamic(updates.signal_ref(|updates| !updates.is_empty()))),
            label()
                .text("󰳛".to_string())
                .text_halign(TextAlign::Center)
                .text_valign(TextAlign::Center)
                .visible(Dynamic(updates.signal_ref(|updates| updates.is_empty()))),
            label()
                .text(Dynamic(
                    updates.signal_ref(|updates| updates.len().to_string())
                ))
                .visible(Dynamic(updates.signal_ref(|updates| !updates.is_empty())))
        ])
}

fn update_menu(
    toggle_menu: Rc<dyn Fn(MenuType, MenuAction)>,
    updates: Mutable<Vec<Update>>,
) -> impl Into<Node> {
    let menu_open = Mutable::new(false);
    let update_present = updates.signal_ref(|updates| !updates.is_empty());

    let scroll_size = updates.signal_ref(|updates| {
        if updates.len() < 10 {
            (-1, updates.len() as i32 * 22)
        } else {
            (-1, 320)
        }
    });

    let number_of_updates = updates.signal_ref(|updates| {
        if updates.is_empty() {
            "Up to date ;)".to_string()
        } else {
            format!("{} updates availabe", updates.len())
        }
    });

    let menu_expander_icon =
        menu_open.signal_ref(|menu_open| if *menu_open { "" } else { "" }.to_string());

    let updates_list = updates.signal_ref(|updates| {
        updates
            .iter()
            .map(|update| {
                container()
                    .orientation(Orientation::Horizontal)
                    .spacing(4)
                    .children(nodes![
                        label()
                            .hexpand(true)
                            .class(vec!["small-text"])
                            .text_halign(TextAlign::Start)
                            .text(update.package.clone()),
                        label()
                            .class(vec!["small-text"])
                            .text_halign(TextAlign::End)
                            .text(format!("{} -> {}", update.from, update.to))
                    ])
                    .into()
            })
            .collect::<Vec<Node>>()
    });

    container()
        .orientation(Orientation::Vertical)
        .hexpand(true)
        .class(vec!["updates-menu"])
        .spacing(4)
        .children(nodes![
            container()
                .class(vec!["menu-voice"])
                .hexpand(true)
                .on_click({
                    let menu_open = menu_open.clone();
                    move || {
                        menu_open.replace_with(|menu_open| !*menu_open);
                    }
                })
                .spacing(16)
                .children(nodes![
                    label()
                        .hexpand(true)
                        .halign(Align::Start)
                        .text(Dynamic(number_of_updates)),
                    label()
                        .text(Dynamic(menu_expander_icon))
                        .visible(Dynamic(update_present))
                ]),
            scrolled_window()
                .size(Dynamic(scroll_size))
                .child(Some(
                    container()
                        .class(vec!["updates-list"])
                        .orientation(Orientation::Vertical)
                        .spacing(4)
                        .children(Dynamic(updates_list)),
                ))
                .hscrollbar_policy(PolicyType::Never)
                .visible(Dynamic(menu_open.signal())),
            separator().orientation(Orientation::Horizontal),
            label()
                .class(vec!["menu-voice"])
                .hexpand(true)
                .on_click(move || {
                    tokio::spawn({
                        async move {
                            update().await;
                        }
                    });
                    toggle_menu(MenuType::Updates, MenuAction::Close);
                })
                .text_halign(TextAlign::Start)
                .text("Update".to_string()),
            label()
                .class(vec!["menu-voice"])
                .hexpand(true)
                .on_click({
                    let updates = updates.clone();
                    move || {
                        tokio::spawn({
                            let updates = updates.clone();
                            async move {
                                updates.replace(vec![]);
                                updates.replace(check_update_now().await);
                            }
                        });
                    }
                })
                .text_halign(TextAlign::Start)
                .text("Check now!".to_string())
        ])
}
