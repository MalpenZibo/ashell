use self::battery::{battery_indicator, battery_settings_indicator};
use crate::{
    bar::{MenuAction, MenuType},
    nodes,
    reactive_gtk::{
        container, label, overlay, separator, Align, Dynamic, Node, NodeBuilder, Orientation,
        TextAlign,
    },
    utils::{
        battery::{get_battery_capacity, BatteryData},
        launcher::{lock, logout, poweroff, reboot, suspend},
        poll,
    },
};
use futures_signals::signal::Mutable;
use std::{rc::Rc, time::Duration};

mod battery;

pub fn settings(toggle_menu: Rc<dyn Fn(MenuType, MenuAction)>) -> impl Into<Node> {
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
        .children(nodes!(battery_indicator(battery)))
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
    let mut main_section = vec![overlay()
        .children(vec![
            container()
                .class(vec!["section"])
                .children(nodes!(content))
                .into(),
            container()
                .class(vec!["overlay"])
                .on_click({
                    let submenu = submenu.clone();
                    move || submenu.set(None)
                })
                .visible(Dynamic(submenu.signal_ref(|submenu| submenu.is_some())))
                .into(),
        ])
        .into()];

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

pub fn settings_menu(battery: Mutable<Option<BatteryData>>) -> impl Into<Node> {
    let submenu: Mutable<Option<SubMenuType>> = Mutable::new(None);

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
