use std::{
    io::{BufRead, BufReader},
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

use futures_signals::signal::Mutable;
use serde::Deserialize;

use crate::utils::poll;

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
    device: String,
}

pub enum ActiveConnection {
    Ethernet(String),
    Wifi {
        ssid: String,
        device: String,
        signal: u32,
    },
}

static WIFI_SIGNAL_ICONS: [&str; 5] = ["󰤭", "󰤟", "󰤢", "󰤥", "󰤨"];

impl ActiveConnection {
    pub fn to_icon(&self) -> &str {
        match self {
            ActiveConnection::Ethernet(_) => "󰈀",
            ActiveConnection::Wifi { signal, .. } => WIFI_SIGNAL_ICONS[*signal as usize],
        }
    }
}

#[derive(Debug)]
pub struct Vpn {
    pub name: String,
    pub active: bool,
}

#[derive(Deserialize, Debug)]
struct IWConfigOut {
    link_quality: String,
}

fn get_connected_wifi_signal(device: &str) -> u32 {
    let command = Command::new("iwconfig")
        .arg(device)
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to execute iwconfig command");

    let command = Command::new("jc")
        .arg("--iwconfig")
        .stdin(Stdio::from(command.stdout.unwrap()))
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to execute jc command");

    let output = command
        .wait_with_output()
        .expect("Failed to read jc command output");
    let output = String::from_utf8_lossy(&output.stdout);

    println!("Output: {}", output);

    let iwconfig = serde_json::from_str::<Vec<IWConfigOut>>(&output).unwrap();
    let iwconfig = iwconfig.first().map(|c| {
        c.link_quality
            .split('/')
            .map(|s| s.parse().ok())
            .collect::<Vec<Option<i32>>>()
    });

    if let Some(iwconfig) = iwconfig {
        match &iwconfig[..] {
            &[Some(quality), Some(max), ..] => (quality as f32 / max as f32 * 4.).floor() as u32,
            _ => 0,
        }
    } else {
        0
    }
}

pub fn get_active_connection() -> Option<ActiveConnection> {
    let command = Command::new("nmcli")
        .arg("d")
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to execute nmcli command");

    let command = Command::new("jc")
        .arg("--nmcli")
        .stdin(Stdio::from(command.stdout.unwrap()))
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to execute jc command");

    let output = command
        .wait_with_output()
        .expect("Failed to read jc command output");
    let output = String::from_utf8_lossy(&output.stdout);

    let devices: Vec<Device> = serde_json::from_str(&output).unwrap();

    let ethernet_connected = devices
        .iter()
        .find(|d| d.r#type == "ethernet" && d.state == "connected");
    let wifi_connected = devices
        .iter()
        .find(|d| d.r#type == "wifi" && d.state == "connected");

    match (ethernet_connected, wifi_connected) {
        (Some(ethernet), _) => Some(ActiveConnection::Ethernet(
            ethernet.connection.clone().unwrap_or_default(),
        )),
        (None, Some(wifi)) => Some(ActiveConnection::Wifi {
            ssid: wifi.connection.clone().unwrap_or_default(),
            device: wifi.device.clone(),
            signal: get_connected_wifi_signal(&wifi.device),
        }),
        _ => None,
    }
}

fn get_vpn() -> Vec<Vpn> {
    let command = Command::new("nmcli")
        .args(["connection", "show", "--active"])
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to execute nmcli command");

    let command = Command::new("jc")
        .arg("--nmcli")
        .stdin(Stdio::from(command.stdout.unwrap()))
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to execute jc command");

    let output = command
        .wait_with_output()
        .expect("Failed to read jc command output");
    let output = String::from_utf8_lossy(&output.stdout);

    let connections: Vec<Connection> = serde_json::from_str(&output).unwrap();
    connections
        .iter()
        .filter_map(|c| {
            if c.r#type == "vpn" {
                Some(Vpn {
                    name: c.name.clone(),
                    active: !c.device.is_empty(),
                })
            } else {
                None
            }
        })
        .collect()
}

pub fn net_monitor(
    active_connection: Mutable<Option<ActiveConnection>>,
    vpn_list: Mutable<Vec<Vpn>>,
) {
    let active_connection1 = active_connection.clone();
    poll(
        move || {
            let wifi = active_connection1
                .read_only()
                .lock_ref()
                .as_ref()
                .and_then(|c| match c {
                    ActiveConnection::Wifi { ssid, device, .. } => {
                        Some((ssid.to_owned(), device.to_owned()))
                    }
                    _ => None,
                });

            if let Some((ssid, device)) = wifi {
                let new_signal = get_connected_wifi_signal(&device);
                active_connection1.replace(Some(ActiveConnection::Wifi {
                    ssid,
                    device,
                    signal: new_signal,
                }));
            }
        },
        Duration::from_secs(60),
    );

    tokio::spawn(async move {
        active_connection.replace(get_active_connection());
        vpn_list.replace(get_vpn());

        let mut handle = Command::new("nmcli")
            .arg("monitor")
            .stdout(Stdio::piped())
            .stdin(std::process::Stdio::null())
            .spawn()
            .expect("Failed to execute command");

        let mut stdout_lines = BufReader::new(handle.stdout.take().unwrap()).lines();

        let mut last_time = Instant::now();
        loop {
            let line = stdout_lines.next().unwrap().unwrap();
            let delta = last_time.elapsed();

            if delta.as_millis() > 50 {
                thread::sleep(Duration::from_millis(500));
                println!("stdout: {}", line);

                active_connection.replace(get_active_connection());
                vpn_list.replace(get_vpn());

                last_time = Instant::now();
            }
        }
    });
}
