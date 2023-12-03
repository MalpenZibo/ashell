use iced::Color;
use serde::Deserialize;
use std::process::Stdio;
use tokio::{join, process::Command};
use crate::{
    components::icons::Icons,
    style::{RED, TEXT, YELLOW},
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

static WIFI_SIGNAL_ICONS: [Icons; 5] = [
    Icons::Wifi0,
    Icons::Wifi1,
    Icons::Wifi2,
    Icons::Wifi3,
    Icons::Wifi4,
];

impl ActiveConnection {
    pub fn name(&self) -> &str {
        match self {
            ActiveConnection::Ethernet(name) => name,
            ActiveConnection::Wifi { ssid, .. } => ssid,
        }
    }

    pub fn get_icon(&self) -> Icons {
        match self {
            ActiveConnection::Ethernet(_) => Icons::Ethernet,
            ActiveConnection::Wifi { signal, .. } => WIFI_SIGNAL_ICONS[*signal as usize],
        }
    }

    pub fn get_icon_type(&self) -> Icons {
        match self {
            ActiveConnection::Ethernet(_) => Icons::Ethernet,
            ActiveConnection::Wifi { .. } => Icons::Wifi4,
        }
    }

    pub fn get_color(&self) -> Color {
        match self {
            ActiveConnection::Ethernet(_) => TEXT,
            ActiveConnection::Wifi { signal, .. } => match signal {
                0 => RED,
                1 => YELLOW,
                _ => TEXT,
            },
        }
    }
}

#[derive(Debug, Clone)]
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

pub async fn get_vpn() -> Vec<Vpn> {
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
