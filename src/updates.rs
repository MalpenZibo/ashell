use std::{fmt::Debug, process::Command, time::Duration};

use futures_signals::signal::Mutable;

use gtk::{
    traits::{GtkWindowExt, WidgetExt},
    ApplicationWindow,
};
use serde::Deserialize;

use crate::{
    reactive_gtk::{
        Align, Box, Component, Context, Label, Node, Orientation, Overlay, PolicyType,
        ScrolledWindow, Separator, Surface, XAlign,
    },
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
    let menu_open: Mutable<Option<(ApplicationWindow, Node)>> = Mutable::new(None);
    let updates: Mutable<Vec<Update>> = Mutable::new(Vec::new());
    let updates1 = updates.clone();

    check_updates(updates.clone());

    Box::default()
        .class(&["rounded-m", "bg", "ph-2", "interactive"])
        .on_click(move || {
            menu_open.replace_with(|menu| {
                if let Some((win, _)) = menu {
                    win.close();
                    None
                } else {
                    let node = ctx.open_surface(
                        Surface::layer(false, (true, true, true, true), None),
                        update_menu(menu_open.clone(), updates.clone()),
                    );
                    Some((node.0, node.1))
                }
            });
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
                        .visible_signal(updates1.signal_ref(|updates| !updates.is_empty()))
                        .into(),
                ])
                .into(),
            Label::default()
                .class(&["pl-2"])
                .text_signal(updates1.signal_ref(|u| u.len().to_string()))
                .visible_signal(updates1.signal_ref(|updates| !updates.is_empty()))
                .into(),
        ])
        .into()
}

pub fn update_menu(
    menu_open: Mutable<Option<(ApplicationWindow, Node)>>,
    updates: Mutable<Vec<Update>>,
) -> impl FnOnce(Context) -> Node {
    move |ctx| {
        let update_list_open = Mutable::new(false);
        let update_list_open1 = update_list_open.clone();

        Overlay::default()
            .children(vec![
                Box::default()
                    .hexpand(true)
                    .vexpand(true)
                    .on_click(move || {
                        ctx.window.hide();
                        menu_open.replace(None);
                    })
                    .into(),
                Box::default()
                    .class(&["m-1", "p-5", "rounded-m", "bg", "border"])
                    .hexpand(false)
                    .vexpand(false)
                    .halign(Align::Start)
                    .valign(Align::Start)
                    .orientation(Orientation::Vertical)
                    .spacing(8)
                    .children(vec![
                        Box::default()
                            .class_signal(updates.signal_ref(|updates| {
                                if !updates.is_empty() {
                                    vec!["interactive", "rounded-s", "pv-2", "ph-4"]
                                } else {
                                    vec!["pv-2", "ph-4"]
                                }
                            }))
                            .on_click(move || {
                                update_list_open1.replace_with(|f| !*f);
                            })
                            .spacing(8)
                            .children(vec![
                                Label::default()
                                    .halign(Align::Start)
                                    .hexpand(true)
                                    .text_signal(updates.signal_ref(|updates| {
                                        if !updates.is_empty() {
                                            format!("{} updates available", updates.len())
                                        } else {
                                            "Up to date!".to_string()
                                        }
                                    }))
                                    .into(),
                                Label::default()
                                    .halign(Align::End)
                                    .text_signal(update_list_open.signal_ref(|f| {
                                        if *f {
                                            ""
                                        } else {
                                            ""
                                        }
                                    }))
                                    .visible_signal(
                                        updates.signal_ref(|updates| !updates.is_empty()),
                                    )
                                    .halign(Align::End)
                                    .into(),
                            ])
                            .into(),
                        ScrolledWindow::default()
                            .visible_signal(update_list_open.signal())
                            .hscrollbar_policy(PolicyType::Never)
                            .size_signal(updates.signal_ref(|updates| {
                                if updates.len() < 10 {
                                    (-1, updates.len() as i32 * 32)
                                } else {
                                    (-1, 320)
                                }
                            }))
                            .child(
                                Box::default()
                                    .orientation(Orientation::Vertical)
                                    .children_signal(updates.signal_ref(|updates| {
                                        updates
                                            .iter()
                                            .map(|update| {
                                                Box::default()
                                                    .class(&["pv-2", "ph-6"])
                                                    .spacing(8)
                                                    .children(vec![
                                                        Label::default()
                                                            .class(&["text-s"])
                                                            .text(&update.package)
                                                            .halign(Align::Start)
                                                            .hexpand(true)
                                                            .into(),
                                                        Label::default()
                                                            .class(&["text-s"])
                                                            .text(format!(
                                                                "{} - {}",
                                                                update.from, update.to
                                                            ))
                                                            .halign(Align::End)
                                                            .into(),
                                                    ])
                                                    .into()
                                            })
                                            .collect::<Vec<Node>>()
                                    })),
                            )
                            .into(),
                        Separator::default()
                            .orientation(Orientation::Horizontal)
                            .into(),
                        Label::default()
                            .text("Update")
                            .xalign(XAlign::Left)
                            .class(&["pv-2", "ph-4", "interactive", "rounded-s"])
                            .on_click(move || {
                                println!("Updating...");
                            })
                            .into(),
                        Label::default()
                            .text("Check now")
                            .xalign(XAlign::Left)
                            .class(&["pv-2", "ph-4", "interactive", "rounded-s"])
                            .on_click(move || {
                                println!("Updating...");
                            })
                            .into(),
                    ])
                    .into(),
            ])
            .into()
    }
}
