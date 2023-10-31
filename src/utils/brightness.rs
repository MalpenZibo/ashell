use std::{
    process::{Command, Stdio},
    time::Duration,
};

use futures_signals::signal::Mutable;

use crate::utils::poll;

pub fn brightness_monitor() -> Mutable<f64> {
    let brightness = Mutable::new(0.);

    poll(
        {
            let brighness = brightness.clone();
            move || {
                let command = Command::new("brightnessctl")
                    .args(["-m", "i"])
                    .stdout(Stdio::piped())
                    .spawn()
                    .expect("Failed to execute brightnessctl command");

                let output = command
                    .wait_with_output()
                    .expect("Failed to read brightnessctl command output");
                let output = String::from_utf8_lossy(&output.stdout);

                let value = output.split(',').nth(2);

                let value = value.and_then(|v| v.parse::<f64>().ok()).unwrap_or(0.);

                brighness.replace(value);
            }
        },
        Duration::from_secs(100),
    );

    brightness
}

pub fn set_brightness(value: u32) {
    let command = Command::new("brightnessctl")
        .args(["set", &format!("{}", value), "-q"])
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to execute brightnessctl command");

    command
        .wait_with_output()
        .expect("Failed to read brightnessctl command output");
}
