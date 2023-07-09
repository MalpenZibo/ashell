use std::{process::Command, time::Duration};

use futures_signals::signal::Mutable;
use serde::Deserialize;

use crate::{
    reactive_gtk::{Align, Box, Component, Label, Node, Overlay},
    utils::poll,
};

#[derive(Deserialize, Debug, Clone)]
pub struct Update {
    pub package: String,
    pub from: String,
    pub to: String,
}

fn check_updates(updates: Mutable<Vec<Update>>) {
    poll(
        move || {
            let check_update_cmd = Command::new("bash")
                .arg("-c")
                .arg("~/.config/scripts/updates check")
                .output()
                .expect("Failed to execute command.");

            let new_updates = String::from_utf8_lossy(&check_update_cmd.stdout);
            let new_updates = serde_json::from_str::<Vec<Update>>(&new_updates).unwrap();

            updates.replace(new_updates);
        },
        Duration::from_secs(600),
    );
}

pub fn update_button() -> Node {
    let updates: Mutable<Vec<Update>> = Mutable::new(Vec::new());
    check_updates(updates.clone());

    Box::default()
        .class(&["rounded-m", "bg", "ph-2", "interactive"])
        .children(vec![
            Overlay::default()
                .size((10, -1))
                .children(vec![
                    Label::default().text("󰣇").halign(Align::Center).into(),
                    Label::default()
                        .text("")
                        .class(&["bg", "rounded-m", "text-xxs", "ml-1", "mb-1"])
                        .halign(Align::Start)
                        .valign(Align::End)
                        .visible_signal(updates.signal_ref(|updates| !updates.is_empty()))
                        .into(),
                ])
                .into(),
            Label::default()
                .class(&["pl-2"])
                .text_signal(updates.signal_ref(|u| u.len().to_string()))
                .visible_signal(updates.signal_ref(|updates| !updates.is_empty()))
                .into(),
        ])
        .into()
}
