use serde::Deserialize;

use crate::modules::ModuleId;

/// When an item shows, if it has anything to show.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Show {
    Always,
    /// In the states its module calls worth showing.
    Notable,
    Never,
    /// In any of these states of its own, by their names in its module.
    States(Vec<&'static str>),
}

impl Show {
    /// Whether an item shows, `is` saying whether it is in a state and
    /// `notable` naming those it is worth showing in: an item with none is
    /// always worth it.
    pub fn admits(&self, notable: &[&str], is: impl Fn(&str) -> bool) -> bool {
        match self {
            Show::Always => true,
            Show::Notable => notable.is_empty() || notable.iter().any(|state| is(state)),
            Show::Never => false,
            Show::States(states) => states.iter().any(|state| is(state)),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Panel {
    Settings,
    Calendar,
    Notifications,
    Media,
    System,
    Tray,
    Updates,
    Launcher,
}

impl Panel {
    pub const ALL: [Panel; 8] = [
        Panel::Settings,
        Panel::Calendar,
        Panel::Notifications,
        Panel::Media,
        Panel::System,
        Panel::Tray,
        Panel::Updates,
        Panel::Launcher,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Panel::Settings => "settings",
            Panel::Calendar => "calendar",
            Panel::Notifications => "notifications",
            Panel::Media => "media",
            Panel::System => "system",
            Panel::Tray => "tray",
            Panel::Updates => "updates",
            Panel::Launcher => "launcher",
        }
    }

    /// The module that draws it, `None` while none does.
    pub fn module(self) -> Option<ModuleId> {
        match self {
            Panel::Calendar => Some(ModuleId::Tempo),
            Panel::Settings => Some(ModuleId::Settings),
            _ => None,
        }
    }
}

/// `"none"`, a panel by its name, or `"config"`, the config window.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(try_from = "String")]
pub enum Open {
    Nothing,
    Panel(Panel),
    Config,
}

impl TryFrom<String> for Open {
    type Error = String;

    fn try_from(name: String) -> Result<Self, String> {
        Open::parse(&name).ok_or_else(|| {
            let panels: Vec<_> = Panel::ALL.iter().map(|panel| panel.name()).collect();
            format!(
                "\"{name}\": expected \"none\", \"config\" or a panel, one of {}",
                panels.join(", ")
            )
        })
    }
}

impl Open {
    pub fn parse(name: &str) -> Option<Self> {
        match name {
            "none" => Some(Open::Nothing),
            "config" => Some(Open::Config),
            name => Panel::ALL
                .into_iter()
                .find(|panel| panel.name() == name)
                .map(Open::Panel),
        }
    }
}

/// `-` for an item no module draws yet.
macro_rules! drawn_by {
    (-) => {
        None
    };
    ($module:ident) => {
        Some(ModuleId::$module)
    };
}

/// `-` for an item whose click is left to its zone.
macro_rules! opens {
    (-) => {
        None
    };
    (Config) => {
        Some(Open::Config)
    };
    ($panel:ident) => {
        Some(Open::Panel(Panel::$panel))
    };
}

macro_rules! items {
    ($($variant:ident = $name:literal, $show:ident, $open:tt, $module:tt;)*) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        pub enum ItemId {
            $($variant,)*
        }

        impl ItemId {
            pub const ALL: &[ItemId] = &[$(ItemId::$variant,)*];

            pub fn name(self) -> &'static str {
                match self {
                    $(ItemId::$variant => $name,)*
                }
            }

            pub fn default_show(self) -> Show {
                match self {
                    $(ItemId::$variant => Show::$show,)*
                }
            }

            pub fn default_open(self) -> Option<Open> {
                match self {
                    $(ItemId::$variant => opens!($open),)*
                }
            }

            /// The module that draws it, `None` while none does.
            pub fn module(self) -> Option<ModuleId> {
                match self {
                    $(ItemId::$variant => drawn_by!($module),)*
                }
            }
        }
    };
}

items! {
    Workspaces = "workspaces", Always, -, -;
    WorkspaceList = "workspace_list", Always, -, -;
    Window = "window", Notable, -, -;
    WindowList = "window_list", Notable, -, -;
    LauncherButton = "launcher_button", Always, Launcher, -;
    Clock = "clock", Always, Calendar, Tempo;
    Date = "date", Always, Calendar, Tempo;
    Weather = "weather", Always, Calendar, Tempo;
    Forecast = "forecast", Always, Calendar, Tempo;
    Notifications = "notifications", Notable, Notifications, -;
    Notices = "notices", Notable, Notifications, -;
    Media = "media", Notable, Media, -;
    MediaDetails = "media_details", Notable, Media, -;
    Tray = "tray", Notable, -, -;
    Updates = "updates", Notable, Updates, -;
    System = "system", Notable, System, -;
    Cpu = "cpu", Notable, System, -;
    Memory = "memory", Notable, System, -;
    Temperature = "temperature", Notable, System, -;
    Disk = "disk", Notable, System, -;
    Privacy = "privacy", Notable, -, -;
    Airplane = "airplane", Notable, Settings, -;
    Dnd = "dnd", Notable, Settings, -;
    IdleInhibitor = "idle_inhibitor", Notable, Settings, Settings;
    PowerProfile = "power_profile", Notable, Settings, Settings;
    Bluetooth = "bluetooth", Notable, Settings, Settings;
    Microphone = "microphone", Notable, Settings, -;
    Volume = "volume", Notable, Settings, -;
    Brightness = "brightness", Never, Settings, Settings;
    Network = "network", Notable, Settings, -;
    Vpn = "vpn", Notable, Settings, -;
    Battery = "battery", Always, Settings, Settings;
    BatteryTime = "battery_time", Always, Settings, Settings;
    PeripheralBattery = "peripheral_battery", Notable, Settings, Settings;
    KeyboardLayout = "keyboard_layout", Notable, -, -;
    KeyboardSubmap = "keyboard_submap", Notable, -, -;
    Config = "config", Always, Config, Config;
}

impl ItemId {
    pub fn parse(name: &str) -> Option<Self> {
        Self::ALL.iter().copied().find(|item| item.name() == name)
    }
}

/// What a gesture on an item does.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Action {
    /// One of the item's [`actions`](ItemId::actions).
    Named(&'static str),
    Open(Open),
    Run(String),
    None,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Gestures {
    pub right_click: Option<Action>,
    pub middle_click: Option<Action>,
    pub scroll_up: Option<Action>,
    pub scroll_down: Option<Action>,
}

impl Gestures {
    pub const KEYS: [&str; 4] = ["right_click", "middle_click", "scroll_up", "scroll_down"];

    pub fn get_mut(&mut self, key: &str) -> Option<&mut Option<Action>> {
        match key {
            "right_click" => Some(&mut self.right_click),
            "middle_click" => Some(&mut self.middle_click),
            "scroll_up" => Some(&mut self.scroll_up),
            "scroll_down" => Some(&mut self.scroll_down),
            _ => None,
        }
    }
}

pub struct ZoneDefaults {
    pub items: &'static [ItemId],
    pub details: &'static [ItemId],
    pub open: Open,
}

pub fn builtin_zone(name: &str) -> Option<ZoneDefaults> {
    use ItemId::*;

    let zone = |items, details, open| ZoneDefaults {
        items,
        details,
        open,
    };

    Some(match name {
        "workspaces" => zone(&[Workspaces], &[WorkspaceList], Open::Nothing),
        "window" => zone(&[Window], &[LauncherButton, WindowList], Open::Nothing),
        "media" => zone(&[Media], &[MediaDetails], Open::Panel(Panel::Media)),
        "clock" => zone(
            &[Weather, Date, Clock, Notifications],
            &[Date, Forecast, Notices],
            Open::Panel(Panel::Calendar),
        ),
        "status" => zone(
            &[
                Privacy,
                Airplane,
                Dnd,
                IdleInhibitor,
                PowerProfile,
                Volume,
                Microphone,
                Bluetooth,
                Network,
                Vpn,
                Battery,
            ],
            &[System, Updates, BatteryTime],
            Open::Panel(Panel::Settings),
        ),
        "privacy" => zone(&[Privacy], &[], Open::Nothing),
        "system" => zone(&[Cpu, Memory, Temperature], &[], Open::Panel(Panel::System)),
        "tray" => zone(&[Tray], &[], Open::Nothing),
        "updates" => zone(&[Updates], &[], Open::Panel(Panel::Updates)),
        "keyboard" => zone(&[KeyboardLayout, KeyboardSubmap], &[], Open::Nothing),
        "config" => zone(&[Config], &[], Open::Config),
        _ => return None,
    })
}
