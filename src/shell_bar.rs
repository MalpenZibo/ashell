use std::{thread::sleep, time::Duration};

use crate::reactive_gtk::{spawner::spawn, Button, Orientation};
use futures_signals::signal::{Mutable, SignalExt};

use crate::{
    launcher::{check_updates, launch_rofi, Update},
    reactive_gtk::{Align, Box, Component, Label, Node, Overlay},
};

fn application_button() -> Node {
    Box::default()
        .class(&["rounded-m", "bg", "interactive"])
        .on_click(launch_rofi)
        .children(vec![Label::default().class(&["ph-2"]).text("󱗼").into()])
        .into()
}

fn update_button() -> Node {
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

pub fn create_shell_bar() -> Node {
    Box::default()
        .class(&["text-bold", "ph-1", "pv-1"])
        .spacing(4)
        .children(vec![application_button(), update_button()])
        .into()
}

// pub fn create_shell_bar() -> Node {
//     Box::default()
//         .class(&["text-bold", "ph-1", "pv-1"])
//         .spacing(4)
//         .children(vec![test2(), test2()])
//         .into()
// }

fn test2() -> Node {
    let test = Mutable::new(0);
    let test_button = test.clone();

    Box::default()
        .children(vec![
            Button::default()
                .on_click(move || {
                    test_button.replace_with(|t| *t + 1);
                })
                .child(Label::default().text("text"))
                .into(),
            Label::default()
                .class(&["mh-4"])
                .text_signal(test.signal_ref(|t| t.to_string()))
                .into(),
        ])
        .into()
}
