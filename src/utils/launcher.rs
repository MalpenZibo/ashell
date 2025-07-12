use std::process::Command;

pub fn execute_command(command: String) {
    tokio::spawn(async move {
        let _ = Command::new("bash")
            .arg("-c")
            .arg(&command)
            .spawn()
            .unwrap_or_else(|_| panic!("Failed to execute command {}", &command))
            .wait();
    });
}

pub fn suspend(cmd: String) {
    tokio::spawn(async move {
        let _ = Command::new("bash")
            .arg("-c")
            .arg(cmd)
            .spawn()
            .expect("Failed to execute command.")
            .wait();
    });
}

pub fn shutdown(cmd: String) {
    tokio::spawn(async move {
        let _ = Command::new("bash")
            .arg("-c")
            .arg(cmd)
            .spawn()
            .expect("Failed to execute command.")
            .wait();
    });
}

pub fn reboot(cmd: String) {
    tokio::spawn(async move {
        let _ = Command::new("bash")
            .arg("-c")
            .arg(cmd)
            .spawn()
            .expect("Failed to execute command.")
            .wait();
    });
}

pub fn logout(cmd: String) {
    tokio::spawn(async move {
        let _ = Command::new("bash")
            .arg("-c")
            .arg(cmd)
            .spawn()
            .expect("Failed to execute command.")
            .wait();
    });
}
