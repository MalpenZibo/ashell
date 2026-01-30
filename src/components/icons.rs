use crate::theme::AshellTheme;
use iced::{
    Color, Element, Font, Length, Theme,
    widget::{
        Text, button as button_fn,
        button::{Status, Style},
        container, text,
    },
};

pub trait Icon {
    fn to_text<'a>(self) -> Text<'a>;

    fn to_text_mono<'a>(self) -> Text<'a>;
}

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
    Bell,
    BellBadge,
}

impl StaticIcon {
    fn get_str(&self) -> &'static str {
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
            StaticIcon::Bell => "\u{eaa2}",
            StaticIcon::BellBadge => "\u{eb9a}",
        }
    }

    fn get_font(&self) -> &'static str {
        match self {
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
            | StaticIcon::HeadphoneBatteryCharging => "Ashell Custom Icon",
            _ => "Symbols Nerd Font",
        }
    }

    fn get_font_mono(&self) -> &'static str {
        match self {
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
            | StaticIcon::HeadphoneBatteryCharging => "Ashell Custom Icon",
            _ => "Symbols Nerd Font Mono",
        }
    }
}

impl Icon for StaticIcon {
    fn to_text<'a>(self) -> Text<'a> {
        text(self.get_str()).font(Font::with_name(self.get_font()))
    }

    fn to_text_mono<'a>(self) -> Text<'a> {
        text(self.get_str()).font(Font::with_name(self.get_font_mono()))
    }
}

#[derive(Clone)]
pub struct DynamicIcon(pub String);

impl Icon for DynamicIcon {
    fn to_text<'a>(self) -> Text<'a> {
        text(self.0).font(Font::with_name("Symbols Nerd Font"))
    }

    fn to_text_mono<'a>(self) -> Text<'a> {
        text(self.0)
            .font(Font::with_name("Symbols Nerd Font Mono"))
            .line_height(1.0)
    }
}

pub fn icon<'a>(icon: impl Icon) -> Text<'a> {
    icon.to_text()
}

pub fn icon_mono<'a>(icon: impl Icon) -> Text<'a> {
    icon.to_text_mono()
}

pub enum IconButtonSize {
    Small,
    Medium,
    Large,
}

enum OnPress<'a, Message> {
    Direct(Message),
    Closure(Box<dyn Fn() -> Message + 'a>),
}

pub type StyleFn<'a, Theme> = Box<dyn for<'b> Fn(&'b Theme, Status) -> Style + 'a>;

pub struct IconButton<'a, I: Icon, Message> {
    theme: &'a AshellTheme,
    icon: I,
    on_press: Option<OnPress<'a, Message>>,
    button_class: StyleFn<'a, Theme>,
    color: Option<Color>,
    size: IconButtonSize,
}

impl<'a, I: Icon, Message> IconButton<'a, I, Message> {
    pub fn on_press(mut self, on_press: Message) -> Self {
        self.on_press = Some(OnPress::Direct(on_press));
        self
    }

    pub fn on_press_with(mut self, on_press: impl Fn() -> Message + 'a) -> Self {
        self.on_press = Some(OnPress::Closure(Box::new(on_press)));
        self
    }

    pub fn on_press_maybe(mut self, on_press: Option<Message>) -> Self {
        self.on_press = on_press.map(OnPress::Direct);
        self
    }

    pub fn style(mut self, style: impl for<'b> Fn(&'b Theme, Status) -> Style + 'a) -> Self {
        self.button_class = Box::new(style) as StyleFn<'a, Theme>;
        self
    }

    pub fn color(self, color: impl Into<Color>) -> Self {
        self.color_maybe(Some(color))
    }

    pub fn color_maybe(mut self, color: Option<impl Into<Color>>) -> Self {
        let color = color.map(Into::into);

        self.color = color;

        self
    }

    pub fn size(mut self, size: IconButtonSize) -> Self {
        self.size = size;

        self
    }
}

impl<'a, I: Icon, Message: 'static + Clone> From<IconButton<'a, I, Message>>
    for Element<'a, Message>
{
    #[inline]
    fn from(value: IconButton<'a, I, Message>) -> Self {
        let (container_size, font_size) = match value.size {
            IconButtonSize::Small => (24., value.theme.font_size.xs),
            IconButtonSize::Medium => (32., value.theme.font_size.xs),
            IconButtonSize::Large => (38., value.theme.font_size.sm),
        };

        let btn = button_fn(
            container(
                icon_mono(value.icon)
                    .size(font_size)
                    .color_maybe(value.color),
            )
            .center_x(Length::Fixed(container_size))
            .center_y(Length::Fixed(container_size))
            .clip(true),
        )
        .padding(0)
        .style(value.button_class);

        let btn = match value.on_press {
            Some(OnPress::Direct(message)) => btn.on_press(message),
            Some(OnPress::Closure(closure)) => btn.on_press_with(closure),
            None => btn,
        };

        btn.into()
    }
}

pub fn icon_button<'a, Message: 'static + Clone>(
    theme: &'a AshellTheme,
    icon: impl Icon,
) -> IconButton<'a, impl Icon, Message> {
    IconButton {
        theme,
        icon,
        on_press: None,
        button_class: Box::new(theme.round_button_style()),
        color: None,
        size: IconButtonSize::Medium,
    }
}
