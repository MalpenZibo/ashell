use std::{process::Stdio, time::Duration};

use gtk4::Widget;
use leptos::{create_memo, create_signal, SignalGet, SignalSet};
use serde::Deserialize;
use tokio::{io::AsyncReadExt, join, process::Command, time::sleep};

use crate::gtk4_wrapper::{label, overlay, spawn, Align, Component};

#[derive(Deserialize, Debug, Clone)]
pub struct Update {
    pub package: String,
    pub from: String,
    pub to: String,
}

async fn check_update_now() -> Vec<Update> {
    println!("Checking for updates");

    let mut check_update_cmd = Command::new("bash")
        .arg("-c")
        .arg("checkupdates; paru -Qua ")
        .stdout(Stdio::piped())
        .output()
        .await
        .expect("Failed to execute command.");

    println!("Updates checked");

    let new_updates = String::from_utf8_lossy(&check_update_cmd.stdout);
    println!("Updates: {}", new_updates);
    let splitted = new_updates.split("\n");
    let mut new_updates: Vec<Update> = Vec::new();
    for update in splitted {
        if update.is_empty() {
            continue;
        }

        let data = update.split(" ").collect::<Vec<&str>>();
        if data.len() < 4 {
            continue;
        }
        new_updates.push(Update {
            package: data[0].to_string(),
            from: data[1].to_string(),
            to: data[3].to_string(),
        });
    }

    println!("Updates parsed {:?}", new_updates);

    new_updates
}

fn update() {
    tokio::spawn(async move {
        Command::new("bash")
            .arg("-c")
            .arg("alacritty -e bash -c \"paru; flatpak update; echo Done - Press enter to exit; read\" &")
            .spawn()
            .expect("Failed to execute command.");
    });
}

pub fn updates() -> Widget {
    let (updates, set_updates) = create_signal::<Vec<Update>>(vec![]);

    spawn(async move {
        loop {
            set_updates.set(check_update_now().await);
            sleep(Duration::from_secs(600)).await;
        }
    });

    let update_present = create_memo(move |_| !updates.get().is_empty());

    overlay()
        .class(vec!["header-button", "arch-icon"])
        .vexpand(false)
        .valign(Align::Center)
        .children(vec![
            label().text("󰣇").into(),
            label().text("").visible(update_present).into(),
        ])
        .into()
}
