use crate::app::Message;
use crate::services::upower::PeripheralDeviceKind;
use hex_color::HexColor;
use iced::futures::StreamExt;
use iced::{Color, Subscription, futures::SinkExt, stream::channel, theme::palette};
use inotify::EventMask;
use inotify::Inotify;
use inotify::WatchMask;
use log::{debug, error, info, warn};
use regex::Regex;
use serde::{Deserialize, Deserializer, de::Visitor};
use serde_with::DisplayFromStr;
use serde_with::serde_as;
use std::path::PathBuf;
use std::time::Duration;
use std::{any::TypeId, collections::HashMap, error::Error, ops::Deref, path::Path};
use tokio::time::sleep;

pub const DEFAULT_CONFIG_FILE_PATH: &str = "~/.config/ashell/config.toml";

#[derive(Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Config {
    pub log_level: String,
    pub position: Position,
    pub layer: Layer,
    pub outputs: Outputs,
    pub modules: Modules,
    #[serde(rename = "CustomModule")]
    pub custom_modules: Vec<CustomModuleDef>,
    pub updates: Option<UpdatesModuleConfig>,
    pub workspaces: WorkspacesModuleConfig,
    pub window_title: WindowTitleConfig,
    pub system_info: SystemInfoModuleConfig,
    pub clock: ClockModuleConfig,
    pub settings: SettingsModuleConfig,
    pub appearance: Appearance,
    pub media_player: MediaPlayerModuleConfig,
    pub keyboard_layout: KeyboardLayoutModuleConfig,
    pub enable_esc_key: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            log_level: "warn".to_owned(),
            position: Position::default(),
            layer: Layer::default(),
            outputs: Outputs::default(),
            modules: Modules::default(),
            updates: None,
            workspaces: WorkspacesModuleConfig::default(),
            window_title: WindowTitleConfig::default(),
            system_info: SystemInfoModuleConfig::default(),
            clock: ClockModuleConfig::default(),
            settings: SettingsModuleConfig::default(),
            appearance: Appearance::default(),
            media_player: MediaPlayerModuleConfig::default(),
            keyboard_layout: KeyboardLayoutModuleConfig::default(),
            custom_modules: vec![],
            enable_esc_key: false,
        }
    }
}

#[derive(Deserialize, Clone, Debug)]
pub struct UpdatesModuleConfig {
    pub check_cmd: String,
    pub update_cmd: String,
}

#[derive(Deserialize, Copy, Clone, Default, PartialEq, Eq, Debug)]
pub enum WorkspaceVisibilityMode {
    #[default]
    All,
    MonitorSpecific,
    MonitorSpecificExclusive,
}

#[derive(Deserialize, Clone, Default, Debug)]
#[serde(default)]
pub struct WorkspacesModuleConfig {
    pub visibility_mode: WorkspaceVisibilityMode,
    pub group_by_monitor: bool,
    pub enable_workspace_filling: bool,
    pub disable_special_workspaces: bool,
    pub max_workspaces: Option<u32>,
    pub workspace_names: Vec<String>,
    pub enable_virtual_desktops: bool,
}

#[derive(Deserialize, Copy, Clone, Default, PartialEq, Eq, Debug)]
pub enum WindowTitleMode {
    #[default]
    Title,
    Class,
    InitialTitle,
    InitialClass,
}

#[derive(Deserialize, Copy, Clone, Debug)]
#[serde(default)]
pub struct WindowTitleConfig {
    pub mode: WindowTitleMode,
    pub truncate_title_after_length: u32,
}

impl Default for WindowTitleConfig {
    fn default() -> Self {
        Self {
            mode: Default::default(),
            truncate_title_after_length: 150,
        }
    }
}

#[derive(Deserialize, Clone, Default, Debug)]
#[serde(default)]
pub struct KeyboardLayoutModuleConfig {
    pub labels: HashMap<String, String>,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(default)]
pub struct SystemInfoCpu {
    #[serde(default)]
    pub warn_threshold: u32,
    #[serde(default)]
    pub alert_threshold: u32,
}

impl Default for SystemInfoCpu {
    fn default() -> Self {
        Self {
            warn_threshold: 60,
            alert_threshold: 80,
        }
    }
}

#[derive(Deserialize, Clone, Debug)]
#[serde(default)]
pub struct SystemInfoMemory {
    pub warn_threshold: u32,
    pub alert_threshold: u32,
}

impl Default for SystemInfoMemory {
    fn default() -> Self {
        Self {
            warn_threshold: 70,
            alert_threshold: 85,
        }
    }
}

#[derive(Deserialize, Clone, Debug)]
#[serde(default)]
pub struct SystemInfoTemperature {
    pub warn_threshold: i32,
    pub alert_threshold: i32,
    pub sensor: String,
}

impl Default for SystemInfoTemperature {
    fn default() -> Self {
        Self {
            warn_threshold: 60,
            alert_threshold: 80,
            sensor: "acpitz temp1".to_string(),
        }
    }
}

#[derive(Deserialize, Clone, Debug)]
#[serde(default)]
pub struct SystemInfoDisk {
    pub warn_threshold: u32,
    pub alert_threshold: u32,
}

impl Default for SystemInfoDisk {
    fn default() -> Self {
        Self {
            warn_threshold: 80,
            alert_threshold: 90,
        }
    }
}

#[derive(Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct SystemInfoDiskIndicatorConfig {
    #[serde(rename = "Disk")]
    pub path: String,
    #[serde(rename = "Name")]
    pub name: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub enum SystemInfoIndicator {
    Cpu,
    Memory,
    MemorySwap,
    Temperature,
    IpAddress,
    DownloadSpeed,
    UploadSpeed,
    #[serde(untagged)]
    Disk(SystemInfoDiskIndicatorConfig),
}

#[derive(Deserialize, Clone, Debug)]
#[serde(default)]
pub struct SystemInfoModuleConfig {
    pub indicators: Vec<SystemInfoIndicator>,
    pub cpu: SystemInfoCpu,
    pub memory: SystemInfoMemory,
    pub temperature: SystemInfoTemperature,
    pub disk: SystemInfoDisk,
}

impl Default for SystemInfoModuleConfig {
    fn default() -> Self {
        Self {
            indicators: vec![
                SystemInfoIndicator::Cpu,
                SystemInfoIndicator::Memory,
                SystemInfoIndicator::Temperature,
            ],
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

#[derive(Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum SettingsIndicator {
    IdleInhibitor,
    PowerProfile,
    Audio,
    Network,
    Vpn,
    Bluetooth,
    Battery,
    PeripheralBattery,
}

#[derive(Deserialize, Copy, Clone, Default, PartialEq, Eq, Debug)]
pub enum BatteryFormat {
    Icon,
    Percentage,
    #[default]
    IconAndPercentage,
}

#[derive(Deserialize, Clone, Default, PartialEq, Eq, Debug)]
pub enum PeripheralIndicators {
    #[default]
    All,
    Specific(Vec<PeripheralDeviceKind>),
}

#[derive(Deserialize, Clone, Debug)]
#[serde(default)]
pub struct SettingsModuleConfig {
    pub lock_cmd: Option<String>,
    pub shutdown_cmd: String,
    pub suspend_cmd: String,
    pub hibernate_cmd: String,
    pub reboot_cmd: String,
    pub logout_cmd: String,
    pub battery_format: BatteryFormat,
    pub peripheral_indicators: PeripheralIndicators,
    pub peripheral_battery_format: BatteryFormat,
    pub audio_sinks_more_cmd: Option<String>,
    pub audio_sources_more_cmd: Option<String>,
    pub wifi_more_cmd: Option<String>,
    pub vpn_more_cmd: Option<String>,
    pub bluetooth_more_cmd: Option<String>,
    pub remove_airplane_btn: bool,
    pub remove_idle_btn: bool,
    pub indicators: Vec<SettingsIndicator>,
    #[serde(rename = "CustomButton")]
    pub custom_buttons: Vec<SettingsCustomButton>,
}

impl Default for SettingsModuleConfig {
    fn default() -> Self {
        Self {
            lock_cmd: Default::default(),
            shutdown_cmd: "shutdown now".to_string(),
            suspend_cmd: "systemctl suspend".to_string(),
            hibernate_cmd: "systemctl hibernate".to_string(),
            reboot_cmd: "systemctl reboot".to_string(),
            logout_cmd: "loginctl kill-user $(whoami)".to_string(),
            battery_format: Default::default(),
            peripheral_indicators: Default::default(),
            peripheral_battery_format: BatteryFormat::Icon,
            audio_sinks_more_cmd: Default::default(),
            audio_sources_more_cmd: Default::default(),
            wifi_more_cmd: Default::default(),
            vpn_more_cmd: Default::default(),
            bluetooth_more_cmd: Default::default(),
            remove_airplane_btn: Default::default(),
            remove_idle_btn: Default::default(),
            indicators: vec![
                SettingsIndicator::IdleInhibitor,
                SettingsIndicator::PowerProfile,
                SettingsIndicator::Audio,
                SettingsIndicator::Bluetooth,
                SettingsIndicator::Network,
                SettingsIndicator::Vpn,
                SettingsIndicator::Battery,
            ],
            custom_buttons: Default::default(),
        }
    }
}

#[derive(Deserialize, Clone, Debug)]
pub struct SettingsCustomButton {
    pub name: String,
    pub icon: String,
    pub command: String,
    pub status_command: Option<String>,
    pub tooltip: Option<String>,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(default)]
pub struct MediaPlayerModuleConfig {
    pub max_title_length: u32,
}

impl Default for MediaPlayerModuleConfig {
    fn default() -> Self {
        MediaPlayerModuleConfig {
            max_title_length: 100,
        }
    }
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

#[derive(Deserialize, Clone, Copy, Debug)]
#[serde(default)]
pub struct MenuAppearance {
    #[serde(deserialize_with = "opacity_deserializer")]
    pub opacity: f32,
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
#[serde(default)]
pub struct Appearance {
    pub font_name: Option<String>,
    #[serde(deserialize_with = "scale_factor_deserializer")]
    pub scale_factor: f64,
    pub style: AppearanceStyle,
    #[serde(deserialize_with = "opacity_deserializer")]
    pub opacity: f32,
    pub menu: MenuAppearance,
    pub background_color: AppearanceColor,
    pub primary_color: AppearanceColor,
    pub secondary_color: AppearanceColor,
    pub success_color: AppearanceColor,
    pub danger_color: AppearanceColor,
    pub text_color: AppearanceColor,
    pub workspace_colors: Vec<AppearanceColor>,
    pub special_workspace_colors: Option<Vec<AppearanceColor>>,
}

static PRIMARY: HexColor = HexColor::rgb(250, 179, 135);

fn scale_factor_deserializer<'de, D>(deserializer: D) -> Result<f64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let v = f64::deserialize(deserializer)?;

    if v <= 0.0 {
        return Err(serde::de::Error::custom(
            "Scale factor must be greater than 0.0",
        ));
    }

    if v > 2.0 {
        return Err(serde::de::Error::custom(
            "Scale factor cannot be greater than 2.0",
        ));
    }

    Ok(v)
}

fn opacity_deserializer<'de, D>(deserializer: D) -> Result<f32, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let v = f32::deserialize(deserializer)?;

    if v < 0.0 {
        return Err(serde::de::Error::custom("Opacity cannot be negative"));
    }

    if v > 1.0 {
        return Err(serde::de::Error::custom(
            "Opacity cannot be greater than 1.0",
        ));
    }

    Ok(v)
}

fn default_opacity() -> f32 {
    1.0
}

impl Default for Appearance {
    fn default() -> Self {
        Self {
            font_name: None,
            scale_factor: 1.0,
            style: AppearanceStyle::default(),
            opacity: default_opacity(),
            menu: MenuAppearance::default(),
            background_color: AppearanceColor::Complete {
                base: HexColor::rgb(30, 30, 46),
                strong: Some(HexColor::rgb(69, 71, 90)),
                weak: Some(HexColor::rgb(49, 50, 68)),
                text: None,
            },
            primary_color: AppearanceColor::Complete {
                base: PRIMARY,
                strong: None,
                weak: None,
                text: Some(HexColor::rgb(30, 30, 46)),
            },
            secondary_color: AppearanceColor::Complete {
                base: HexColor::rgb(17, 17, 27),
                strong: Some(HexColor::rgb(24, 24, 37)),
                weak: None,
                text: None,
            },
            success_color: AppearanceColor::Simple(HexColor::rgb(166, 227, 161)),
            danger_color: AppearanceColor::Complete {
                base: HexColor::rgb(243, 139, 168),
                weak: Some(HexColor::rgb(249, 226, 175)),
                strong: None,
                text: None,
            },
            text_color: AppearanceColor::Simple(HexColor::rgb(205, 214, 244)),
            workspace_colors: vec![
                AppearanceColor::Simple(PRIMARY),
                AppearanceColor::Simple(HexColor::rgb(180, 190, 254)),
                AppearanceColor::Simple(HexColor::rgb(203, 166, 247)),
            ],
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

#[derive(Deserialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Layer {
    #[default]
    Bottom,
    Overlay,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ModuleName {
    Updates,
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
                    "Updates" => ModuleName::Updates,
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

pub fn get_config(path: Option<PathBuf>) -> Result<(Config, PathBuf), Box<dyn Error + Send>> {
    match path {
        Some(p) => {
            info!("Config path provided {p:?}");
            expand_path(p).and_then(|expanded| {
                if !expanded.exists() {
                    Err(Box::new(std::io::Error::new(
                        std::io::ErrorKind::NotFound,
                        format!("Config file does not exist: {}", expanded.display()),
                    )))
                } else {
                    Ok((read_config(&expanded).unwrap_or_default(), expanded))
                }
            })
        }
        None => expand_path(PathBuf::from(DEFAULT_CONFIG_FILE_PATH)).map(|expanded| {
            let parent = expanded
                .parent()
                .expect("Failed to get default config parent directory");

            if !parent.exists() {
                std::fs::create_dir_all(parent)
                    .expect("Failed to create default config parent directory");
            }

            (read_config(&expanded).unwrap_or_default(), expanded)
        }),
    }
}

fn expand_path(path: PathBuf) -> Result<PathBuf, Box<dyn Error + Send>> {
    let str_path = path.to_string_lossy();
    let expanded =
        shellexpand::full(&str_path).map_err(|e| Box::new(e) as Box<dyn Error + Send>)?;

    Ok(PathBuf::from(expanded.to_string()))
}

fn read_config(path: &Path) -> Result<Config, Box<dyn Error + Send>> {
    let content =
        std::fs::read_to_string(path).map_err(|e| Box::new(e) as Box<dyn Error + Send>)?;

    info!("Decoding config file {path:?}");

    let res = toml::from_str(&content);

    match res {
        Ok(config) => {
            info!("Config file loaded successfully");
            Ok(config)
        }
        Err(e) => {
            warn!("Failed to parse config file: {e}");
            Err(Box::new(e))
        }
    }
}

enum Event {
    Changed,
    Removed,
}

pub fn subscription(path: &Path) -> Subscription<Message> {
    let id = TypeId::of::<Config>();
    let path = path.to_path_buf();

    Subscription::run_with_id(
        id,
        channel(100, async move |mut output| {
            match (path.parent(), path.file_name(), Inotify::init()) {
                (Some(folder), Some(file_name), Ok(inotify)) => {
                    debug!("Watching config file at {path:?}");

                    let res = inotify.watches().add(
                        folder,
                        WatchMask::CREATE | WatchMask::DELETE | WatchMask::MOVE | WatchMask::MODIFY,
                    );

                    if let Err(e) = res {
                        error!("Failed to add watch for {folder:?}: {e}");
                        return;
                    }

                    let buffer = [0; 1024];
                    let stream = inotify.into_event_stream(buffer);

                    if let Ok(stream) = stream {
                        let mut stream = stream.ready_chunks(10);

                        debug!("Starting config file watch loop");

                        loop {
                            let events = stream.next().await.unwrap_or(vec![]);

                            debug!("Received inotify events: {events:?}");

                            let mut file_event = None;

                            for event in events {
                                debug!("Event: {event:?}");
                                match event {
                                    Ok(inotify::Event {
                                        name: Some(name),
                                        mask: EventMask::DELETE | EventMask::MOVED_FROM,
                                        ..
                                    }) if file_name == name => {
                                        debug!("File deleted or moved");
                                        file_event = Some(Event::Removed);
                                    }
                                    Ok(inotify::Event {
                                        name: Some(name),
                                        mask:
                                            EventMask::CREATE | EventMask::MODIFY | EventMask::MOVED_TO,
                                        ..
                                    }) if file_name == name => {
                                        debug!("File created or moved");

                                        file_event = Some(Event::Changed);
                                    }
                                    _ => {
                                        debug!("Ignoring event");
                                    }
                                }
                            }

                            match file_event {
                                Some(Event::Changed) => {
                                    info!("Reload config file");

                                    let new_config = read_config(&path).unwrap_or_default();

                                    let _ = output
                                        .send(Message::ConfigChanged(Box::new(new_config)))
                                        .await;
                                }
                                Some(Event::Removed) => {
                                    // wait and double check if the file is really gone
                                    sleep(Duration::from_millis(500)).await;

                                    if !path.exists() {
                                        info!("Config file removed");
                                        let _ = output
                                            .send(Message::ConfigChanged(Box::default()))
                                            .await;
                                    }
                                }
                                None => {
                                    debug!("No relevant file event detected.");
                                }
                            }
                        }
                    } else {
                        error!("Failed to create inotify event stream");
                    }
                }
                (None, _, _) => {
                    error!(
                        "Config file path does not have a parent directory, cannot watch for changes"
                    );
                }
                (_, None, _) => {
                    error!("Config file path does not have a file name, cannot watch for changes");
                }
                (_, _, Err(e)) => {
                    error!("Failed to initialize inotify: {e}");
                }
            }
        }),
    )
}
