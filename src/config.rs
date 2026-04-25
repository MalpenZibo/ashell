use crate::app::Message;
use crate::services::upower::PeripheralDeviceKind;
use chrono::Locale;
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
use std::{collections::HashMap, error::Error, ops::Deref, path::Path};
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
    pub notifications: NotificationsModuleConfig,
    pub tray: TrayModuleConfig,
    pub tempo: TempoModuleConfig,
    pub settings: SettingsModuleConfig,
    pub appearance: Appearance,
    pub media_player: MediaPlayerModuleConfig,
    pub keyboard_layout: KeyboardLayoutModuleConfig,
    pub enable_esc_key: bool,
    pub osd: OsdConfig,
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
            notifications: NotificationsModuleConfig::default(),
            tray: TrayModuleConfig::default(),
            tempo: TempoModuleConfig::default(),
            settings: SettingsModuleConfig::default(),
            appearance: Appearance::default(),
            media_player: MediaPlayerModuleConfig::default(),
            keyboard_layout: KeyboardLayoutModuleConfig::default(),
            custom_modules: vec![],
            enable_esc_key: false,
            osd: OsdConfig::default(),
        }
    }
}

impl Config {
    fn validate(&mut self) {
        if let Some(ref mut updates) = self.updates {
            updates.validate();
        }
        self.system_info.validate();
    }
}

#[derive(Deserialize, Clone, Debug)]
pub struct UpdatesModuleConfig {
    pub check_cmd: String,
    pub update_cmd: String,
    #[serde(default = "UpdatesModuleConfig::default_interval")]
    pub interval: u64,
}

impl UpdatesModuleConfig {
    const fn default_interval() -> u64 {
        3600
    }

    fn validate(&mut self) {
        if self.interval == 0 {
            warn!("UpdatesModuleConfig.interval is 0, setting to 1");
            self.interval = 1;
        }
    }
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
    pub invert_scroll_direction: Option<InvertScrollDirection>,
}

#[derive(Deserialize, Copy, Clone, Default, PartialEq, Eq, Debug)]
pub enum InvertScrollDirection {
    #[default]
    All,
    Mouse,
    Trackpad,
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
    pub warn_threshold: u32,
    pub alert_threshold: u32,

    pub format: CpuFormat,
}

fn validate_thresholds<T: PartialOrd + Copy + std::fmt::Display>(
    warn: &mut T,
    alert: &mut T,
    name: &str,
) {
    if *warn >= *alert {
        warn!(
            "{name} warn_threshold ({warn}) >= alert_threshold ({alert}), setting both to {alert}"
        );
        *warn = *alert;
    }
}

impl SystemInfoCpu {
    fn validate(&mut self) {
        validate_thresholds(&mut self.warn_threshold, &mut self.alert_threshold, "CPU");
    }
}

impl Default for SystemInfoCpu {
    fn default() -> Self {
        Self {
            warn_threshold: 60,
            alert_threshold: 80,
            format: CpuFormat::Percentage,
        }
    }
}

#[derive(Deserialize, Clone, Debug)]
#[serde(default)]
pub struct SystemInfoMemory {
    pub warn_threshold: u32,
    pub alert_threshold: u32,
    pub format: MemoryFormat,
}

impl SystemInfoMemory {
    fn validate(&mut self) {
        validate_thresholds(
            &mut self.warn_threshold,
            &mut self.alert_threshold,
            "Memory",
        );
    }
}

impl Default for SystemInfoMemory {
    fn default() -> Self {
        Self {
            warn_threshold: 70,
            alert_threshold: 85,
            format: MemoryFormat::Percentage,
        }
    }
}

const DEFAULT_TEMP_WARN_CELSIUS: i32 = 60;
const DEFAULT_TEMP_ALERT_CELSIUS: i32 = 80;

#[derive(Deserialize, Clone, Debug)]
#[serde(default)]
pub struct SystemInfoTemperature {
    warn_threshold: Option<i32>,
    alert_threshold: Option<i32>,
    pub sensor: String,
    pub format: TemperatureFormat,
}

impl SystemInfoTemperature {
    pub fn warn_threshold(&self) -> i32 {
        self.warn_threshold.unwrap_or_else(|| match self.format {
            TemperatureFormat::Celsius => DEFAULT_TEMP_WARN_CELSIUS,
            TemperatureFormat::Fahrenheit => celsius_to_fahrenheit(DEFAULT_TEMP_WARN_CELSIUS),
        })
    }

    pub fn alert_threshold(&self) -> i32 {
        self.alert_threshold.unwrap_or_else(|| match self.format {
            TemperatureFormat::Celsius => DEFAULT_TEMP_ALERT_CELSIUS,
            TemperatureFormat::Fahrenheit => celsius_to_fahrenheit(DEFAULT_TEMP_ALERT_CELSIUS),
        })
    }
}

impl SystemInfoTemperature {
    fn validate(&mut self) {
        if let (Some(warn), Some(alert)) = (&mut self.warn_threshold, &mut self.alert_threshold) {
            validate_thresholds(warn, alert, "Temperature");
        }
    }
}

impl Default for SystemInfoTemperature {
    fn default() -> Self {
        Self {
            warn_threshold: None,
            alert_threshold: None,
            sensor: "acpitz temp1".to_string(),
            format: TemperatureFormat::Celsius,
        }
    }
}

fn celsius_to_fahrenheit(cel: i32) -> i32 {
    cel * 9 / 5 + 32
}

#[derive(Clone, Debug, Deserialize, Default)]
pub enum DiskFormat {
    #[default]
    Percentage,
    Fraction,
}

#[derive(Clone, Debug, Deserialize, Default)]
pub enum MemoryFormat {
    #[default]
    Percentage,
    Fraction,
}

#[derive(Clone, Debug, Deserialize, Default)]
pub enum CpuFormat {
    #[default]
    Percentage,
    Frequency,
}

#[derive(Clone, Debug, Deserialize, Default)]
pub enum TemperatureFormat {
    #[default]
    Celsius,
    Fahrenheit,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(default)]
pub struct SystemInfoDisk {
    pub warn_threshold: u32,
    pub alert_threshold: u32,
    pub format: DiskFormat,
    pub deduplicate: bool,
}

impl SystemInfoDisk {
    fn validate(&mut self) {
        validate_thresholds(&mut self.warn_threshold, &mut self.alert_threshold, "Disk");
    }
}

impl Default for SystemInfoDisk {
    fn default() -> Self {
        Self {
            warn_threshold: 80,
            alert_threshold: 90,
            format: DiskFormat::Percentage,
            deduplicate: true,
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
    #[serde(default = "SystemInfoModuleConfig::default_interval")]
    pub interval: u64,
    pub cpu: SystemInfoCpu,
    pub memory: SystemInfoMemory,
    pub temperature: SystemInfoTemperature,
    pub disk: SystemInfoDisk,
}

impl SystemInfoModuleConfig {
    const fn default_interval() -> u64 {
        5
    }

    fn validate(&mut self) {
        if self.interval == 0 {
            warn!("SystemInfoModuleConfig.interval is 0, setting to 1");
            self.interval = 1;
        }
        self.cpu.validate();
        self.memory.validate();
        self.temperature.validate();
        self.disk.validate();
    }
}

impl Default for SystemInfoModuleConfig {
    fn default() -> Self {
        Self {
            indicators: vec![
                SystemInfoIndicator::Cpu,
                SystemInfoIndicator::Memory,
                SystemInfoIndicator::Temperature,
            ],
            interval: Self::default_interval(),
            cpu: SystemInfoCpu::default(),
            memory: SystemInfoMemory::default(),
            temperature: SystemInfoTemperature::default(),
            disk: SystemInfoDisk::default(),
        }
    }
}

fn deserialize_locale<'de, D>(deserializer: D) -> Result<Locale, D::Error>
where
    D: Deserializer<'de>,
{
    let s: String = Deserialize::deserialize(deserializer)?;
    Ok(Locale::try_from(s.as_str()).unwrap_or(Locale::en_US))
}
#[derive(Deserialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ToastPosition {
    TopLeft,
    #[default]
    TopRight,
    BottomLeft,
    BottomRight,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(default)]
pub struct NotificationsModuleConfig {
    pub format: String,
    pub show_timestamps: bool,
    pub show_bodies: bool,
    pub grouped: bool,
    pub toast: bool,
    pub toast_position: ToastPosition,
    pub toast_timeout: u64,
    pub toast_limit: usize,
    pub toast_max_height: u32,
    pub blocklist: Vec<RegexCfg>,
}
impl Default for NotificationsModuleConfig {
    fn default() -> Self {
        Self {
            format: "%H:%M".to_string(),
            show_timestamps: true,
            show_bodies: true,
            grouped: false,
            toast: true,
            toast_position: ToastPosition::default(),
            toast_timeout: 5000,
            toast_limit: 5,
            toast_max_height: 150,
            blocklist: vec![],
        }
    }
}

#[derive(Deserialize, Clone, Debug, Default)]
pub struct TrayModuleConfig {
    pub blocklist: Vec<RegexCfg>,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(default)]
pub struct TempoModuleConfig {
    pub clock_format: String,
    #[serde(default)]
    pub formats: Vec<String>,
    #[serde(default)]
    pub timezones: Vec<String>,
    #[serde(default)]
    pub weather_location: Option<WeatherLocation>,
    pub weather_indicator: WeatherIndicator,
    #[serde(deserialize_with = "deserialize_locale")]
    pub locale: Locale,
}

#[derive(Deserialize, Default, Clone, Debug, PartialEq, Eq)]
pub enum WeatherIndicator {
    #[default]
    IconAndTemperature,
    Icon,
    None,
}

#[derive(Deserialize, Default, Clone, Debug, PartialEq)]
pub enum WeatherLocation {
    #[default]
    Current,
    City(String),
    Coordinates(f32, f32),
}

impl std::hash::Hash for WeatherLocation {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::mem::discriminant(self).hash(state);
        match self {
            WeatherLocation::Current => {}
            WeatherLocation::City(city) => city.hash(state),
            WeatherLocation::Coordinates(lat, lon) => {
                lat.to_bits().hash(state);
                lon.to_bits().hash(state);
            }
        }
    }
}

impl Default for TempoModuleConfig {
    fn default() -> Self {
        Self {
            clock_format: "%a %d %b %R".to_string(),
            formats: vec![],
            timezones: vec![],
            weather_location: None,
            weather_indicator: WeatherIndicator::IconAndTemperature,
            locale: Locale::en_US,
        }
    }
}

#[derive(Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum SettingsIndicator {
    IdleInhibitor,
    PowerProfile,
    Audio,
    Microphone,
    Network,
    Vpn,
    Bluetooth,
    Battery,
    PeripheralBattery,
    Brightness,
}

#[derive(Deserialize, Copy, Clone, Default, PartialEq, Eq, Debug)]
pub enum SettingsFormat {
    Icon,
    #[serde(alias = "Value")]
    Percentage,
    #[default]
    #[serde(alias = "IconAndValue")]
    IconAndPercentage,
    Time,
    IconAndTime,
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
    pub battery_format: SettingsFormat,
    pub battery_hide_when_full: bool,
    pub peripheral_indicators: PeripheralIndicators,
    pub peripheral_battery_format: SettingsFormat,
    pub peripheral_expanded_by_default: bool,
    pub audio_indicator_format: SettingsFormat,
    pub microphone_indicator_format: SettingsFormat,
    pub network_indicator_format: SettingsFormat,
    pub bluetooth_indicator_format: SettingsFormat,
    pub brightness_indicator_format: SettingsFormat,
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
            battery_format: SettingsFormat::IconAndPercentage,
            battery_hide_when_full: false,
            peripheral_indicators: Default::default(),
            peripheral_battery_format: SettingsFormat::Icon,
            peripheral_expanded_by_default: false,
            audio_indicator_format: SettingsFormat::Icon,
            microphone_indicator_format: SettingsFormat::Icon,
            network_indicator_format: SettingsFormat::Icon,
            bluetooth_indicator_format: SettingsFormat::Icon,
            brightness_indicator_format: SettingsFormat::Icon,
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
                SettingsIndicator::Microphone,
                SettingsIndicator::Bluetooth,
                SettingsIndicator::Network,
                SettingsIndicator::Vpn,
                SettingsIndicator::Battery,
                SettingsIndicator::Brightness,
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

#[derive(Deserialize, Copy, Clone, Default, PartialEq, Eq, Debug)]
pub enum MediaPlayerFormat {
    Icon,
    #[default]
    IconAndTitle,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(default)]
pub struct MediaPlayerModuleConfig {
    pub max_title_length: u32,
    pub indicator_format: MediaPlayerFormat,
}

impl Default for MediaPlayerModuleConfig {
    fn default() -> Self {
        MediaPlayerModuleConfig {
            max_title_length: 100,
            indicator_format: MediaPlayerFormat::default(),
        }
    }
}

fn hex_to_color(hex: HexColor) -> Color {
    Color::from_rgb8(hex.r, hex.g, hex.b)
}

fn hex_to_pair(hex: HexColor, text: Option<HexColor>, text_fallback: Color) -> palette::Pair {
    palette::Pair::new(
        hex_to_color(hex),
        text.map(hex_to_color).unwrap_or(text_fallback),
    )
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
            AppearanceColor::Simple(color) => hex_to_color(*color),
            AppearanceColor::Complete { base, .. } => hex_to_color(*base),
        }
    }

    pub fn get_text(&self) -> Option<Color> {
        match self {
            AppearanceColor::Simple(_) => None,
            AppearanceColor::Complete { text, .. } => text.map(hex_to_color),
        }
    }

    pub fn get_weak_pair(&self, text_fallback: Color) -> Option<palette::Pair> {
        match self {
            AppearanceColor::Simple(_) => None,
            AppearanceColor::Complete { weak, text, .. } => {
                weak.map(|color| hex_to_pair(color, *text, text_fallback))
            }
        }
    }

    pub fn get_strong_pair(&self, text_fallback: Color) -> Option<palette::Pair> {
        match self {
            AppearanceColor::Simple(_) => None,
            AppearanceColor::Complete { strong, text, .. } => {
                strong.map(|color| hex_to_pair(color, *text, text_fallback))
            }
        }
    }
}

#[derive(Deserialize, Clone, Copy, Debug)]
#[serde(untagged)]
pub enum BackgroundAppearanceColor {
    Simple(HexColor),
    Complete {
        base: HexColor,
        weakest: Option<HexColor>,
        weaker: Option<HexColor>,
        weak: Option<HexColor>,
        neutral: Option<HexColor>,
        strong: Option<HexColor>,
        stronger: Option<HexColor>,
        strongest: Option<HexColor>,
        text: Option<HexColor>,
    },
}

impl BackgroundAppearanceColor {
    pub fn get_base(&self) -> Color {
        match self {
            BackgroundAppearanceColor::Simple(color) => hex_to_color(*color),
            BackgroundAppearanceColor::Complete { base, .. } => hex_to_color(*base),
        }
    }

    pub fn get_text(&self) -> Option<Color> {
        match self {
            BackgroundAppearanceColor::Simple(_) => None,
            BackgroundAppearanceColor::Complete { text, .. } => text.map(hex_to_color),
        }
    }

    pub fn get_pair(&self, level: BackgroundLevel, text_fallback: Color) -> Option<palette::Pair> {
        match self {
            BackgroundAppearanceColor::Simple(_) => None,
            BackgroundAppearanceColor::Complete {
                weakest,
                weaker,
                weak,
                neutral,
                strong,
                stronger,
                strongest,
                text,
                ..
            } => {
                let hex = match level {
                    BackgroundLevel::Weakest => *weakest,
                    BackgroundLevel::Weaker => *weaker,
                    BackgroundLevel::Weak => *weak,
                    BackgroundLevel::Neutral => *neutral,
                    BackgroundLevel::Strong => *strong,
                    BackgroundLevel::Stronger => *stronger,
                    BackgroundLevel::Strongest => *strongest,
                };
                hex.map(|h| hex_to_pair(h, *text, text_fallback))
            }
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum BackgroundLevel {
    Weakest,
    Weaker,
    Weak,
    Neutral,
    Strong,
    Stronger,
    Strongest,
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
    pub background_color: BackgroundAppearanceColor,
    pub primary_color: AppearanceColor,
    pub success_color: AppearanceColor,
    pub warning_color: AppearanceColor,
    pub danger_color: AppearanceColor,
    pub text_color: AppearanceColor,
    pub workspace_colors: Vec<AppearanceColor>,
    pub special_workspace_colors: Option<Vec<AppearanceColor>>,
}

static PRIMARY: HexColor = HexColor::rgb(122, 162, 247);

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
            background_color: BackgroundAppearanceColor::Complete {
                base: HexColor::rgb(26, 27, 38),
                weakest: None,
                weaker: None,
                weak: Some(HexColor::rgb(36, 39, 58)),
                neutral: None,
                strong: Some(HexColor::rgb(65, 72, 104)),
                stronger: None,
                strongest: None,
                text: None,
            },
            primary_color: AppearanceColor::Simple(PRIMARY),
            success_color: AppearanceColor::Simple(HexColor::rgb(158, 206, 106)),
            warning_color: AppearanceColor::Simple(HexColor::rgb(224, 175, 104)),
            danger_color: AppearanceColor::Simple(HexColor::rgb(247, 118, 142)),
            text_color: AppearanceColor::Simple(HexColor::rgb(169, 177, 214)),
            workspace_colors: vec![
                AppearanceColor::Simple(PRIMARY),
                AppearanceColor::Simple(HexColor::rgb(158, 206, 106)),
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
    Top,
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
    Tempo,
    Privacy,
    Settings,
    MediaPlayer,
    Custom(String),
    Notifications,
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
                    "Notifications" => ModuleName::Notifications,
                    "Tempo" => ModuleName::Tempo,
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
                ModuleName::Tempo,
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

#[derive(Deserialize, Copy, Clone, Default, PartialEq, Eq, Debug)]
pub enum CustomModuleType {
    #[default]
    Button,
    Text,
}

#[serde_as]
#[derive(Deserialize, Clone, Debug)]
pub struct CustomModuleDef {
    pub name: String,
    #[serde(default)]
    pub command: Option<String>,
    #[serde(default)]
    pub icon: Option<String>,

    /// yields json lines containing text, alt, (pot tooltip)
    pub listen_cmd: Option<String>,
    /// map of regex -> icon
    pub icons: Option<HashMap<RegexCfg, String>>,
    /// regex to show alert
    pub alert: Option<RegexCfg>,
    /// Display type: Button (clickable) or Text (display only)
    #[serde(default)]
    pub r#type: CustomModuleType,
    // .. appearance etc
}

#[derive(Deserialize, Clone, Debug)]
#[serde(default)]
pub struct OsdConfig {
    pub enabled: bool,
    pub timeout: u64,
}

impl Default for OsdConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            timeout: 1500,
        }
    }
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
            // Safety: DEFAULT_CONFIG_FILE_PATH is "~/.config/ashell/config.toml" which
            // always has directory components. shellexpand only expands ~/$HOME and never
            // strips path components, so .parent() always returns Some.
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
            let mut config: Config = config;
            config.validate();
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
    let path = path.to_path_buf();

    Subscription::run_with(path, |path| {
        let path = std::fs::canonicalize(path).unwrap_or_else(|_| path.clone());
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
        })
    })
}
