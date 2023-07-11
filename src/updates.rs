use std::{process::Command, time::Duration};

use futures_signals::signal::Mutable;
use gtk::traits::GtkWindowExt;
use serde::Deserialize;

use crate::{
    reactive_gtk::{Align, Box, Component, Context, Label, Node, Overlay, Surface},
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

pub fn update_button(ctx: Context) -> Node {
    let updates: Mutable<Vec<Update>> = Mutable::new(Vec::new());
    check_updates(updates.clone());

    Box::default()
        .class(&["rounded-m", "bg", "ph-2", "interactive"])
        .on_click(move || {
            ctx.open_surface(
                Surface::layer(false, (true, true, true, true), None),
                update_menu,
            )
        })
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

pub fn update_menu(ctx: Context) -> Node {
    Overlay::default()
        .children(vec![
            Box::default()
                .hexpand(true)
                .vexpand(true)
                .class(&["test"])
                .on_click(move || ctx.window.close())
                .into(),
            Box::default()
                .class(&["m-1", "p-5", "rounded-m", "bg", "border"])
                .hexpand(false)
                .vexpand(false)
                .halign(Align::Start)
                .valign(Align::Start)
                .children(vec![Label::default().text("Hello, world!").into()])
                .into(),
        ])
        .into()
}
