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
struct RawSink {
    index: u32,
    active_port: String,
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
    pub fn to_icon(&self) -> &str {
        if self.mute || self.volume == 0 {
            "󰸈"
        } else if self.volume < 34 {
            "󰕿"
        } else if self.volume < 67 {
            "󰖀"
        } else {
            "󰕾"
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

    let raw_sinks: Vec<RawSink> = serde_json::from_str(&output).unwrap();

    let sinks = raw_sinks
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
                    active: s.active_port == p.name,
                })
                .collect::<Vec<Sink>>()
        })
        .collect();

    println!("Sinks: {:?}", sinks);

    sinks
}

fn get_sources() -> u32 {
    0
}

pub fn audio_subscribe(sinks: Mutable<Vec<Sink>>, sources: Mutable<u32>) {
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
