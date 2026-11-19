use serde::Deserialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Show {
    Always,
    Notable,
    Never,
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
}

/// `"none"`, or a panel by its name.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(try_from = "String")]
pub enum Open {
    Nothing,
    Panel(Panel),
}

impl TryFrom<String> for Open {
    type Error = String;

    fn try_from(name: String) -> Result<Self, String> {
        Open::parse(&name).ok_or_else(|| {
            let panels: Vec<_> = Panel::ALL.iter().map(|panel| panel.name()).collect();
            format!(
                "\"{name}\": expected \"none\" or a panel, one of {}",
                panels.join(", ")
            )
        })
    }
}

impl Open {
    pub fn parse(name: &str) -> Option<Self> {
        if name == "none" {
            return Some(Open::Nothing);
        }

        Panel::ALL
            .into_iter()
            .find(|panel| panel.name() == name)
            .map(Open::Panel)
    }
}

/// `-` for an item whose click is left to its zone.
macro_rules! opens {
    (-) => {
        None
    };
    ($panel:ident) => {
        Some(Open::Panel(Panel::$panel))
    };
}

macro_rules! items {
    ($($variant:ident = $name:literal, $show:ident, $open:tt;)*) => {
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
        }
    };
}

items! {
    Workspaces = "workspaces", Always, -;
    WorkspaceList = "workspace_list", Always, -;
    Window = "window", Notable, -;
    WindowList = "window_list", Notable, -;
    LauncherButton = "launcher_button", Always, Launcher;
    Clock = "clock", Always, Calendar;
    Date = "date", Always, Calendar;
    Weather = "weather", Always, Calendar;
    Forecast = "forecast", Always, Calendar;
    Notifications = "notifications", Notable, Notifications;
    Notices = "notices", Notable, Notifications;
    Media = "media", Notable, Media;
    MediaDetails = "media_details", Notable, Media;
    Tray = "tray", Notable, -;
    Updates = "updates", Notable, Updates;
    System = "system", Notable, System;
    Cpu = "cpu", Notable, System;
    Memory = "memory", Notable, System;
    Temperature = "temperature", Notable, System;
    Disk = "disk", Notable, System;
    Privacy = "privacy", Notable, -;
    Airplane = "airplane", Notable, Settings;
    Dnd = "dnd", Notable, Settings;
    IdleInhibitor = "idle_inhibitor", Notable, Settings;
    PowerProfile = "power_profile", Notable, Settings;
    Bluetooth = "bluetooth", Notable, Settings;
    Microphone = "microphone", Notable, Settings;
    Volume = "volume", Notable, Settings;
    Brightness = "brightness", Never, Settings;
    Network = "network", Notable, Settings;
    Vpn = "vpn", Notable, Settings;
    Battery = "battery", Always, Settings;
    PeripheralBattery = "peripheral_battery", Notable, Settings;
    KeyboardLayout = "keyboard_layout", Notable, -;
    KeyboardSubmap = "keyboard_submap", Notable, -;
}

impl ItemId {
    pub fn parse(name: &str) -> Option<Self> {
        Self::ALL.iter().copied().find(|item| item.name() == name)
    }

    /// What its gestures can be bound to, by name.
    pub fn actions(self) -> &'static [&'static str] {
        match self {
            ItemId::Clock => &[CYCLE_FORMAT, NEXT_TIMEZONE, PREVIOUS_TIMEZONE],
            ItemId::Date => &[NEXT_TIMEZONE, PREVIOUS_TIMEZONE],
            _ => &[],
        }
    }

    pub fn default_gestures(self) -> Gestures {
        let named = |name| Some(Action::Named(name));
        match self {
            ItemId::Clock => Gestures {
                right_click: named(CYCLE_FORMAT),
                scroll_up: named(NEXT_TIMEZONE),
                scroll_down: named(PREVIOUS_TIMEZONE),
                ..Gestures::default()
            },
            ItemId::Date => Gestures {
                scroll_up: named(NEXT_TIMEZONE),
                scroll_down: named(PREVIOUS_TIMEZONE),
                ..Gestures::default()
            },
            _ => Gestures::default(),
        }
    }
}

pub const CYCLE_FORMAT: &str = "cycle_format";
pub const NEXT_TIMEZONE: &str = "next_timezone";
pub const PREVIOUS_TIMEZONE: &str = "previous_timezone";

/// What a gesture on an item does.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Action {
    /// One of the item's [`actions`](ItemId::actions).
    Named(&'static str),
    Open(Panel),
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
                Bluetooth,
                Microphone,
                Volume,
                Network,
                Battery,
            ],
            &[System, Updates],
            Open::Panel(Panel::Settings),
        ),
        "privacy" => zone(&[Privacy], &[], Open::Nothing),
        "system" => zone(&[Cpu, Memory, Temperature], &[], Open::Panel(Panel::System)),
        "tray" => zone(&[Tray], &[], Open::Nothing),
        "updates" => zone(&[Updates], &[], Open::Panel(Panel::Updates)),
        "keyboard" => zone(&[KeyboardLayout, KeyboardSubmap], &[], Open::Nothing),
        _ => return None,
    })
}
