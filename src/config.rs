use iced::futures::{SinkExt, StreamExt};
use inotify::{EventMask, Inotify, WatchMask};
use log::warn;
use serde::{Deserialize, Deserializer};
use std::{env, fs::File, path::Path};

use crate::app::Message;

const CONFIG_PATH: &str = "~/.config/ashell.yml";

#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct UpdatesModuleConfig {
    pub check_cmd: String,
    pub update_cmd: String,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SystemModuleConfig {
    #[serde(default)]
    pub disabled: bool,
    #[serde(default = "default_cpu_warn_threshold")]
    pub cpu_warn_threshold: u32,
    #[serde(default = "default_cpu_alert_threshold")]
    pub cpu_alert_threshold: u32,
    #[serde(default = "default_mem_warn_threshold")]
    pub mem_warn_threshold: u32,
    #[serde(default = "default_mem_alert_threshold")]
    pub mem_alert_threshold: u32,
    #[serde(default = "default_temp_warn_threshold")]
    pub temp_warn_threshold: i32,
    #[serde(default = "default_temp_alert_threshold")]
    pub temp_alert_threshold: i32,
}

fn default_cpu_warn_threshold() -> u32 {
    60
}

fn default_cpu_alert_threshold() -> u32 {
    80
}

fn default_mem_warn_threshold() -> u32 {
    70
}

fn default_mem_alert_threshold() -> u32 {
    85
}

fn default_temp_warn_threshold() -> i32 {
    60
}

fn default_temp_alert_threshold() -> i32 {
    80
}

impl Default for SystemModuleConfig {
    fn default() -> Self {
        Self {
            disabled: false,
            cpu_warn_threshold: default_cpu_warn_threshold(),
            cpu_alert_threshold: default_cpu_alert_threshold(),
            mem_warn_threshold: default_mem_warn_threshold(),
            mem_alert_threshold: default_mem_alert_threshold(),
            temp_warn_threshold: default_temp_warn_threshold(),
            temp_alert_threshold: default_temp_alert_threshold(),
        }
    }
}

#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ClockModuleConfig {
    pub format: String,
}

impl Default for ClockModuleConfig {
    fn default() -> Self {
        Self {
            format: "%a %d %b %R".to_string(),
        }
    }
}

#[derive(Deserialize, Default, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SettingsModuleConfig {
    pub lock_cmd: Option<String>,
    pub audio_sinks_more_cmd: Option<String>,
    pub audio_sources_more_cmd: Option<String>,
    pub wifi_more_cmd: Option<String>,
    pub vpn_more_cmd: Option<String>,
    pub bluetooth_more_cmd: Option<String>,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    #[serde(default = "default_log_level")]
    pub log_level: log::LevelFilter,
    pub app_launcher_cmd: Option<String>,
    #[serde(default = "default_truncate_title_after_length")]
    pub truncate_title_after_length: u32,
    #[serde(deserialize_with = "try_default")]
    pub updates: Option<UpdatesModuleConfig>,
    #[serde(default)]
    pub system: SystemModuleConfig,
    #[serde(default)]
    pub clock: ClockModuleConfig,
    #[serde(default)]
    pub settings: SettingsModuleConfig,
}

fn try_default<'de, T, D>(deserializer: D) -> Result<T, D::Error>
where
    T: Deserialize<'de> + Default + std::fmt::Debug,
    D: Deserializer<'de>,
{
    // Try to deserialize the UpdatesModuleConfig
    let result: Result<T, D::Error> = T::deserialize(deserializer);

    // If it fails, return None
    result.or_else(|err| {
        warn!("error deserializing: {:?}", err);
        Ok(T::default())
    })
}

fn default_log_level() -> log::LevelFilter {
    log::LevelFilter::Warn
}

fn default_truncate_title_after_length() -> u32 {
    150
}

impl Default for Config {
    fn default() -> Self {
        Self {
            log_level: default_log_level(),
            app_launcher_cmd: None,
            truncate_title_after_length: default_truncate_title_after_length(),
            updates: None,
            system: SystemModuleConfig::default(),
            clock: ClockModuleConfig::default(),
            settings: SettingsModuleConfig::default(),
        }
    }
}

pub fn read_config() -> Result<Config, serde_yaml::Error> {
    let home_dir = env::var("HOME").expect("Could not get HOME environment variable");
    let file_path = format!("{}{}", home_dir, CONFIG_PATH.replace('~', ""));
    let config_file = File::open(file_path);

    if let Ok(config_file) = config_file {
        log::info!("Reading config file");
        serde_yaml::from_reader(config_file)
    } else {
        Ok(Config::default())
    }
}

pub fn subscription() -> iced::Subscription<Message> {
    iced::subscription::channel("config-watcher", 100, move |mut output| async move {
        let home_dir = env::var("HOME").expect("Could not get HOME environment variable");
        let file_path = format!("{}{}", home_dir, CONFIG_PATH.replace('~', ""));

        loop {
            let inotify = Inotify::init().expect("Failed to initialize inotify");

            let path = Path::new(&file_path);
            if path.exists() {
                log::debug!("watch path {:?}", path);
                inotify
                    .watches()
                    .add(
                        path,
                        WatchMask::MODIFY
                            .union(WatchMask::CLOSE_WRITE)
                            .union(WatchMask::DELETE)
                            .union(WatchMask::MOVE_SELF),
                    )
                    .expect("Failed to add file watch");
            } else {
                log::info!("watch directory {:?}", path.parent().unwrap());
                inotify
                    .watches()
                    .add(
                        path.parent().unwrap(),
                        WatchMask::CREATE
                            .union(WatchMask::MOVED_TO)
                            .union(WatchMask::MOVE_SELF),
                    )
                    .expect("Failed to add create file watch");
            }

            let mut buffer = [0; 1024];
            let mut stream = inotify
                .into_event_stream(&mut buffer)
                .expect("Failed to create event stream");

            loop {
                let event = stream.next().await;
                match event {
                    Some(Ok(inotify::Event {
                        mask: EventMask::CREATE | EventMask::MOVED_TO | EventMask::MOVE_SELF,
                        name: Some(name),
                        ..
                    })) => {
                        if name == "ashell.yml" {
                            log::info!("Config file created");

                            let new_config = read_config();
                            if let Ok(new_config) = new_config {
                                let _ = output
                                    .send(Message::ConfigChanged(Box::new(new_config)))
                                    .await;
                            } else {
                                log::warn!("Failed to read config file: {:?}", new_config);
                            }

                            break;
                        }
                    }
                    Some(Ok(inotify::Event {
                        mask: EventMask::MODIFY | EventMask::MOVE_SELF | EventMask::CLOSE_WRITE,
                        ..
                    })) => {
                        log::info!("Config file modified");

                        let new_config = read_config();
                        if let Ok(new_config) = new_config {
                            let _ = output
                                .send(Message::ConfigChanged(Box::new(new_config)))
                                .await;
                        } else {
                            log::warn!("Failed to read config file: {:?}", new_config);
                        }
                    }
                    Some(Ok(inotify::Event {
                        mask: EventMask::DELETE,
                        ..
                    })) => {
                        log::info!("Config file deleted");
                        let _ = output.send(Message::ConfigChanged(Box::default())).await;

                        break;
                    }
                    _ => {}
                }
            }
        }
    })
}
