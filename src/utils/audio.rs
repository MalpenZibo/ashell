use futures_signals::signal::Mutable;
use serde::Deserialize;
use std::{
    process::Stdio,
    time::{Duration, Instant},
};
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::Command,
    time::sleep,
};

#[derive(Deserialize, Debug)]
struct RawProperties {
    #[serde(alias = "device.description")]
    device_description: String,
}

#[derive(Deserialize, Debug)]
struct RawChannelVolume {
    value_percent: String,
}

#[derive(Deserialize, Debug)]
struct RawVolume {
    #[serde(alias = "front-left")]
    front_left: RawChannelVolume,
    #[serde(alias = "front-right")]
    front_right: RawChannelVolume,
}

#[derive(Deserialize, Debug)]
struct RawPort {
    name: String,
    description: String,
    r#type: String,
    availability: String,
}

#[derive(Deserialize, Debug)]
struct RawEntry {
    index: u32,
    active_port: Option<String>,
    state: String,
    volume: RawVolume,
    balance: f32,
    mute: bool,
    properties: RawProperties,
    ports: Vec<RawPort>,
}

#[derive(Debug)]
pub struct Sink {
    pub index: u32,
    pub name: String,
    pub description: String,
    pub r#type: String,
    pub volume: u32,
    pub mute: bool,
    pub active: bool,
}

impl Sink {
    fn is_muted(&self) -> bool {
        self.mute || self.volume == 0
    }

    pub fn to_icon(&self) -> &str {
        if self.is_muted() {
            "󰸈"
        } else if self.volume < 34 {
            "󰕿"
        } else if self.volume < 67 {
            "󰖀"
        } else {
            "󰕾"
        }
    }

    pub fn to_type_icon(&self) -> &str {
        match self {
            Sink { r#type, .. } if r#type == "Headphones" && self.is_muted() => "󰟎",
            Sink { r#type, .. } if r#type == "Headphones" && !self.is_muted() => "󰋋",
            _ => {
                if self.is_muted() {
                    "󰖁"
                } else {
                    "󰕾"
                }
            }
        }
    }
}

#[derive(Debug)]
pub struct Source {
    pub index: u32,
    pub name: String,
    pub description: String,
    pub volume: u32,
    pub mute: bool,
    pub active: bool,
}

impl Source {
    pub fn to_icon(&self) -> &str {
        if self.volume > 0 && !self.mute {
            "󰍬"
        } else {
            "󰍭"
        }
    }
}

async fn get_sinks() -> Vec<Sink> {
    let command = Command::new("pactl")
        .args(["-f", "json", "list", "sinks"])
        .stdout(Stdio::piped())
        .output()
        .await
        .expect("Failed to execute pactl command");

    let output = String::from_utf8_lossy(&command.stdout);

    let raw_entry: Vec<RawEntry> = serde_json::from_str(&output).unwrap();

    let sinks = raw_entry
        .iter()
        .flat_map(|s| {
            s.ports
                .iter()
                .filter_map(|p| {
                    if p.availability != "not available" {
                        Some(Sink {
                            index: s.index,
                            name: p.name.to_string(),
                            description: format!(
                                "{} - {}",
                                p.description, s.properties.device_description
                            ),
                            r#type: p.r#type.to_string(),
                            volume: {
                                let left = s
                                    .volume
                                    .front_left
                                    .value_percent
                                    .replace('%', "")
                                    .parse::<u32>()
                                    .unwrap();

                                let right = s
                                    .volume
                                    .front_right
                                    .value_percent
                                    .replace('%', "")
                                    .parse::<u32>()
                                    .unwrap();

                                ((left as f32 * f32::abs((-1. + s.balance) / 2.))
                                    + right as f32 * f32::abs((1. + s.balance) / 2.))
                                    as u32
                            },
                            mute: s.mute,
                            active: s.active_port.as_ref() == Some(&p.name) && s.state == "RUNNING",
                        })
                    } else {
                        None
                    }
                })
                .collect::<Vec<Sink>>()
        })
        .collect();

    sinks
}

async fn get_sources() -> Vec<Source> {
    let command = Command::new("pactl")
        .args(["-f", "json", "list", "sources"])
        .stdout(Stdio::piped())
        .output()
        .await
        .expect("Failed to execute pactl command");

    let output = String::from_utf8_lossy(&command.stdout);

    let raw_entry: Vec<RawEntry> = serde_json::from_str(&output).unwrap();

    let sources = raw_entry
        .iter()
        .filter(|s| {
            !s.ports.is_empty()
                && s.ports
                    .iter()
                    .any(|p| p.r#type == "Mic" && p.availability != "not available")
        })
        .flat_map(|s| {
            s.ports
                .iter()
                .map(|p| Source {
                    index: s.index,
                    name: p.name.to_string(),
                    description: format!("{} - {}", p.description, s.properties.device_description),
                    volume: {
                        let left = s
                            .volume
                            .front_left
                            .value_percent
                            .replace('%', "")
                            .parse::<u32>()
                            .unwrap();

                        let right = s
                            .volume
                            .front_right
                            .value_percent
                            .replace('%', "")
                            .parse::<u32>()
                            .unwrap();

                        ((left as f32 * f32::abs((-1. + s.balance) / 2.))
                            + right as f32 * f32::abs((1. + s.balance) / 2.))
                            as u32
                    },
                    mute: s.mute,
                    active: s.active_port.as_ref() == Some(&p.name) && s.state == "RUNNING",
                })
                .collect::<Vec<Source>>()
        })
        .collect();

    sources
}

pub fn audio_monitor() -> (Mutable<Vec<Sink>>, Mutable<Vec<Source>>) {
    let sinks = Mutable::new(vec![]);
    let sources = Mutable::new(vec![]);

    tokio::spawn({
        let sinks = sinks.clone();
        let sources = sources.clone();

        async move {
            sinks.replace(get_sinks().await);
            sources.replace(get_sources().await);

            let mut handle = Command::new("pactl")
                .arg("subscribe")
                .stdout(Stdio::piped())
                .stdin(std::process::Stdio::null())
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

                        sinks.replace(get_sinks().await);
                        sources.replace(get_sources().await);

                        last_time = Instant::now();
                    }
                }
            }
        }
    });

    (sinks, sources)
}

pub async fn toggle_volume(sinks: Mutable<Vec<Sink>>) {
    Command::new("pactl")
        .args(["set-sink-mute", "@DEFAULT_SINK@", "toggle"])
        .stdout(Stdio::piped())
        .output()
        .await
        .expect("Failed to execute pactl command");

    sinks.replace(get_sinks().await);
}

pub async fn set_volume(sinks: Mutable<Vec<Sink>>, new_volume: u32) {
    let command = Command::new("pactl")
        .args(["get-sink-mute", "@DEFAULT_SINK@"])
        .stdout(Stdio::piped())
        .output()
        .await
        .expect("Failed to execute pactl command");

    let output = String::from_utf8_lossy(&command.stdout);
    if output == "Mute: yes" && new_volume > 0 {
        Command::new("pactl")
            .args(["set-sink-mute", "@DEFAULT_SINK@", "toggle"])
            .stdout(Stdio::piped())
            .output()
            .await
            .expect("Failed to execute pactl command");
    }

    Command::new("pactl")
        .args([
            "set-sink-volume",
            "@DEFAULT_SINK@",
            &format!("{}%", new_volume),
        ])
        .stdout(Stdio::piped())
        .output()
        .await
        .expect("Failed to execute pactl command");

    sinks.replace(get_sinks().await);
}

pub async fn set_sink(sinks: Mutable<Vec<Sink>>, index: u32, name: String) {
    Command::new("pactl")
        .arg("set-default-sink")
        .arg(&format!("{}", index))
        .stdout(Stdio::piped())
        .output()
        .await
        .expect("Failed to execute pactl command");

    Command::new("pactl")
        .arg("set-sink-port")
        .arg(&format!("{}", index))
        .arg(&name)
        .stdout(Stdio::piped())
        .output()
        .await
        .expect("Failed to execute pactl command");

    sinks.replace(get_sinks().await);
}

pub async fn set_microphone(sources: Mutable<Vec<Source>>, new_volume: u32) {
    let command = Command::new("pactl")
        .args(["get-source-mute", "@DEFAULT_SOURCE@"])
        .stdout(Stdio::piped())
        .output()
        .await
        .expect("Failed to execute pactl command");

    let output = String::from_utf8_lossy(&command.stdout);
    if output == "Mute: yes" && new_volume > 0 {
        Command::new("pactl")
            .args(["set-source-mute", "@DEFAULT_SOURCE@", "toggle"])
            .stdout(Stdio::piped())
            .output()
            .await
            .expect("Failed to execute pactl command");
    }

    Command::new("pactl")
        .args([
            "set-source-volume",
            "@DEFAULT_SOURCE@",
            &format!("{}%", new_volume),
        ])
        .stdout(Stdio::piped())
        .output()
        .await
        .expect("Failed to execute pactl command");

    sources.replace(get_sources().await);
}

pub async fn toggle_microphone(sources: Mutable<Vec<Source>>) {
    Command::new("pactl")
        .args(["set-source-mute", "@DEFAULT_SOURCE@", "toggle"])
        .stdout(Stdio::piped())
        .output()
        .await
        .expect("Failed to execute pactl command");

    sources.replace(get_sources().await);
}

pub async fn set_source(sources: Mutable<Vec<Source>>, index: u32, name: String) {
    Command::new("pactl")
        .arg("set-default-source")
        .arg(&format!("{}", index))
        .stdout(Stdio::piped())
        .output()
        .await
        .expect("Failed to execute pactl command");

    Command::new("pactl")
        .arg("set-source-port")
        .arg(&format!("{}", index))
        .arg(&name)
        .stdout(Stdio::piped())
        .output()
        .await
        .expect("Failed to execute pactl command");

    sources.replace(get_sources().await);
}
