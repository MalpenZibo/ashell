use std::collections::HashMap;
use std::path::Path;

use guido::prelude::Color;
use hex_color::HexColor;
use serde::{Deserialize, Deserializer, de::Visitor};

// ---------------------------------------------------------------------------
// Top-level Config
// ---------------------------------------------------------------------------

#[derive(Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Config {
    pub log_level: String,
    pub language: Option<String>,
    pub region: Option<String>,
    pub position: Position,
    pub outputs: Outputs,
    pub modules: Modules,
    pub updates: Option<UpdatesModuleConfig>,
    pub workspaces: WorkspacesModuleConfig,
    pub window_title: WindowTitleConfig,
    pub system_info: SystemInfoModuleConfig,
    pub clock: ClockModuleConfig,
    pub tempo: TempoModuleConfig,
    pub settings: SettingsModuleConfig,
    pub appearance: Appearance,
    pub media_player: MediaPlayerModuleConfig,
    pub keyboard_layout: KeyboardLayoutModuleConfig,
    pub notifications: NotificationsModuleConfig,
    #[serde(rename = "CustomModule")]
    pub custom_modules: Vec<CustomModuleDef>,
    pub osd: OsdConfig,
    pub enable_esc_key: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            log_level: "warn".to_owned(),
            language: None,
            region: None,
            position: Position::default(),
            outputs: Outputs::default(),
            modules: Modules::default(),
            updates: None,
            workspaces: WorkspacesModuleConfig::default(),
            window_title: WindowTitleConfig::default(),
            system_info: SystemInfoModuleConfig::default(),
            clock: ClockModuleConfig::default(),
            tempo: TempoModuleConfig::default(),
            settings: SettingsModuleConfig::default(),
            appearance: Appearance::default(),
            media_player: MediaPlayerModuleConfig::default(),
            keyboard_layout: KeyboardLayoutModuleConfig::default(),
            notifications: NotificationsModuleConfig::default(),
            custom_modules: Vec::new(),
            osd: OsdConfig::default(),
            enable_esc_key: false,
        }
    }
}

// ---------------------------------------------------------------------------
// Position
// ---------------------------------------------------------------------------

#[derive(Deserialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Position {
    #[default]
    Top,
    Bottom,
}

// ---------------------------------------------------------------------------
// Outputs
// ---------------------------------------------------------------------------

/// Which outputs get a bar: every one, the compositor-chosen active one, or
/// those matching a connector name / EDID substring.
#[derive(Deserialize, Clone, Default, Debug, PartialEq, Eq)]
pub enum Outputs {
    #[default]
    All,
    Active,
    Targets(Vec<String>),
}

// ---------------------------------------------------------------------------
// Modules layout
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ModuleName {
    Updates,
    Workspaces,
    WindowTitle,
    SystemInfo,
    KeyboardLayout,
    KeyboardSubmap,
    Tray,
    Clock,
    Tempo,
    Privacy,
    Settings,
    MediaPlayer,
    Notifications,
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
                    "Tempo" => ModuleName::Tempo,
                    "Privacy" => ModuleName::Privacy,
                    "Notifications" => ModuleName::Notifications,
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
            left: vec![ModuleDef::Group(vec![
                ModuleName::Updates,
                ModuleName::Workspaces,
            ])],
            center: vec![ModuleDef::Single(ModuleName::WindowTitle)],
            right: vec![ModuleDef::Group(vec![
                ModuleName::Settings,
                ModuleName::SystemInfo,
                ModuleName::Clock,
            ])],
        }
    }
}

// ---------------------------------------------------------------------------
// Updates
// ---------------------------------------------------------------------------

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
}

// ---------------------------------------------------------------------------
// Workspaces
// ---------------------------------------------------------------------------

#[derive(Deserialize, Copy, Clone, Default, PartialEq, Eq, Debug)]
pub enum WorkspaceVisibilityMode {
    #[default]
    All,
    MonitorSpecific,
    MonitorSpecificExclusive,
}

#[derive(Deserialize, Copy, Clone, Default, PartialEq, Eq, Debug)]
pub enum InvertScrollDirection {
    #[default]
    All,
    Mouse,
    Trackpad,
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

// ---------------------------------------------------------------------------
// Window title
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// System info
// ---------------------------------------------------------------------------

#[derive(Deserialize, Clone, Debug)]
#[serde(default)]
pub struct SystemInfoCpu {
    pub warn_threshold: u32,
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

// ---------------------------------------------------------------------------
// Clock
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Tempo
// ---------------------------------------------------------------------------

#[derive(Deserialize, Default, Clone, Debug, PartialEq)]
pub enum WeatherLocation {
    #[default]
    Current,
    City(String),
    Coordinates(f32, f32),
}

#[derive(Deserialize, Default, Clone, Copy, Debug, PartialEq, Eq)]
pub enum WeatherIndicator {
    #[default]
    IconAndTemperature,
    Icon,
    None,
}

#[derive(Deserialize, Default, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WindSpeedUnit {
    #[default]
    Kmh,
    Mph,
    Ms,
}

impl WindSpeedUnit {
    pub fn symbol(&self) -> &'static str {
        match self {
            WindSpeedUnit::Kmh => "km/h",
            WindSpeedUnit::Mph => "mph",
            WindSpeedUnit::Ms => "m/s",
        }
    }

    pub fn api_param(&self) -> &'static str {
        match self {
            WindSpeedUnit::Kmh => "kmh",
            WindSpeedUnit::Mph => "mph",
            WindSpeedUnit::Ms => "ms",
        }
    }
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
    pub wind_speed_unit: Option<WindSpeedUnit>,
}

impl TempoModuleConfig {
    pub fn resolved_wind_speed_unit(&self) -> WindSpeedUnit {
        match self.wind_speed_unit {
            Some(unit) => unit,
            None => match crate::i18n::unit_system() {
                crate::i18n::UnitSystem::Imperial => WindSpeedUnit::Mph,
                crate::i18n::UnitSystem::Metric => WindSpeedUnit::Kmh,
            },
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
            weather_indicator: WeatherIndicator::default(),
            wind_speed_unit: None,
        }
    }
}

// ---------------------------------------------------------------------------
// Settings
// ---------------------------------------------------------------------------

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
pub struct SettingsModuleConfig {
    #[serde(default, deserialize_with = "empty_string_as_none")]
    pub lock_cmd: Option<String>,
    pub shutdown_cmd: String,
    pub suspend_cmd: String,
    #[serde(default, deserialize_with = "empty_string_as_none")]
    pub hibernate_cmd: Option<String>,
    pub reboot_cmd: String,
    pub logout_cmd: String,
    pub battery_format: SettingsFormat,
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
    pub volume_step: u8,
    pub max_volume: u8,
    pub indicators: Vec<SettingsIndicator>,
    #[serde(rename = "CustomButton")]
    pub custom_buttons: Vec<SettingsCustomButton>,
}

impl Default for SettingsModuleConfig {
    fn default() -> Self {
        Self {
            lock_cmd: None,
            shutdown_cmd: "shutdown now".to_string(),
            suspend_cmd: "systemctl suspend".to_string(),
            hibernate_cmd: None,
            reboot_cmd: "systemctl reboot".to_string(),
            logout_cmd: "loginctl kill-user $(whoami)".to_string(),
            battery_format: SettingsFormat::IconAndPercentage,
            audio_indicator_format: SettingsFormat::Icon,
            microphone_indicator_format: SettingsFormat::Icon,
            network_indicator_format: SettingsFormat::Icon,
            bluetooth_indicator_format: SettingsFormat::Icon,
            brightness_indicator_format: SettingsFormat::Icon,
            audio_sinks_more_cmd: None,
            audio_sources_more_cmd: None,
            wifi_more_cmd: None,
            vpn_more_cmd: None,
            bluetooth_more_cmd: None,
            remove_airplane_btn: false,
            remove_idle_btn: false,
            volume_step: 5,
            max_volume: 100,
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
            custom_buttons: Vec::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Media player
// ---------------------------------------------------------------------------

#[derive(Deserialize, Copy, Clone, Default, PartialEq, Eq, Debug)]
pub enum MediaPlayerFormat {
    Icon,
    #[serde(alias = "Title")]
    Text,
    #[default]
    #[serde(alias = "IconAndTitle")]
    IconAndText,
}

#[derive(Deserialize, Copy, Clone, PartialEq, Eq, Debug)]
pub enum MediaPlayerTextField {
    Artist,
    Title,
    Album,
}

#[derive(Deserialize, Copy, Clone, PartialEq, Eq, Debug)]
pub enum MediaPlayerVisualizer {
    Background,
    Before,
    After,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(default)]
pub struct MediaPlayerModuleConfig {
    pub indicator_format: MediaPlayerFormat,
    pub indicator_fields: Vec<MediaPlayerTextField>,
    pub max_text_length: u32,
    pub indicator_visualizer: Option<MediaPlayerVisualizer>,
    pub menu_visualizer: bool,
    pub visualizer_framerate: u32,
}

impl Default for MediaPlayerModuleConfig {
    fn default() -> Self {
        Self {
            indicator_format: MediaPlayerFormat::default(),
            indicator_fields: vec![MediaPlayerTextField::Artist, MediaPlayerTextField::Title],
            max_text_length: 100,
            indicator_visualizer: None,
            menu_visualizer: false,
            visualizer_framerate: 30,
        }
    }
}

// ---------------------------------------------------------------------------
// Keyboard layout
// ---------------------------------------------------------------------------

#[derive(Deserialize, Clone, Default, Debug)]
#[serde(default)]
pub struct KeyboardLayoutModuleConfig {
    pub labels: HashMap<String, String>,
}

// ---------------------------------------------------------------------------
// Notifications
// ---------------------------------------------------------------------------

/// A regex in config (matched against notification/tray app names).
#[derive(Clone, Debug)]
pub struct RegexCfg(pub regex::Regex);

impl<'de> Deserialize<'de> for RegexCfg {
    fn deserialize<D>(d: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(d)?;
        regex::Regex::new(&s)
            .map(RegexCfg)
            .map_err(serde::de::Error::custom)
    }
}

impl PartialEq for RegexCfg {
    fn eq(&self, other: &Self) -> bool {
        self.0.as_str() == other.0.as_str()
    }
}
impl Eq for RegexCfg {}

impl std::hash::Hash for RegexCfg {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.as_str().hash(state);
    }
}

fn empty_string_as_none<'de, D>(d: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let opt = Option::<String>::deserialize(d)?;
    Ok(opt.filter(|s| !s.is_empty()))
}

#[derive(Deserialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CustomModuleType {
    #[default]
    Button,
    Text,
}

#[derive(Deserialize, Clone, Debug)]
pub struct CustomModuleDef {
    pub name: String,
    #[serde(default, deserialize_with = "empty_string_as_none")]
    pub command: Option<String>,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default, deserialize_with = "empty_string_as_none")]
    pub listen_cmd: Option<String>,
    #[serde(default)]
    pub icons: Option<HashMap<RegexCfg, String>>,
    #[serde(default)]
    pub alert: Option<RegexCfg>,
    #[serde(rename = "type", default)]
    pub r#type: CustomModuleType,
    #[serde(default, deserialize_with = "empty_string_as_none")]
    pub on_right_click: Option<String>,
    #[serde(default, deserialize_with = "empty_string_as_none")]
    pub on_middle_click: Option<String>,
    #[serde(default, deserialize_with = "empty_string_as_none")]
    pub on_scroll_up: Option<String>,
    #[serde(default, deserialize_with = "empty_string_as_none")]
    pub on_scroll_down: Option<String>,
}

#[derive(Deserialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
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
            blocklist: Vec::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// OSD
// ---------------------------------------------------------------------------

#[derive(Deserialize, Clone, Copy, Debug)]
#[serde(default)]
pub struct OsdConfig {
    pub enabled: bool,
    pub timeout: u64,
    pub show_volume_percentage: bool,
    pub show_brightness_percentage: bool,
}

impl Default for OsdConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            timeout: 1500,
            show_volume_percentage: false,
            show_brightness_percentage: false,
        }
    }
}

// ---------------------------------------------------------------------------
// Appearance
// ---------------------------------------------------------------------------

#[derive(Deserialize, Default, Copy, Clone, Eq, PartialEq, Debug)]
pub enum AppearanceStyle {
    #[default]
    Islands,
    Solid,
    Gradient,
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
    pub fn base(&self) -> Color {
        match self {
            AppearanceColor::Simple(c) => hex_to_color(*c),
            AppearanceColor::Complete { base, .. } => hex_to_color(*base),
        }
    }

    pub fn weak(&self) -> Option<Color> {
        match self {
            AppearanceColor::Simple(_) => None,
            AppearanceColor::Complete { weak, .. } => weak.map(hex_to_color),
        }
    }

    pub fn strong(&self) -> Option<Color> {
        match self {
            AppearanceColor::Simple(_) => None,
            AppearanceColor::Complete { strong, .. } => strong.map(hex_to_color),
        }
    }

    pub fn text(&self) -> Option<Color> {
        match self {
            AppearanceColor::Simple(_) => None,
            AppearanceColor::Complete { text, .. } => text.map(hex_to_color),
        }
    }
}

fn hex_to_color(c: HexColor) -> Color {
    Color::rgb(c.r as f32 / 255.0, c.g as f32 / 255.0, c.b as f32 / 255.0)
}

#[derive(Deserialize, Clone, Copy, Debug)]
#[serde(default)]
pub struct MenuAppearance {
    pub opacity: f32,
    pub backdrop: f32,
}

impl Default for MenuAppearance {
    fn default() -> Self {
        Self {
            opacity: 1.0,
            backdrop: 0.0,
        }
    }
}

static PRIMARY: HexColor = HexColor::rgb(250, 179, 135);

#[derive(Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Appearance {
    pub font_name: Option<String>,
    pub scale_factor: f64,
    pub style: AppearanceStyle,
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

impl Default for Appearance {
    fn default() -> Self {
        Self {
            font_name: None,
            scale_factor: 1.0,
            style: AppearanceStyle::default(),
            opacity: 1.0,
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

// ---------------------------------------------------------------------------
// Loading
// ---------------------------------------------------------------------------

pub fn load_config(path: &Path) -> Config {
    match std::fs::read_to_string(path) {
        Ok(content) => match toml::from_str(&content) {
            Ok(config) => {
                log::info!("Config loaded from {path:?}");
                config
            }
            Err(e) => {
                log::warn!("Failed to parse config file {path:?}: {e}");
                Config::default()
            }
        },
        Err(_) => {
            log::info!("No config file at {path:?}, using defaults");
            Config::default()
        }
    }
}
