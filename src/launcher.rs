use std::process::Command;

pub fn launch_rofi() {
    Command::new("bash")
        .arg("-c")
        .arg("~/.config/rofi/launcher.sh")
        .output()
        .expect("Failed to execute command.");
}
