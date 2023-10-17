use glib::ObjectExt;
use gtk4::Widget;
use leptos::{create_memo, create_signal, Memo, ReadSignal, SignalGet, SignalSet};
use std::{rc::Rc, time::Duration};
use tokio::time::sleep;

use crate::{
    bar::{MenuAction, MenuType},
    gtk4_wrapper::{
        container, label, overlay, separator, spawn, Align, Component, Orientation, TextAlign,
    },
    modules::settings::battery::{battery_indicator, battery_settings_label},
    utils::{
        battery::{get_battery_capacity, BatteryData},
        launcher::{lock, logout, poweroff, reboot, suspend},
        net::net_monitor,
    },
};

use self::net::net_indicator;

mod battery;
mod net;

pub fn settings(toggle_menu: Rc<dyn Fn(MenuType, MenuAction)>) -> Widget {
    let (battery, set_battery) = create_signal(get_battery_capacity());
    let (active_connection, vpn_list) = net_monitor();

    spawn(async move {
        loop {
            sleep(Duration::from_secs(60)).await;
            set_battery.set(get_battery_capacity());
        }
    });

    container()
        .class(vec!["header-button", "settings"])
        .spacing(8)
        .on_click(move || {
            toggle_menu(
                MenuType::Settings,
                MenuAction::Open(Box::new(move || (settings_menu(battery), Align::End))),
            )
        })
        .children(vec![
            net_indicator(active_connection),
            battery_indicator(battery),
        ])
        .into()
}

enum Round {
    Top,
    Bottom,
}

fn section(
    sub_menu_open: Memo<bool>,
    close_sub_menu: impl Fn() + 'static,
    content: Widget,
    round: Option<Round>,
) -> Widget {
    overlay()
        .children(vec![
            container()
                .class(vec!["section"])
                .children(vec![content])
                .into(),
            container()
                .class(match round {
                    Some(Round::Top) => vec!["overlay-top"],
                    Some(Round::Bottom) => vec!["overlay-bottom"],
                    None => vec!["overlay"],
                })
                .on_click(close_sub_menu)
                .visible(sub_menu_open)
                .into(),
        ])
        .into()
}

#[derive(Clone, Copy, PartialEq)]
enum SubMenu {
    Power,
}

fn settings_menu(battery: ReadSignal<Option<BatteryData>>) -> Widget {
    let (sub_menu, set_sub_menu) = create_signal(Option::<SubMenu>::None);

    let sub_menu_is_open = create_memo(move |_| sub_menu.get().is_some());

    let close_sub_menu = move || set_sub_menu.set(None);

    container()
        .orientation(Orientation::Vertical)
        .size((350, -1))
        .hexpand(true)
        .spacing(4)
        .children(vec![
            section(
                sub_menu_is_open,
                close_sub_menu,
                container()
                    .spacing(4)
                    .children(vec![
                        container()
                            .children(vec![battery_settings_label(battery)])
                            .hexpand(true)
                            .into(),
                        container()
                            .spacing(4)
                            .children(vec![
                                label()
                                    .class(vec!["settings-button"])
                                    .on_click(lock)
                                    .text("")
                                    .into(),
                                label()
                                    .class(vec!["settings-button"])
                                    .text("󰐥")
                                    .on_click(move || set_sub_menu.set(Some(SubMenu::Power)))
                                    .into(),
                            ])
                            .into(),
                    ])
                    .into(),
                Some(Round::Top),
            ),
            power_sub_menu(sub_menu),
            section(
                sub_menu_is_open,
                close_sub_menu,
                label().class(vec!["menu-voice"]).text("Settings").into(),
                None,
            ),
        ])
        .into()
}

fn power_sub_menu(sub_menu: ReadSignal<Option<SubMenu>>) -> Widget {
    let visible = create_memo(move |_| sub_menu.get() == Some(SubMenu::Power));

    container()
        .orientation(Orientation::Vertical)
        .spacing(4)
        .children(vec![
            label()
                .text_halign(TextAlign::Start)
                .class(vec!["menu-voice"])
                .text("Suspend")
                .on_click(suspend)
                .into(),
            label()
                .text_halign(TextAlign::Start)
                .class(vec!["menu-voice"])
                .text("Reboot")
                .on_click(reboot)
                .into(),
            label()
                .text_halign(TextAlign::Start)
                .class(vec!["menu-voice"])
                .text("Power Off")
                .on_click(poweroff)
                .into(),
            separator().into(),
            label()
                .text_halign(TextAlign::Start)
                .class(vec!["menu-voice"])
                .text("Logout")
                .on_click(logout)
                .into(),
        ])
        .visible(visible)
        .into()
}
