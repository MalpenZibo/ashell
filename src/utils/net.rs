use futures_signals::signal::Mutable;
use serde::Deserialize;
use std::{
    process::Stdio,
    time::{Duration, Instant},
};
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    join,
    process::Command,
    time::sleep,
};

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

#[derive(Debug, Clone)]
pub enum ActiveConnection {
    Ethernet(String),
    Wifi {
        ssid: String,
        device: String,
        signal: u32,
    },
}

static WIFI_SIGNAL_ICONS: [&str; 5] = ["󰤭", "󰤟", "󰤢", "󰤥", "󰤨"];
static WIFI_SIGNAL_CLASS: [&str; 5] = [
    "wifi-signal-0",
    "wifi-signal-1",
    "wifi-signal-2",
    "wifi-signal-3",
    "wifi-signal-4",
];

impl ActiveConnection {
    pub fn to_icon(&self) -> &str {
        match self {
            ActiveConnection::Ethernet(_) => "󰈀",
            ActiveConnection::Wifi { signal, .. } => WIFI_SIGNAL_ICONS[*signal as usize],
        }
    }

    pub fn get_classes(&self) -> Vec<&'static str> {
        match self {
            ActiveConnection::Ethernet(_) => vec!["ethernet"],
            ActiveConnection::Wifi { signal, .. } => {
                vec!["wifi", WIFI_SIGNAL_CLASS[*signal as usize]]
            }
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

async fn get_connected_wifi_signal(device: &str) -> u32 {
    let mut iwconfig = Command::new("iwconfig")
        .arg(device)
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to execute iwconfig command");

    let iwconfig_out: Stdio = iwconfig
        .stdout
        .take()
        .unwrap()
        .try_into()
        .expect("failed to convert to Stdio");

    let jc = Command::new("jc")
        .arg("--iwconfig")
        .stdin(iwconfig_out)
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to execute jc command");

    let (_, jc_out) = join!(iwconfig.wait(), jc.wait_with_output());

    let jc_out = jc_out.expect("Failed to read jc command output").stdout;
    let output = String::from_utf8_lossy(&jc_out);

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

pub async fn get_active_connection() -> Option<ActiveConnection> {
    let mut nmcli = Command::new("nmcli")
        .arg("d")
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to execute nmcli command");

    let nmcli_out: Stdio = nmcli
        .stdout
        .take()
        .unwrap()
        .try_into()
        .expect("failed to convert to Stdio");

    let jc = Command::new("jc")
        .arg("--nmcli")
        .stdin(nmcli_out)
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to execute jc command");

    let (_, jc_out) = join!(nmcli.wait(), jc.wait_with_output());

    let jc_out = jc_out.expect("Failed to read jc command output").stdout;
    let output = String::from_utf8_lossy(&jc_out);

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
            signal: get_connected_wifi_signal(&wifi.device).await,
        }),
        _ => None,
    }
}

async fn get_vpn() -> Vec<Vpn> {
    let mut nmcli = Command::new("nmcli")
        .args(["connection", "show", "--active"])
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to execute nmcli command");

    let nmcli_out: Stdio = nmcli
        .stdout
        .take()
        .unwrap()
        .try_into()
        .expect("failed to convert to Stdio");

    let jc = Command::new("jc")
        .arg("--nmcli")
        .stdin(nmcli_out)
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to execute jc command");

    let (_, jc_out) = join!(nmcli.wait(), jc.wait_with_output());

    let jc_out = jc_out.expect("Failed to read jc command output").stdout;
    let output = String::from_utf8_lossy(&jc_out);

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

pub fn net_monitor() -> (Mutable<Option<ActiveConnection>>, Mutable<Vec<Vpn>>) {
    let active_connection = Mutable::new(None);
    let vpn_list = Mutable::new(vec![]);

    tokio::spawn({
        let active_connection = active_connection.clone();
        async move {
            let wifi = active_connection.get_cloned().and_then(|c| match c {
                ActiveConnection::Wifi { ssid, device, .. } => Some((ssid, device)),
                _ => None,
            });

            if let Some((ssid, device)) = wifi {
                let new_signal = get_connected_wifi_signal(&device).await;
                active_connection.replace(Some(ActiveConnection::Wifi {
                    ssid,
                    device,
                    signal: new_signal,
                }));
            }
            sleep(Duration::from_secs(60)).await
        }
    });

    tokio::spawn({
        let active_connection = active_connection.clone();
        let vpn_list = vpn_list.clone();

        async move {
            active_connection.set(get_active_connection().await);
            vpn_list.set(get_vpn().await);

            let mut handle = Command::new("nmcli")
                .arg("monitor")
                .stdout(Stdio::piped())
                .stdin(Stdio::null())
                .spawn()
                .expect("Failed to execute command");

            if let Some(ref mut stdout) = handle.stdout {
                let reader = BufReader::new(stdout);
                let mut lines = reader.lines();

                let mut last_time = Instant::now();
                loop {
                    let _line = lines
                        .next_line()
                        .await
                        .ok()
                        .flatten()
                        .unwrap_or("".to_string());

                    let delta = last_time.elapsed();

                    if delta.as_millis() > 50 {
                        sleep(Duration::from_millis(500)).await;

                        active_connection.replace(get_active_connection().await);
                        vpn_list.replace(get_vpn().await);

                        last_time = Instant::now();
                    }
                }
            }
        }
    });

    (active_connection, vpn_list)
}
