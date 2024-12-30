use hex_color::HexColor;
use iced::{
    futures::{SinkExt, StreamExt},
    stream::channel,
    theme::palette,
    Color, Subscription,
};
use inotify::{EventMask, Inotify, WatchMask};
use serde::Deserialize;
use std::{any::TypeId, env, fs::File, path::Path, time::Duration};
use tokio::time::sleep;

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

#[derive(Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct KeyboardLayoutModule {
    #[serde(default)]
    pub disabled: bool,
}

#[derive(Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct KeyboardSubmapModule {
    #[serde(default)]
    pub disabled: bool,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct KeyboardModuleConfig {
    #[serde(default = "default_keyboard_layout")]
    pub layout: KeyboardLayoutModule,
    #[serde(default = "default_keyboard_submap")]
    pub submap: KeyboardSubmapModule,
}

fn default_keyboard_layout() -> KeyboardLayoutModule {
    KeyboardLayoutModule { disabled: false }
}

fn default_keyboard_submap() -> KeyboardSubmapModule {
    KeyboardSubmapModule { disabled: false }
}

impl Default for KeyboardModuleConfig {
    fn default() -> Self {
        Self {
            layout: default_keyboard_layout(),
            submap: default_keyboard_submap(),
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

#[derive(Deserialize, Clone, Copy, Debug)]
#[serde(untagged)]
#[serde(rename_all = "camelCase")]
pub enum AppearanceColor {
    Simple(HexColor),
    Complete {
        base: HexColor,
        strong: Option<HexColor>,
        weak: Option<HexColor>,
        text: Option<HexColor>,
    },
}

impl AppearanceColor {
    pub fn get_base(&self) -> Color {
        match self {
            AppearanceColor::Simple(color) => Color::from_rgb8(color.r, color.g, color.b),
            AppearanceColor::Complete { base, .. } => Color::from_rgb8(base.r, base.g, base.b),
        }
    }

    pub fn get_text(&self) -> Option<Color> {
        match self {
            AppearanceColor::Simple(_) => None,
            AppearanceColor::Complete { text, .. } => {
                text.map(|color| Color::from_rgb8(color.r, color.g, color.b))
            }
        }
    }

    pub fn get_weak_pair(&self, text_fallback: Color) -> Option<palette::Pair> {
        match self {
            AppearanceColor::Simple(_) => None,
            AppearanceColor::Complete { weak, text, .. } => weak.map(|color| {
                palette::Pair::new(
                    Color::from_rgb8(color.r, color.g, color.b),
                    text.map(|color| Color::from_rgb8(color.r, color.g, color.b))
                        .unwrap_or(text_fallback),
                )
            }),
        }
    }

    pub fn get_strong_pair(&self, text_fallback: Color) -> Option<palette::Pair> {
        match self {
            AppearanceColor::Simple(_) => None,
            AppearanceColor::Complete { strong, text, .. } => strong.map(|color| {
                palette::Pair::new(
                    Color::from_rgb8(color.r, color.g, color.b),
                    text.map(|color| Color::from_rgb8(color.r, color.g, color.b))
                        .unwrap_or(text_fallback),
                )
            }),
        }
    }
}

#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Appearance {
    #[serde(default = "default_background_color")]
    pub background_color: AppearanceColor,
    #[serde(default = "default_primary_color")]
    pub primary_color: AppearanceColor,
    #[serde(default = "default_secondary_color")]
    pub secondary_color: AppearanceColor,
    #[serde(default = "default_success_color")]
    pub success_color: AppearanceColor,
    #[serde(default = "default_danger_color")]
    pub danger_color: AppearanceColor,
    #[serde(default = "default_text_color")]
    pub text_color: AppearanceColor,
    #[serde(default = "default_workspace_colors")]
    pub workspace_colors: Vec<AppearanceColor>,
    pub special_workspace_colors: Option<Vec<AppearanceColor>>,
}

static PRIMARY: HexColor = HexColor::rgb(250, 179, 135);

fn default_background_color() -> AppearanceColor {
    AppearanceColor::Complete {
        base: HexColor::rgb(30, 30, 46),
        strong: Some(HexColor::rgb(69, 71, 90)),
        weak: Some(HexColor::rgb(49, 50, 68)),
        text: None,
    }
}

fn default_primary_color() -> AppearanceColor {
    AppearanceColor::Complete {
        base: PRIMARY,
        strong: None,
        weak: None,
        text: Some(HexColor::rgb(30, 30, 46)),
    }
}

fn default_secondary_color() -> AppearanceColor {
    AppearanceColor::Complete {
        base: HexColor::rgb(17, 17, 27),
        strong: Some(HexColor::rgb(24, 24, 37)),
        weak: None,
        text: None,
    }
}

fn default_success_color() -> AppearanceColor {
    AppearanceColor::Simple(HexColor::rgb(166, 227, 161))
}

fn default_danger_color() -> AppearanceColor {
    AppearanceColor::Complete {
        base: HexColor::rgb(243, 139, 168),
        weak: Some(HexColor::rgb(249, 226, 175)),
        strong: None,
        text: None,
    }
}

fn default_text_color() -> AppearanceColor {
    AppearanceColor::Simple(HexColor::rgb(205, 214, 244))
}

fn default_workspace_colors() -> Vec<AppearanceColor> {
    vec![
        AppearanceColor::Simple(PRIMARY),
        AppearanceColor::Simple(HexColor::rgb(180, 190, 254)),
        AppearanceColor::Simple(HexColor::rgb(203, 166, 247)),
    ]
}

impl Default for Appearance {
    fn default() -> Self {
        Self {
            background_color: default_background_color(),
            primary_color: default_primary_color(),
            secondary_color: default_secondary_color(),
            success_color: default_success_color(),
            danger_color: default_danger_color(),
            text_color: default_text_color(),
            workspace_colors: default_workspace_colors(),
            special_workspace_colors: None,
        }
    }
}

#[derive(Deserialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Position {
    #[default]
    Top,
    Bottom,
}

#[derive(Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Module {
    Launcher,
    Clipboard,
    Updates,
    Workspaces,
    Title,
    SystemInfo,
    KeyboardSubmap,
    KeyboardLayout,
    Clock,
    Privacy,
    Settings,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    #[serde(default = "default_log_level")]
    pub log_level: String,
    #[serde(default)]
    pub position: Position,
    #[serde(default)]
    pub outputs: Vec<String>,
    pub app_launcher_cmd: Option<String>,
    pub clipboard_cmd: Option<String>,
    #[serde(default = "default_truncate_title_after_length")]
    pub truncate_title_after_length: u32,
    #[serde(default = "default_left")]
    pub left: Vec<Module>,
    #[serde(default = "default_center")]
    pub center: Vec<Module>,
    #[serde(default = "default_right")]
    pub right: Vec<Module>,
    #[serde(default)]
    pub updates: Option<UpdatesModuleConfig>,
    #[serde(default)]
    pub system: SystemModuleConfig,
    #[serde(default)]
    pub keyboard: KeyboardModuleConfig,
    #[serde(default)]
    pub clock: ClockModuleConfig,
    #[serde(default)]
    pub settings: SettingsModuleConfig,
    #[serde(default)]
    pub appearance: Appearance,
}

fn default_log_level() -> String {
    "warn".to_owned()
}

fn default_truncate_title_after_length() -> u32 {
    150
}

fn default_left() -> Vec<Module> {
    vec![Module::Workspaces]
}

fn default_center() -> Vec<Module> {
    vec![Module::Title]
}

fn default_right() -> Vec<Module> {
    vec![Module::Clock, Module::Settings]
}

impl Default for Config {
    fn default() -> Self {
        Self {
            log_level: default_log_level(),
            position: Position::Top,
            outputs: vec![],
            app_launcher_cmd: None,
            clipboard_cmd: None,
            truncate_title_after_length: default_truncate_title_after_length(),
            updates: None,
            system: SystemModuleConfig::default(),
            keyboard: KeyboardModuleConfig::default(),
            clock: ClockModuleConfig::default(),
            settings: SettingsModuleConfig::default(),
            appearance: Appearance::default(),
            left: default_left(),
            center: default_center(),
            right: default_right(),
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

pub fn subscription() -> Subscription<Message> {
    let id = TypeId::of::<Config>();

    Subscription::run_with_id(
        id,
        channel(100, |mut output| async move {
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
                    log::debug!("waiting for event");
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

                            sleep(Duration::from_millis(500)).await;

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
                        Some(Ok(inotify::Event {
                            mask: EventMask::DELETE,
                            ..
                        })) => {
                            log::info!("Config file deleted");
                            let _ = output.send(Message::ConfigChanged(Box::default())).await;

                            break;
                        }
                        other => {
                            log::debug!("other event {:?}", other);
                        }
                    }
                }
            }
        }),
    )
}
