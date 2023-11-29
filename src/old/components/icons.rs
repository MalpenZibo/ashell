use crate::{
    nodes,
    reactive_gtk::{container, label, AsStr, Dynamic, IntoSignal, Node, NodeBuilder, Orientation},
};
use futures_signals::signal::SignalExt;

#[derive(Copy, Clone, Default)]
pub enum Icons {
    #[default]
    None,
    Launcher,
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
    Mic0,
    Mic1,
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
    Ethernet,
    Vpn,
    Lock,
    Power,
    RightArrow,
    Brightness,
    Point
}

impl From<Icons> for &'static str {
    fn from(icon: Icons) -> &'static str {
        match icon {
            Icons::None => "",
            Icons::Launcher => "󱗼",
            Icons::NoUpdatesAvailable => "󰗠",
            Icons::UpdatesAvailable => "󰳛",
            Icons::MenuClosed => "",
            Icons::MenuOpen => "",
            Icons::Cpu => "󰔂",
            Icons::Mem => "󰘚",
            Icons::Temp => "󰔏",
            Icons::Speaker0 => "󰸈",
            Icons::Speaker1 => "󰕿",
            Icons::Speaker2 => "󰖀",
            Icons::Speaker3 => "󰕾",
            Icons::Headphones0 => "󰟎",
            Icons::Headphones1 => "󰋋",
            Icons::Mic0 => "󰍭",
            Icons::Mic1 => "󰍬",
            Icons::Battery0 => "󰂃",
            Icons::Battery1 => "󰁼",
            Icons::Battery2 => "󰁾",
            Icons::Battery3 => "󰂀",
            Icons::Battery4 => "󰁹",
            Icons::BatteryCharging => "󰂄",
            Icons::Wifi0 => "󰤭",
            Icons::Wifi1 => "󰤟",
            Icons::Wifi2 => "󰤢",
            Icons::Wifi3 => "󰤥",
            Icons::Wifi4 => "󰤨",
            Icons::Ethernet => "󰈀",
            Icons::Vpn => "󰖂",
            Icons::Lock => "",
            Icons::Power => "󰐥",
            Icons::RightArrow => "󰁔",
            Icons::Brightness => "󰃟",
            Icons::Point => ""
        }
    }
}

pub fn icon(icon: impl IntoSignal<Icons>) -> impl Into<Node> {
    label()
        .class(vec!["icon"])
        .text::<&str>(Dynamic(icon.into_signal().map(|icon| icon.into())))
}

pub fn icon_with_class<C: AsStr>(
    icon: impl IntoSignal<Icons>,
    classes: impl IntoSignal<Vec<C>>,
) -> impl Into<Node> {
    let classes = classes.into_signal().map(|classes| {
        [
            vec!["icon".to_string()],
            classes
                .iter()
                .map(|c| c.with_str(|c| c.to_string()))
                .collect(),
        ]
        .concat()
    });
    label()
        .class(Dynamic(classes))
        .text::<&str>(Dynamic(icon.into_signal().map(|icon| icon.into())))
}

pub fn icon_with_text<T: AsStr, C: AsStr>(
    r#type: impl IntoSignal<Icons>,
    text: impl IntoSignal<T> + 'static,
    classes: impl IntoSignal<Vec<C>> + 'static,
) -> impl Into<Node> {
    container()
        .class(classes)
        .orientation(Orientation::Horizontal)
        .spacing(4)
        .children(nodes!(icon(r#type), label().text::<T>(text)))
}
