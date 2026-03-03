use guido::prelude::*;
use guido::reactive::{IntoMaybeDyn, MaybeDyn};

fn nerd_font_family() -> FontFamily {
    FontFamily::Name("Symbols Nerd Font".into())
}

fn nerd_font_family_mono() -> FontFamily {
    FontFamily::Name("Symbols Nerd Font Mono".into())
}

fn custom_icon_font() -> FontFamily {
    FontFamily::Name("Ashell Custom Icon".into())
}

// ---------------------------------------------------------------------------
// Icon data types
// ---------------------------------------------------------------------------

#[derive(Copy, Clone, Default)]
#[allow(dead_code)]
pub enum StaticIcon {
    #[default]
    None,
    Refresh,
    NoUpdatesAvailable,
    UpdatesAvailable,
    MenuClosed,
    MenuOpen,
    Cpu,
    Mem,
    Temp,
    Speaker0,
    Speaker1,
    Speaker2,
    Speaker3,
    Headphones0,
    Headphones1,
    Headset,
    Mic0,
    Mic1,
    MonitorSpeaker,
    ScreenShare,
    Battery0,
    Battery1,
    Battery2,
    Battery3,
    Battery4,
    BatteryCharging,
    Wifi0,
    Wifi1,
    Wifi2,
    Wifi3,
    Wifi4,
    Wifi5,
    WifiLock1,
    WifiLock2,
    WifiLock3,
    WifiLock4,
    WifiLock5,
    Ethernet,
    Vpn,
    Bluetooth,
    BluetoothConnected,
    PowerSaver,
    Balanced,
    Performance,
    EyeOpened,
    EyeClosed,
    Lock,
    Power,
    Reboot,
    Suspend,
    Hibernate,
    Logout,
    RightArrow,
    Brightness,
    Point,
    Close,
    Airplane,
    Webcam,
    SkipPrevious,
    Play,
    Pause,
    SkipNext,
    MusicNote,
    Drive,
    IpAddress,
    DownloadSpeed,
    UploadSpeed,
    Copy,
    LeftChevron,
    RightChevron,
    Keyboard,
    Mouse,
    Gamepad,
    KeyboardBatteryFull,
    KeyboardBatteryMedium,
    KeyboardBatteryLow,
    KeyboardBatteryAlert,
    KeyboardBatteryCharging,
    MouseBatteryFull,
    MouseBatteryMedium,
    MouseBatteryLow,
    MouseBatteryAlert,
    MouseBatteryCharging,
    HeadphoneBatteryFull,
    HeadphoneBatteryMedium,
    HeadphoneBatteryLow,
    HeadphoneBatteryAlert,
    HeadphoneBatteryCharging,
    GamepadBatteryFull,
    GamepadBatteryMedium,
    GamepadBatteryLow,
    GamepadBatteryAlert,
    GamepadBatteryCharging,
    Remove,
}

impl StaticIcon {
    pub fn get_str(&self) -> &'static str {
        match self {
            StaticIcon::None => "",
            StaticIcon::Refresh => "\u{f0453}",
            StaticIcon::NoUpdatesAvailable => "\u{f05e0}",
            StaticIcon::UpdatesAvailable => "\u{f0cdb}",
            StaticIcon::MenuClosed => "\u{f035f}",
            StaticIcon::MenuOpen => "\u{f035d}",
            StaticIcon::Cpu => "\u{f0502}",
            StaticIcon::Mem => "\u{efc5}",
            StaticIcon::Temp => "\u{f050f}",
            StaticIcon::Speaker0 => "\u{f0e08}",
            StaticIcon::Speaker1 => "\u{f057f}",
            StaticIcon::Speaker2 => "\u{f0580}",
            StaticIcon::Speaker3 => "\u{f057e}",
            StaticIcon::Headphones0 => "\u{f07ce}",
            StaticIcon::Headphones1 => "\u{f02cb}",
            StaticIcon::Headset => "\u{f02ce}",
            StaticIcon::Mic0 => "\u{f036d}",
            StaticIcon::Mic1 => "\u{f036c}",
            StaticIcon::ScreenShare => "\u{f1483}",
            StaticIcon::MonitorSpeaker => "\u{f0f5f}",
            StaticIcon::Battery0 => "\u{f0083}",
            StaticIcon::Battery1 => "\u{f007c}",
            StaticIcon::Battery2 => "\u{f007e}",
            StaticIcon::Battery3 => "\u{f0080}",
            StaticIcon::Battery4 => "\u{f0079}",
            StaticIcon::BatteryCharging => "\u{f0084}",
            StaticIcon::Wifi0 => "\u{f092d}",
            StaticIcon::Wifi1 => "\u{f092f}",
            StaticIcon::Wifi2 => "\u{f091f}",
            StaticIcon::Wifi3 => "\u{f0922}",
            StaticIcon::Wifi4 => "\u{f0925}",
            StaticIcon::Wifi5 => "\u{f0928}",
            StaticIcon::WifiLock1 => "\u{f092c}",
            StaticIcon::WifiLock2 => "\u{f0921}",
            StaticIcon::WifiLock3 => "\u{f0924}",
            StaticIcon::WifiLock4 => "\u{f0927}",
            StaticIcon::WifiLock5 => "\u{f092a}",
            StaticIcon::Ethernet => "\u{f0200}",
            StaticIcon::Vpn => "\u{f0582}",
            StaticIcon::Bluetooth => "\u{f00af}",
            StaticIcon::BluetoothConnected => "\u{f00b1}",
            StaticIcon::PowerSaver => "\u{f0f86}",
            StaticIcon::Balanced => "\u{f0f85}",
            StaticIcon::Performance => "\u{f04c5}",
            StaticIcon::EyeOpened => "\u{f0208}",
            StaticIcon::EyeClosed => "\u{f0209}",
            StaticIcon::Lock => "\u{f033e}",
            StaticIcon::Power => "\u{f0425}",
            StaticIcon::Reboot => "\u{f0450}",
            StaticIcon::Suspend => "\u{f0904}",
            StaticIcon::Hibernate => "\u{f0717}",
            StaticIcon::Logout => "\u{f05fd}",
            StaticIcon::RightArrow => "\u{f0054}",
            StaticIcon::Brightness => "\u{f00e0}",
            StaticIcon::Point => "\u{f444}",
            StaticIcon::Close => "\u{f0156}",
            StaticIcon::Airplane => "\u{f001d}",
            StaticIcon::Webcam => "\u{f03d}",
            StaticIcon::SkipPrevious => "\u{f04ae}",
            StaticIcon::Play => "\u{f040a}",
            StaticIcon::Pause => "\u{f03e4}",
            StaticIcon::SkipNext => "\u{f04ad}",
            StaticIcon::MusicNote => "\u{f0387}",
            StaticIcon::Drive => "\u{f02ca}",
            StaticIcon::IpAddress => "\u{f0a60}",
            StaticIcon::DownloadSpeed => "\u{f06f4}",
            StaticIcon::UploadSpeed => "\u{f06f6}",
            StaticIcon::Copy => "\u{f018f}",
            StaticIcon::LeftChevron => "\u{f0141}",
            StaticIcon::RightChevron => "\u{f0142}",
            StaticIcon::Keyboard => "\u{f030c}",
            StaticIcon::Mouse => "\u{f037d}",
            StaticIcon::Gamepad => "\u{f05ba}",
            StaticIcon::KeyboardBatteryFull => "\u{c0000}",
            StaticIcon::KeyboardBatteryMedium => "\u{c0001}",
            StaticIcon::KeyboardBatteryLow => "\u{c0002}",
            StaticIcon::KeyboardBatteryAlert => "\u{c0003}",
            StaticIcon::KeyboardBatteryCharging => "\u{c0004}",
            StaticIcon::MouseBatteryFull => "\u{c0005}",
            StaticIcon::MouseBatteryMedium => "\u{c0006}",
            StaticIcon::MouseBatteryLow => "\u{c0007}",
            StaticIcon::MouseBatteryAlert => "\u{c0008}",
            StaticIcon::MouseBatteryCharging => "\u{c0009}",
            StaticIcon::HeadphoneBatteryFull => "\u{c000a}",
            StaticIcon::HeadphoneBatteryMedium => "\u{c000b}",
            StaticIcon::HeadphoneBatteryLow => "\u{c000c}",
            StaticIcon::HeadphoneBatteryAlert => "\u{c000d}",
            StaticIcon::HeadphoneBatteryCharging => "\u{c000e}",
            StaticIcon::GamepadBatteryFull => "\u{f074d}",
            StaticIcon::GamepadBatteryMedium => "\u{f074f}",
            StaticIcon::GamepadBatteryLow => "\u{f074e}",
            StaticIcon::GamepadBatteryAlert => "\u{f074b}",
            StaticIcon::GamepadBatteryCharging => "\u{f0a22}",
            StaticIcon::Remove => "\u{f0377}",
        }
    }

    fn uses_custom_font(&self) -> bool {
        matches!(
            self,
            StaticIcon::KeyboardBatteryFull
                | StaticIcon::KeyboardBatteryMedium
                | StaticIcon::KeyboardBatteryLow
                | StaticIcon::KeyboardBatteryAlert
                | StaticIcon::KeyboardBatteryCharging
                | StaticIcon::MouseBatteryFull
                | StaticIcon::MouseBatteryMedium
                | StaticIcon::MouseBatteryLow
                | StaticIcon::MouseBatteryAlert
                | StaticIcon::MouseBatteryCharging
                | StaticIcon::HeadphoneBatteryFull
                | StaticIcon::HeadphoneBatteryMedium
                | StaticIcon::HeadphoneBatteryLow
                | StaticIcon::HeadphoneBatteryAlert
                | StaticIcon::HeadphoneBatteryCharging
        )
    }

    pub fn font_family(&self) -> FontFamily {
        if self.uses_custom_font() {
            custom_icon_font()
        } else {
            nerd_font_family()
        }
    }

    pub fn font_family_mono(&self) -> FontFamily {
        if self.uses_custom_font() {
            custom_icon_font()
        } else {
            nerd_font_family_mono()
        }
    }
}

/// Arbitrary icon character string (always uses Nerd Font).
#[derive(Clone)]
pub struct DynamicIcon(pub String);

// ---------------------------------------------------------------------------
// IconKind — unified icon data: predefined variant OR arbitrary string
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub enum IconKind {
    Static(StaticIcon),
    Dynamic(String),
}

impl Default for IconKind {
    fn default() -> Self {
        IconKind::Static(StaticIcon::None)
    }
}

impl IconKind {
    fn get_str(&self) -> &str {
        match self {
            IconKind::Static(s) => s.get_str(),
            IconKind::Dynamic(s) => s.as_str(),
        }
    }

    fn font_family(&self) -> FontFamily {
        match self {
            IconKind::Static(s) => s.font_family(),
            IconKind::Dynamic(_) => nerd_font_family(),
        }
    }

    fn font_family_mono(&self) -> FontFamily {
        match self {
            IconKind::Static(s) => s.font_family_mono(),
            IconKind::Dynamic(_) => nerd_font_family_mono(),
        }
    }
}

impl From<StaticIcon> for IconKind {
    fn from(i: StaticIcon) -> Self {
        IconKind::Static(i)
    }
}

impl From<DynamicIcon> for IconKind {
    fn from(d: DynamicIcon) -> Self {
        IconKind::Dynamic(d.0)
    }
}

// ---------------------------------------------------------------------------
// IntoMaybeDyn impls — let callers write icon().ic(StaticIcon::Wifi5)
// or icon().ic("custom_char") without explicit wrapping
// ---------------------------------------------------------------------------

impl IntoMaybeDyn<StaticIcon> for StaticIcon {
    fn into_maybe_dyn(self) -> MaybeDyn<StaticIcon> {
        MaybeDyn::Static(self)
    }
}

impl IntoMaybeDyn<IconKind> for IconKind {
    fn into_maybe_dyn(self) -> MaybeDyn<IconKind> {
        MaybeDyn::Static(self)
    }
}

impl IntoMaybeDyn<IconKind> for StaticIcon {
    fn into_maybe_dyn(self) -> MaybeDyn<IconKind> {
        MaybeDyn::Static(IconKind::Static(self))
    }
}

impl IntoMaybeDyn<IconKind> for DynamicIcon {
    fn into_maybe_dyn(self) -> MaybeDyn<IconKind> {
        MaybeDyn::Static(IconKind::Dynamic(self.0))
    }
}

impl IntoMaybeDyn<IconKind> for &str {
    fn into_maybe_dyn(self) -> MaybeDyn<IconKind> {
        MaybeDyn::Static(IconKind::Dynamic(self.to_string()))
    }
}

// Convenience: pass IconKind or StaticIcon directly to Option<IconKind> props
impl IntoMaybeDyn<Option<IconKind>> for IconKind {
    fn into_maybe_dyn(self) -> MaybeDyn<Option<IconKind>> {
        MaybeDyn::Static(Some(self))
    }
}

impl IntoMaybeDyn<Option<IconKind>> for StaticIcon {
    fn into_maybe_dyn(self) -> MaybeDyn<Option<IconKind>> {
        MaybeDyn::Static(Some(IconKind::Static(self)))
    }
}

// ---------------------------------------------------------------------------
// Icon component
// ---------------------------------------------------------------------------

#[component]
pub struct Icon {
    #[prop]
    ic: IconKind,
    #[prop(default = "false")]
    mono: bool,
    #[prop(default = "Color::WHITE")]
    color: Color,
    #[prop(default = "14.0")]
    font_size: f32,
}

impl Icon {
    fn render(&self) -> impl Widget + use<> {
        let ic = self.ic.clone();
        let ic2 = self.ic.clone();
        let mono = self.mono.clone();

        text(move || ic.get().get_str().to_string())
            .font_family(move || {
                let i = ic2.get();
                if mono.get() {
                    i.font_family_mono()
                } else {
                    i.font_family()
                }
            })
            .color(self.color.clone())
            .font_size(self.font_size.clone())
    }
}
