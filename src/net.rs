use std::{
    io::{self, BufRead, BufReader},
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

use serde::{Deserialize, __private::de};
use serde_json::json;

fn process_output_line(line: &str) {
    // Implement your logic to process each line of output here
    println!("Output line: {}", line);
}

#[derive(Deserialize, Debug)]
struct Device {
    device: String,
    r#type: String,
    state: String,
    connection: Option<String>,
}

#[derive(Deserialize, Debug)]
struct Connection {
    name: String,
    uuid: String,
    r#type: String,
}

enum ActiveConnection {
    Ethernet(String),
    Wifi((String, u32)),
}

struct Vpn {
    name: String,
    active: bool,
}

struct NetState {
    active_connection: Option<Connection>,
    wifi: Vec<String>,
    vpn: Vec<Vpn>,
}

fn make_content() {
    let nmcli_d = Command::new("nmcli")
        .arg("d")
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to execute nmcli command");

    let nmcli_connection = Command::new("nmcli")
        .arg("connection")
        .arg("show")
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to execute nmcli command");

    let jc_d = Command::new("jc")
        .arg("--nmcli")
        .stdin(Stdio::from(nmcli_d.stdout.unwrap()))
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to execute jc command");

    let jc_connection = Command::new("jc")
        .arg("--nmcli")
        .stdin(Stdio::from(nmcli_connection.stdout.unwrap()))
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to execute jc command");

    let jc_d_output = jc_d
        .wait_with_output()
        .expect("Failed to read jc command output");

    let jc_connection_output = jc_connection
        .wait_with_output()
        .expect("Failed to read jc command output");
    let jc_d_stdout = String::from_utf8_lossy(&jc_d_output.stdout);
    let jc_connection_stdout = String::from_utf8_lossy(&jc_connection_output.stdout);

    let devices: Vec<Device> = serde_json::from_str(&jc_d_stdout).unwrap();
    let connections: Vec<Device> = serde_json::from_str(&jc_d_stdout).unwrap();

    let ethernet_connected = devices
        .iter()
        .find(|d| d.r#type == "ethernet" && d.state == "connected");
    let wifi_connected = devices
        .iter()
        .find(|d| d.r#type == "wifi" && d.state == "connected");

    let active_connection = match (ethernet_connected, wifi_connected) {
        (Some(ethernet), _) => Some(ActiveConnection::Ethernet(
            ethernet.connection.clone().unwrap_or_default(),
        )),
        (None, Some(wifi)) => Some(ActiveConnection::Wifi((
            wifi.connection.clone().unwrap_or_default(),
            0,
        ))),
        _ => None,
    };
}

fn monitor() {
    thread::spawn(move || loop {
        let mut handle = Command::new("nmcli")
            .arg("monitor")
            .stdout(Stdio::piped())
            .stdin(std::process::Stdio::null())
            .spawn()
            .expect("Failed to execute command");

        // Create a buffered reader to read the command's output

        let mut stdout_lines = BufReader::new(handle.stdout.take().unwrap()).lines();

        let mut last_time = Instant::now();
        loop {
            let line = stdout_lines.next().unwrap().unwrap();
            let delta = last_time.elapsed();

            if delta.as_millis() > 50 {
                thread::sleep(Duration::from_millis(500));
                println!("stdout: {}", line);

                make_content();

                last_time = Instant::now();
            }
        }
    });

    let mut line = String::new();
    let stdin = io::stdin();
    stdin.lock().read_line(&mut line).unwrap();
}

#[cfg(test)]
mod tests {
    use crate::net::monitor;

    #[test]
    fn monitor_changes() {
        monitor();

        println!("exit");
    }
}
