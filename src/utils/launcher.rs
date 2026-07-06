use log::error;
use tokio::process::Command;

fn run_command(cmd: String, context: &'static str) {
    tokio::spawn(async move {
        match Command::new("bash").arg("-c").arg(&cmd).spawn() {
            Ok(child) => {
                if let Err(e) = child.wait_with_output().await {
                    error!("{context} command failed: {e}");
                }
            }
            Err(e) => {
                error!("Failed to execute {context} command: {e}");
            }
        }
    });
}

pub fn execute_command(command: &str) {
    run_command(command.to_owned(), "execute");
}

pub fn suspend(cmd: &str) {
    run_command(cmd.to_owned(), "suspend");
}

pub fn hibernate(cmd: &str) {
    run_command(cmd.to_owned(), "hibernate");
}

pub fn shutdown(cmd: &str) {
    run_command(cmd.to_owned(), "shutdown");
}

pub fn reboot(cmd: &str) {
    run_command(cmd.to_owned(), "reboot");
}

pub fn logout(cmd: &str) {
    run_command(cmd.to_owned(), "logout");
}
