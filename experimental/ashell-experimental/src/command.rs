//! The commands a configuration names, run with the shell.

/// Runs `command` with the shell, off the UI thread.
pub fn run(command: String) {
    std::thread::spawn(move || {
        match std::process::Command::new("sh")
            .arg("-c")
            .arg(&command)
            .status()
        {
            Ok(status) if !status.success() => log::warn!("`{command}` ended with {status}"),
            Ok(_) => {}
            Err(err) => log::warn!("`{command}` did not start: {err}"),
        }
    });
}
