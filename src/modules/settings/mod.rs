use self::{
    battery::{battery_indicator, battery_settings_indicator},
    net::net_indicator,
};
use crate::{
    bar::{MenuAction, MenuType},
    nodes,
    reactive_gtk::{
        container, label, overlay, scale, separator, Align, Dynamic, MaybeSignal, Node,
        NodeBuilder, Orientation, TextAlign,
    },
    utils::{
        battery::{get_battery_capacity, BatteryData},
        brightness::{brightness_monitor, set_brightness},
        launcher::{lock, logout, poweroff, reboot, suspend},
        net::net_monitor,
        poll,
    },
};
use futures_signals::signal::Mutable;
use std::{rc::Rc, time::Duration};

mod audio;
mod battery;
mod net;

pub fn settings(toggle_menu: Rc<dyn Fn(MenuType, MenuAction)>) -> impl Into<Node> {
    let (active_connection, vpn_list) = net_monitor();
    let battery = Mutable::new(get_battery_capacity());

    poll(
        {
            let battery = battery.clone();
            move || {
                battery.replace(get_battery_capacity());
            }
        },
        Duration::from_secs(60),
    );

    container()
        .class(vec!["bar-item", "settings", "interactive"])
        .spacing(8)
        .on_click({
            let battery = battery.clone();
            move || {
                toggle_menu(
                    MenuType::Settings,
                    MenuAction::Open(Box::new({
                        let battery = battery.clone();
                        move || (settings_menu(battery.clone()).into(), Align::End)
                    })),
                )
            }
        })
        .children(nodes!(
            net_indicator(active_connection),
            battery_indicator(battery)
        ))
}

pub enum Round {
    Top,
    Bottom,
}

#[derive(Clone, Copy, PartialEq)]
pub enum SubMenuType {
    Power,
}

pub fn section(
    submenu: Mutable<Option<SubMenuType>>,
    content: impl Into<Node>,
    children: Vec<(SubMenuType, Node)>,
) -> impl Into<Node> {
    let mut submenu_sections: Vec<Node> = children
        .into_iter()
        .map(|elem| {
            elem.1.visible(Dynamic({
                let submenu = submenu.clone();
                submenu.signal_ref(move |submenu| {
                    submenu
                        .as_ref()
                        .map(|submenu| *submenu == elem.0)
                        .unwrap_or(false)
                })
            }))
        })
        .collect();
    let mut main_section = nodes![overlay().children(nodes![
        container().class(vec!["section"]).children(nodes!(content)),
        container()
            .class(vec!["overlay"])
            .on_click({
                let submenu = submenu.clone();
                move || submenu.set(None)
            })
            .visible(Dynamic(submenu.signal_ref(|submenu| submenu.is_some())))
    ])];

    main_section.append(&mut submenu_sections);

    container()
        .orientation(Orientation::Vertical)
        .children(main_section)
}

pub fn vmargin(submenu: Mutable<Option<SubMenuType>>, round: Round) -> impl Into<Node> {
    overlay().children(nodes!(
        container().class(match round {
            Round::Top => vec!["settings-menu-top-margin"],
            Round::Bottom => vec!["settings-menu-bottom-margin"],
        }),
        container()
            .class(match round {
                Round::Top => vec!["overlay-top"],
                Round::Bottom => vec!["overlay-bottom"],
            })
            .visible(Dynamic(submenu.signal_ref(|submenu| submenu.is_some())))
    ))
}

pub fn slider(
    indicator: impl MaybeSignal<String>,
    indicator_classes: Vec<&str>,
    value: Mutable<f64>,
    range: (f64, f64),
    on_change: impl Fn(f64) + 'static,
    on_toggle: Option<impl Fn() + 'static>,
    with_menu: bool,
) -> impl Into<Node> {
    let indicator_classes = [indicator_classes, {
        if on_toggle.is_none() {
            vec!["settings-item"]
        } else {
            vec!["settings-item", "interactive"]
        }
    }]
    .concat();
    let mut children = nodes!(
        label()
            .class(indicator_classes)
            .text(indicator),
        scale()
            .hexpand(true)
            .value(Dynamic(value.clone().signal()))
            .round_digits(0)
            .range(range)
            .on_change(move |new_value| {
                value.replace(new_value);
                on_change(new_value);
            })
    );

    if with_menu {
        children.push(
            label()
                .class(vec!["settings-item", "interactive"])
                .text("󰁔".to_string())
                .into(),
        );
    }

    container()
        .class(vec!["settings-slider"])
        .children(children)
}

pub fn settings_menu(battery: Mutable<Option<BatteryData>>) -> impl Into<Node> {
    let submenu: Mutable<Option<SubMenuType>> = Mutable::new(None);
    let brightness = brightness_monitor();

    container()
        .orientation(Orientation::Vertical)
        .size((350, -1))
        .hexpand(true)
        .children(nodes!(
            vmargin(submenu.clone(), Round::Top),
            section(
                submenu.clone(),
                container().spacing(4).children(nodes!(
                    container()
                        .spacing(4)
                        .children(nodes!(battery_settings_indicator(battery)))
                        .hexpand(true),
                    container().spacing(4).children(nodes!(
                        label()
                            .class(vec!("settings-item", "interactive"))
                            .on_click(lock)
                            .text("".to_string()),
                        label()
                            .class(vec!("settings-item", "interactive"))
                            .on_click({
                                let submenu = submenu.clone();
                                move || submenu.set(Some(SubMenuType::Power))
                            })
                            .text("󰐥".to_string())
                    ))
                ),),
                vec!(power_submenu()),
            ),
            section(
                submenu.clone(),
                slider(
                    "󰃟".to_string(),
                    vec!("brightness-icon-fix"),
                    brightness.clone(),
                    (0., 255.),
                    |value| {
                        set_brightness(value as u32);
                    },
                    None::<fn()>,
                    false
                ),
                vec!(),
            ),
            vmargin(submenu.clone(), Round::Bottom)
        ))
}

pub fn power_submenu() -> (SubMenuType, Node) {
    (
        SubMenuType::Power,
        container()
            .orientation(Orientation::Vertical)
            .spacing(4)
            .children(vec![
                label()
                    .text_halign(TextAlign::Start)
                    .class(vec!["menu-voice"])
                    .text("Suspend".to_string())
                    .on_click(suspend)
                    .into(),
                label()
                    .text_halign(TextAlign::Start)
                    .class(vec!["menu-voice"])
                    .text("Reboot".to_string())
                    .on_click(reboot)
                    .into(),
                label()
                    .text_halign(TextAlign::Start)
                    .class(vec!["menu-voice"])
                    .text("Power Off".to_string())
                    .on_click(poweroff)
                    .into(),
                separator().into(),
                label()
                    .text_halign(TextAlign::Start)
                    .class(vec!["menu-voice"])
                    .text("Logout".to_string())
                    .on_click(logout)
                    .into(),
            ])
            .into(),
    )
}
