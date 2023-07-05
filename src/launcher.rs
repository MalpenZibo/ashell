use std::{
    process::Command,
    thread::{self, sleep},
    time::Duration,
};

use futures_signals::signal::Mutable;
use serde::Deserialize;
use serde_json::json;

pub fn launch_rofi() {
    Command::new("bash")
        .arg("-c")
        .arg("~/.config/rofi/launcher.sh")
        .output()
        .expect("Failed to execute command.");
}

#[derive(Deserialize, Debug, Clone)]
pub struct Update {
    pub package: String,
    pub from: String,
    pub to: String,
}

pub fn check_updates(updates: Mutable<Vec<Update>>) {
    thread::spawn(move || loop {
        {
            let check_update_cmd = Command::new("bash")
                .arg("-c")
                .arg("~/.config/scripts/updates check")
                .output()
                .expect("Failed to execute command.");

            let new_updates = String::from_utf8_lossy(&check_update_cmd.stdout);
            let new_updates = serde_json::from_str::<Vec<Update>>(&new_updates).unwrap();

            updates.replace(new_updates);
        }

        sleep(Duration::from_secs(600));
    });
}
