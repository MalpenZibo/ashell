use std::{
    io::{BufRead, BufReader},
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

use futures_signals::signal::Mutable;
use serde::Deserialize;

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

fn get_sinks() -> Vec<Sink> {
    let command = Command::new("pactl")
        .args(["-f", "json", "list", "sinks"])
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to execute pactl command");

    let output = command
        .wait_with_output()
        .expect("Failed to read jc command output");
    let output = String::from_utf8_lossy(&output.stdout);

    let raw_entry: Vec<RawEntry> = serde_json::from_str(&output).unwrap();

    let sinks = raw_entry
        .iter()
        .flat_map(|s| {
            s.ports
                .iter()
                .map(|p| Sink {
                    index: s.index,
                    name: p.name.to_string(),
                    description: format!("{} - {}", p.description, s.properties.device_description),
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
                    active: s.active_port.as_ref() == Some(&p.name),
                })
                .collect::<Vec<Sink>>()
        })
        .collect();

    println!("Sinks: {:?}", sinks);

    sinks
}

fn get_sources() -> Vec<Source> {
    let command = Command::new("pactl")
        .args(["-f", "json", "list", "sources"])
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to execute pactl command");

    let output = command
        .wait_with_output()
        .expect("Failed to read jc command output");
    let output = String::from_utf8_lossy(&output.stdout);

    let raw_entry: Vec<RawEntry> = serde_json::from_str(&output).unwrap();

    let sources = raw_entry
        .iter()
        .filter(|s| !s.ports.is_empty())
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

pub fn audio_subscribe(sinks: Mutable<Vec<Sink>>, sources: Mutable<Vec<Source>>) {
    sinks.replace(get_sinks());
    sources.replace(get_sources());

    tokio::spawn(async move {
        let mut handle = Command::new("pactl")
            .arg("subscribe")
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

                sinks.replace(get_sinks());
                sources.replace(get_sources());

                last_time = Instant::now();
            }
        }
    });
}

pub fn toggle_volume(sinks: Mutable<Vec<Sink>>) {
    let command = Command::new("pactl")
        .args(["set-sink-mute", "@DEFAULT_SINK@", "toggle"])
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to execute pactl command");

    command
        .wait_with_output()
        .expect("Failed to read pactl toggle command output");

    sinks.replace(get_sinks());
}

pub fn set_volume(sinks: Mutable<Vec<Sink>>, new_volume: u32) {
    let command = Command::new("pactl")
        .args(["get-sink-mute", "@DEFAULT_SINK@"])
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to execute pactl command");

    let output = command
        .wait_with_output()
        .expect("Failed to read pactl toggle command output");
    let output = String::from_utf8_lossy(&output.stdout);
    if output == "Mute: yes" && new_volume > 0 {
        let command = Command::new("pactl")
            .args(["set-sink-mute", "@DEFAULT_SINK@", "toggle"])
            .stdout(Stdio::piped())
            .spawn()
            .expect("Failed to execute pactl command");

        command
            .wait_with_output()
            .expect("Failed to read pactl toggle command output");
    }

    let command = Command::new("pactl")
        .args([
            "set-sink-volume",
            "@DEFAULT_SINK@",
            &format!("{}%", new_volume),
        ])
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to execute pactl command");

    command
        .wait_with_output()
        .expect("Failed to read pactl toggle command output");

    sinks.replace(get_sinks());
}

pub fn set_sink(sinks: Mutable<Vec<Sink>>, index: u32, name: String) {
    let command = Command::new("pactl")
        .args(["set-sink-port", &format!("{}", index), &name])
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to execute pactl command");

    command
        .wait_with_output()
        .expect("Failed to read pactl toggle command output");

    sinks.replace(get_sinks());
}

pub fn set_microphone(sources: Mutable<Vec<Source>>, new_volume: u32) {
    let command = Command::new("pactl")
        .args(["get-source-mute", "@DEFAULT_SOURCE@"])
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to execute pactl command");

    let output = command
        .wait_with_output()
        .expect("Failed to read pactl toggle command output");
    let output = String::from_utf8_lossy(&output.stdout);
    if output == "Mute: yes" && new_volume > 0 {
        let command = Command::new("pactl")
            .args(["set-source-mute", "@DEFAULT_SOURCE@", "toggle"])
            .stdout(Stdio::piped())
            .spawn()
            .expect("Failed to execute pactl command");

        command
            .wait_with_output()
            .expect("Failed to read pactl toggle command output");
    }

    let command = Command::new("pactl")
        .args([
            "set-source-volume",
            "@DEFAULT_SOURCE@",
            &format!("{}%", new_volume),
        ])
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to execute pactl command");

    command
        .wait_with_output()
        .expect("Failed to read pactl toggle command output");

    sources.replace(get_sources());
}

pub fn toggle_microphone(sources: Mutable<Vec<Source>>) {
    let command = Command::new("pactl")
        .args(["set-source-mute", "@DEFAULT_SOURCE@", "toggle"])
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to execute pactl command");

    command
        .wait_with_output()
        .expect("Failed to read pactl toggle command output");

    sources.replace(get_sources());
}

pub fn set_source(sources: Mutable<Vec<Source>>, index: u32, name: String) {
    let command = Command::new("pactl")
        .args(["set-source-port", &format!("{}", index), &name])
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to execute pactl command");

    command
        .wait_with_output()
        .expect("Failed to read pactl toggle command output");

    sources.replace(get_sources());
}

