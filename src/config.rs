use hex_color::HexColor;
use iced::{
    Color, Subscription,
    futures::{SinkExt, StreamExt},
    stream::channel,
    theme::palette,
};
use inotify::{Event, EventMask, Inotify, WatchMask};
use regex::Regex;
use serde::{Deserialize, Deserializer, de::Visitor};
use serde_with::DisplayFromStr;
use serde_with::serde_as;
use std::{
    any::TypeId, collections::HashMap, env, error::Error, fs::File, io::Read, ops::Deref,
    path::Path,
};

use crate::app::Message;

const CONFIG_PATH: &str = "~/.config/ashell/config.toml";

#[derive(Deserialize, Clone, Debug)]
pub struct UpdatesModuleConfig {
    pub check_cmd: String,
    pub update_cmd: String,
}

#[derive(Deserialize, Clone, Default, PartialEq, Eq, Debug)]
pub enum WorkspaceVisibilityMode {
    #[default]
    All,
    MonitorSpecific,
}

#[derive(Deserialize, Clone, Default, Debug)]
pub struct WorkspacesModuleConfig {
    #[serde(default)]
    pub visibility_mode: WorkspaceVisibilityMode,
    #[serde(default)]
    pub enable_workspace_filling: bool,
    pub max_workspaces: Option<u32>,
}

#[derive(Deserialize, Clone, Default, PartialEq, Eq, Debug)]
pub enum WindowTitleMode {
    #[default]
    Title,
    Class,
}

#[derive(Deserialize, Clone, Default, Debug)]
pub struct WindowTitleConfig {
    #[serde(default)]
    pub mode: WindowTitleMode,
    #[serde(default = "default_truncate_title_after_length")]
    pub truncate_title_after_length: u32,
}

#[derive(Deserialize, Clone, Default, Debug)]
pub struct KeyboardLayoutModuleConfig {
    #[serde(default)]
    pub labels: HashMap<String, String>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct SystemInfoCpu {
    #[serde(default = "default_cpu_warn_threshold")]
    pub warn_threshold: u32,
    #[serde(default = "default_cpu_alert_threshold")]
    pub alert_threshold: u32,
}

impl Default for SystemInfoCpu {
    fn default() -> Self {
        Self {
            warn_threshold: default_cpu_warn_threshold(),
            alert_threshold: default_cpu_alert_threshold(),
        }
    }
}

#[derive(Deserialize, Clone, Debug)]
pub struct SystemInfoMemory {
    #[serde(default = "default_mem_warn_threshold")]
    pub warn_threshold: u32,
    #[serde(default = "default_mem_alert_threshold")]
    pub alert_threshold: u32,
}

impl Default for SystemInfoMemory {
    fn default() -> Self {
        Self {
            warn_threshold: default_mem_warn_threshold(),
            alert_threshold: default_mem_alert_threshold(),
        }
    }
}

#[derive(Deserialize, Clone, Debug)]
pub struct SystemInfoTemperature {
    #[serde(default = "default_temp_warn_threshold")]
    pub warn_threshold: i32,
    #[serde(default = "default_temp_alert_threshold")]
    pub alert_threshold: i32,
}

impl Default for SystemInfoTemperature {
    fn default() -> Self {
        Self {
            warn_threshold: default_temp_warn_threshold(),
            alert_threshold: default_temp_alert_threshold(),
        }
    }
}

#[derive(Deserialize, Clone, Debug)]
pub struct SystemInfoDisk {
    #[serde(default = "default_disk_warn_threshold")]
    pub warn_threshold: u32,
    #[serde(default = "default_disk_alert_threshold")]
    pub alert_threshold: u32,
}

impl Default for SystemInfoDisk {
    fn default() -> Self {
        Self {
            warn_threshold: default_disk_warn_threshold(),
            alert_threshold: default_disk_alert_threshold(),
        }
    }
}

#[derive(Deserialize, Clone, Debug)]
pub enum SystemIndicator {
    Cpu,
    Memory,
    MemorySwap,
    Temperature,
    Disk(String),
    IpAddress,
    DownloadSpeed,
    UploadSpeed,
}

#[derive(Deserialize, Clone, Debug)]
pub struct SystemModuleConfig {
    #[serde(default = "default_system_indicators")]
    pub indicators: Vec<SystemIndicator>,
    #[serde(default)]
    pub cpu: SystemInfoCpu,
    #[serde(default)]
    pub memory: SystemInfoMemory,
    #[serde(default)]
    pub temperature: SystemInfoTemperature,
    #[serde(default)]
    pub disk: SystemInfoDisk,
}

fn default_system_indicators() -> Vec<SystemIndicator> {
    vec![
        SystemIndicator::Cpu,
        SystemIndicator::Memory,
        SystemIndicator::Temperature,
    ]
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

fn default_disk_warn_threshold() -> u32 {
    80
}

fn default_disk_alert_threshold() -> u32 {
    90
}

impl Default for SystemModuleConfig {
    fn default() -> Self {
        Self {
            indicators: default_system_indicators(),
            cpu: SystemInfoCpu::default(),
            memory: SystemInfoMemory::default(),
            temperature: SystemInfoTemperature::default(),
            disk: SystemInfoDisk::default(),
        }
    }
}

#[derive(Deserialize, Clone, Debug)]
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

fn default_shutdown_cmd() -> String {
    "shutdown now".to_string()
}

fn default_suspend_cmd() -> String {
    "systemctl suspend".to_string()
}

fn default_reboot_cmd() -> String {
    "systemctl reboot".to_string()
}

fn default_logout_cmd() -> String {
    "loginctl kill-user $(whoami)".to_string()
}

#[derive(Deserialize, Default, Clone, Debug)]
pub struct SettingsModuleConfig {
    pub lock_cmd: Option<String>,
    #[serde(default = "default_shutdown_cmd")]
    pub shutdown_cmd: String,
    #[serde(default = "default_suspend_cmd")]
    pub suspend_cmd: String,
    #[serde(default = "default_reboot_cmd")]
    pub reboot_cmd: String,
    #[serde(default = "default_logout_cmd")]
    pub logout_cmd: String,
    pub audio_sinks_more_cmd: Option<String>,
    pub audio_sources_more_cmd: Option<String>,
    pub wifi_more_cmd: Option<String>,
    pub vpn_more_cmd: Option<String>,
    pub bluetooth_more_cmd: Option<String>,
    pub remove_airplane_btn: bool,
}

#[derive(Deserialize, Clone, Debug)]
pub struct MediaPlayerModuleConfig {
    #[serde(default = "default_media_player_max_title_length")]
    pub max_title_length: u32,
}

impl Default for MediaPlayerModuleConfig {
    fn default() -> Self {
        MediaPlayerModuleConfig {
            max_title_length: default_media_player_max_title_length(),
        }
    }
}

fn default_media_player_max_title_length() -> u32 {
    100
}

#[derive(Deserialize, Clone, Copy, Debug)]
#[serde(untagged)]
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

#[derive(Deserialize, Default, Copy, Clone, Eq, PartialEq, Debug)]
pub enum AppearanceStyle {
    #[default]
    Islands,
    Solid,
    Gradient,
}

#[derive(Deserialize, Clone, Debug)]
pub struct MenuAppearance {
    #[serde(default = "default_opacity")]
    pub opacity: f32,
    #[serde(default)]
    pub backdrop: f32,
}

impl Default for MenuAppearance {
    fn default() -> Self {
        Self {
            opacity: default_opacity(),
            backdrop: f32::default(),
        }
    }
}

#[derive(Deserialize, Clone, Debug)]
pub struct Appearance {
    #[serde(default)]
    pub font_name: Option<String>,
    #[serde(default)]
    pub style: AppearanceStyle,
    #[serde(default = "default_opacity")]
    pub opacity: f32,
    #[serde(default)]
    pub menu: MenuAppearance,
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

fn default_opacity() -> f32 {
    1.0
}

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
            font_name: None,
            style: AppearanceStyle::default(),
            opacity: default_opacity(),
            menu: MenuAppearance::default(),
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ModuleName {
    AppLauncher,
    Updates,
    Clipboard,
    Workspaces,
    WindowTitle,
    SystemInfo,
    KeyboardLayout,
    KeyboardSubmap,
    Tray,
    Clock,
    Privacy,
    Settings,
    MediaPlayer,
    Custom(String),
}

impl<'de> Deserialize<'de> for ModuleName {
    fn deserialize<D>(deserializer: D) -> Result<ModuleName, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ModuleNameVisitor;
        impl Visitor<'_> for ModuleNameVisitor {
            type Value = ModuleName;
            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a string representing a ModuleName")
            }
            fn visit_str<E>(self, value: &str) -> Result<ModuleName, E>
            where
                E: serde::de::Error,
            {
                Ok(match value {
                    "AppLauncher" => ModuleName::AppLauncher,
                    "Updates" => ModuleName::Updates,
                    "Clipboard" => ModuleName::Clipboard,
                    "Workspaces" => ModuleName::Workspaces,
                    "WindowTitle" => ModuleName::WindowTitle,
                    "SystemInfo" => ModuleName::SystemInfo,
                    "KeyboardLayout" => ModuleName::KeyboardLayout,
                    "KeyboardSubmap" => ModuleName::KeyboardSubmap,
                    "Tray" => ModuleName::Tray,
                    "Clock" => ModuleName::Clock,
                    "Privacy" => ModuleName::Privacy,
                    "Settings" => ModuleName::Settings,
                    "MediaPlayer" => ModuleName::MediaPlayer,
                    other => ModuleName::Custom(other.to_string()),
                })
            }
        }
        deserializer.deserialize_str(ModuleNameVisitor)
    }
}

#[derive(Deserialize, Clone, Debug)]
#[serde(untagged)]
pub enum ModuleDef {
    Single(ModuleName),
    Group(Vec<ModuleName>),
}

#[derive(Deserialize, Clone, Debug)]
pub struct Modules {
    #[serde(default)]
    pub left: Vec<ModuleDef>,
    #[serde(default)]
    pub center: Vec<ModuleDef>,
    #[serde(default)]
    pub right: Vec<ModuleDef>,
}

impl Default for Modules {
    fn default() -> Self {
        Self {
            left: vec![ModuleDef::Single(ModuleName::Workspaces)],
            center: vec![ModuleDef::Single(ModuleName::WindowTitle)],
            right: vec![ModuleDef::Group(vec![
                ModuleName::Clock,
                ModuleName::Privacy,
                ModuleName::Settings,
            ])],
        }
    }
}

#[derive(Deserialize, Clone, Default, Debug, PartialEq, Eq)]
pub enum Outputs {
    #[default]
    All,
    Active,
    #[serde(deserialize_with = "non_empty")]
    Targets(Vec<String>),
}

fn non_empty<'de, D, T>(d: D) -> Result<Vec<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    let vec = <Vec<T>>::deserialize(d)?;
    if vec.is_empty() {
        use serde::de::Error;

        Err(D::Error::custom("need non-empty"))
    } else {
        Ok(vec)
    }
}

/// Newtype wrapper around `Regex`to be deserializable and usable as a hashmap key
#[serde_as]
#[derive(Debug, Clone, Deserialize)]
#[serde(transparent)]
pub struct RegexCfg(#[serde_as(as = "DisplayFromStr")] pub Regex);

impl PartialEq for RegexCfg {
    fn eq(&self, other: &Self) -> bool {
        self.0.as_str() == other.0.as_str()
    }
}
impl Eq for RegexCfg {}

impl std::hash::Hash for RegexCfg {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        // hash the raw pattern string
        self.0.as_str().hash(state);
    }
}

impl Deref for RegexCfg {
    type Target = Regex;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[serde_as]
#[derive(Deserialize, Clone, Debug)]
pub struct CustomModuleDef {
    pub name: String,
    pub command: String,
    #[serde(default)]
    pub icon: Option<String>,

    /// yields json lines containing text, alt, (pot tooltip)
    pub listen_cmd: Option<String>,
    /// map of regex -> icon
    pub icons: Option<HashMap<RegexCfg, String>>,
    /// regex to show alert
    pub alert: Option<RegexCfg>,
    // .. appearance etc
}

#[derive(Deserialize, Clone, Debug)]
pub struct Config {
    #[serde(default = "default_log_level")]
    pub log_level: String,
    #[serde(default)]
    pub position: Position,
    #[serde(default)]
    pub outputs: Outputs,
    #[serde(default)]
    pub modules: Modules,
    pub app_launcher_cmd: Option<String>,
    #[serde(rename = "CustomModule", default)]
    pub custom_modules: Vec<CustomModuleDef>,
    pub clipboard_cmd: Option<String>,
    #[serde(default)]
    pub updates: Option<UpdatesModuleConfig>,
    #[serde(default)]
    pub workspaces: WorkspacesModuleConfig,
    #[serde(default)]
    pub window_title: WindowTitleConfig,
    #[serde(default)]
    pub system: SystemModuleConfig,
    #[serde(default)]
    pub clock: ClockModuleConfig,
    #[serde(default)]
    pub settings: SettingsModuleConfig,
    #[serde(default)]
    pub appearance: Appearance,
    #[serde(default)]
    pub media_player: MediaPlayerModuleConfig,
    #[serde(default)]
    pub keyboard_layout: KeyboardLayoutModuleConfig,
}

fn default_log_level() -> String {
    "warn".to_owned()
}

fn default_truncate_title_after_length() -> u32 {
    150
}

impl Default for Config {
    fn default() -> Self {
        Self {
            log_level: default_log_level(),
            position: Position::Top,
            outputs: Outputs::default(),
            modules: Modules::default(),
            app_launcher_cmd: None,
            clipboard_cmd: None,
            updates: None,
            workspaces: WorkspacesModuleConfig::default(),
            window_title: WindowTitleConfig::default(),
            system: SystemModuleConfig::default(),
            clock: ClockModuleConfig::default(),
            settings: SettingsModuleConfig::default(),
            appearance: Appearance::default(),
            media_player: MediaPlayerModuleConfig::default(),
            keyboard_layout: KeyboardLayoutModuleConfig::default(),
            custom_modules: vec![],
        }
    }
}

pub fn read_config() -> Result<Config, Box<dyn Error + Send>> {
    let home_dir = env::var("HOME").expect("Could not get HOME environment variable");
    let file_path = format!("{}{}", home_dir, CONFIG_PATH.replace('~', ""));

    let mut content = String::new();
    let read_result = File::open(&file_path).and_then(|mut file| file.read_to_string(&mut content));

    match read_result {
        Ok(_) => {
            log::info!("Reading config file");

            toml::from_str(&content).map_err(|e| Box::new(e) as Box<dyn Error + Send>)
        }
        Err(e) => Err(Box::new(e)),
    }
}

pub fn subscription() -> Subscription<Message> {
    let id = TypeId::of::<Config>();

    Subscription::run_with_id(
        id,
        channel(100, async |mut output| {
            let home_dir = env::var("HOME").expect("Could not get HOME environment variable");

            let file_path = format!("{}{}", home_dir, CONFIG_PATH.replace('~', ""));
            let config_file_path = Path::new(&file_path);

            let ashell_config_dir = config_file_path
                .parent()
                .expect("Failed to get ashell config directory");
            let config_dir = ashell_config_dir
                .parent()
                .expect("Failed to get config directory");

            loop {
                let inotify = Inotify::init().expect("Failed to initialize inotify");

                let mut watches = inotify.watches();

                if ashell_config_dir.exists() {
                    watches
                        .add(
                            ashell_config_dir,
                            WatchMask::MOVE
                                | WatchMask::MODIFY
                                | WatchMask::MOVE_SELF
                                | WatchMask::CREATE
                                | WatchMask::DELETE_SELF,
                        )
                        .expect("Failed to add file watch for the ashell config directory");

                    let mut buffer = [0; 1024];
                    let mut stream = inotify
                        .into_event_stream(&mut buffer)
                        .expect("Failed to create event stream");

                    loop {
                        let event = stream.next().await;

                        log::debug!("ashell config folder event: {event:?}");

                        if let Some(Ok(Event { mask, name, .. })) = event {
                            match mask {
                                EventMask::DELETE_SELF | EventMask::MOVE_SELF => {
                                    log::warn!("ashell config directory disappear");

                                    let _ =
                                        output.send(Message::ConfigChanged(Box::default())).await;

                                    break;
                                }
                                _ => {
                                    log::info!("ashell config file events: {name:?}");
                                    if name.is_some_and(|name| name == "config.toml") {
                                        let new_config = read_config();
                                        match new_config {
                                            Ok(new_config) => {
                                                let _ = output
                                                    .send(Message::ConfigChanged(Box::new(
                                                        new_config,
                                                    )))
                                                    .await;
                                            }
                                            Err(err) => {
                                                log::warn!("Failed to read config file: {err:?}");
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                } else {
                    watches
                        .add(config_dir, WatchMask::CREATE | WatchMask::MOVED_TO)
                        .expect("Failed to add file watch for the config directory");

                    let mut buffer = [0; 1024];
                    let mut stream = inotify
                        .into_event_stream(&mut buffer)
                        .expect("Failed to create event stream");

                    let event = stream.next().await;

                    log::debug!("Config folder event: {event:?}");

                    if let Some(Ok(_)) = event {
                        if config_file_path.exists() {
                            log::info!("Config file created");

                            let new_config = read_config();
                            match new_config {
                                Ok(new_config) => {
                                    let _ = output
                                        .send(Message::ConfigChanged(Box::new(new_config)))
                                        .await;
                                }
                                Err(err) => {
                                    log::warn!("Failed to read config file: {err:?}");
                                }
                            }
                        }
                    }
                }
            }
        }),
    )
}
