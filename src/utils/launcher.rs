use std::process::Command;

use log::error;

fn run_command(cmd: String, context: &'static str) {
    tokio::spawn(async move {
        match Command::new("bash").arg("-c").arg(&cmd).spawn() {
            Ok(mut child) => {
                if let Err(e) = child.wait() {
                    error!("{context} command failed: {e}");
                }
            }
            Err(e) => {
                error!("Failed to execute {context} command: {e}");
            }
        }
    });
}

pub fn execute_command(command: String) {
    run_command(command, "execute");
}

pub fn suspend(cmd: String) {
    run_command(cmd, "suspend");
}

pub fn hibernate(cmd: String) {
    run_command(cmd, "hibernate");
}

pub fn shutdown(cmd: String) {
    run_command(cmd, "shutdown");
}

pub fn reboot(cmd: String) {
    run_command(cmd, "reboot");
}

pub fn logout(cmd: String) {
    run_command(cmd, "logout");
}
