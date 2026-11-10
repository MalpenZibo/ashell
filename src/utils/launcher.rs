use crate::config::{DEFAULT_REBOOT_CMD, DEFAULT_SHUTDOWN_CMD, DEFAULT_SUSPEND_CMD};
use log::{error, warn};
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

/// The three power actions with a built-in default command. Custom commands
/// for other actions never get this fallback.
#[derive(Clone, Copy)]
enum PowerAction {
    Shutdown,
    Suspend,
    Reboot,
}

impl PowerAction {
    fn builtin_command(self) -> &'static str {
        match self {
            PowerAction::Shutdown => DEFAULT_SHUTDOWN_CMD,
            PowerAction::Suspend => DEFAULT_SUSPEND_CMD,
            PowerAction::Reboot => DEFAULT_REBOOT_CMD,
        }
    }

    fn fallback_command(self) -> &'static str {
        match self {
            PowerAction::Shutdown => "loginctl poweroff",
            PowerAction::Suspend => "loginctl suspend",
            PowerAction::Reboot => "loginctl reboot",
        }
    }

    fn context(self) -> &'static str {
        match self {
            PowerAction::Shutdown => "shutdown",
            PowerAction::Suspend => "suspend",
            PowerAction::Reboot => "reboot",
        }
    }
}

/// Returns the fallback only while `cmd` is still the built-in default. A
/// command with extra arguments, a wrapper such as `sudo`, or a shell
/// expression is user intent and is left alone.
fn builtin_fallback<'a>(cmd: &str, builtin: &str, fallback: &'a str) -> Option<&'a str> {
    (cmd.trim() == builtin).then_some(fallback)
}

/// Run a built-in power command, retrying the matching `loginctl` command when
/// the executable is missing. Exit status 127 from the shell means "command not
/// found"; any other failure is left as-is, so a permission or inhibitor error
/// never runs a second power mechanism.
fn run_power_command(action: PowerAction, cmd: &str) {
    let context = action.context();
    let builtin = action.builtin_command();
    let Some(fallback) = builtin_fallback(cmd, builtin, action.fallback_command()) else {
        run_command(cmd.to_owned(), context);
        return;
    };

    let cmd = cmd.to_owned();
    tokio::spawn(async move {
        match Command::new("bash").arg("-c").arg(&cmd).output().await {
            Ok(output) if output.status.code() == Some(127) => {
                warn!("{context}: `{builtin}` is unavailable, retrying with `{fallback}`");

                match Command::new("bash").arg("-c").arg(fallback).output().await {
                    Ok(output) if !output.status.success() => {
                        error!("{context} fallback `{fallback}` failed: {}", output.status);
                    }
                    Ok(_) => {}
                    Err(e) => {
                        error!("Failed to execute {context} fallback `{fallback}`: {e}");
                    }
                }
            }
            Ok(output) if !output.status.success() => {
                error!("{context} command failed: {}", output.status);
            }
            Ok(_) => {}
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
    run_power_command(PowerAction::Suspend, cmd);
}

pub fn hibernate(cmd: &str) {
    // There is no built-in hibernate command, so anything set here is custom.
    run_command(cmd.to_owned(), "hibernate");
}

pub fn shutdown(cmd: &str) {
    run_power_command(PowerAction::Shutdown, cmd);
}

pub fn reboot(cmd: &str) {
    run_power_command(PowerAction::Reboot, cmd);
}

pub fn logout(cmd: &str) {
    run_command(cmd.to_owned(), "logout");
}

#[cfg(test)]
mod tests {
    use super::builtin_fallback;
    use crate::config::{DEFAULT_REBOOT_CMD, DEFAULT_SHUTDOWN_CMD, DEFAULT_SUSPEND_CMD};

    #[test]
    fn builtin_defaults_get_a_fallback() {
        for (default, fallback) in [
            (DEFAULT_SHUTDOWN_CMD, "loginctl poweroff"),
            (DEFAULT_SUSPEND_CMD, "loginctl suspend"),
            (DEFAULT_REBOOT_CMD, "loginctl reboot"),
        ] {
            assert_eq!(builtin_fallback(default, default, fallback), Some(fallback));
            assert_eq!(
                builtin_fallback(&format!("  {default}\n"), default, fallback),
                Some(fallback)
            );
        }
    }

    #[test]
    fn custom_power_commands_are_not_rewritten() {
        for cmd in [
            "systemctl --user reboot",
            "sudo systemctl reboot",
            "systemctl reboot --force",
            "systemctl reboot || notify-send \"Failed\"",
            "loginctl reboot",
            "shutdown -h now",
        ] {
            assert_eq!(
                builtin_fallback(cmd, DEFAULT_REBOOT_CMD, "loginctl reboot"),
                None,
                "{cmd} must keep its configured behaviour"
            );
        }
    }
}
