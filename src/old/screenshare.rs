use std::{process::Command, time::Duration};

use futures_signals::signal::Mutable;

use crate::{
    reactive_gtk::{Box, Button, Component, Label, Node},
    utils::poll,
};

static APP_LIST: [&str; 1] = ["wf-recorder"];

fn check_app_running(app_name: &str) -> bool {
    let command = Command::new("pgrep")
        .arg("-x")
        .arg(app_name)
        .output()
        .expect("failed to execute pgrep command");

    let output_str = String::from_utf8_lossy(&command.stdout);

    !output_str.is_empty()
}

pub fn kill(screenshare: Mutable<bool>) {
    for app_name in APP_LIST {
        if check_app_running(app_name) {
            Command::new("killall")
                .arg("-s")
                .arg("SIGINT")
                .arg(app_name)
                .output()
                .expect("failed to execute killall command");
        }
    }

    screenshare.replace(false);
}

pub fn screenshare() -> Node {
    let screenshare = Mutable::new(false);
    let screenshare1 = screenshare.clone();

    poll(
        move || {
            for app_name in APP_LIST {
                if check_app_running(app_name) {
                    screenshare1.replace(true);
                    break;
                }
            }
        },
        Duration::from_secs(5),
    );

    Box::default()
        .class(&["rounded-m", "fg-black", "bg-yellow", "ph-2", "pv-1"])
        .spacing(4)
        .visible_signal(screenshare.signal())
        .children(vec![
            Label::default()
                .class(&["fg-black", "text-m"])
                .text("󱒃")
                .into(),
            Button::default()
                .class(&["rounded-full"])
                .size((20, 20))
                .child(Label::default().text("󱎘"))
                .on_click(move || kill(screenshare.clone()))
                .into(),
        ])
        .into()
}
