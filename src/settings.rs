use std::time::Duration;

use futures_signals::signal::{Mutable, Signal, SignalExt};
use gtk::{
    traits::{GtkWindowExt, WidgetExt},
    ApplicationWindow,
};

use crate::{
    audio::{audio_subscribe, set_sink, set_volume, toggle_volume, Sink, Source},
    battery::{get_battery_capacity, BatteryData},
    launcher,
    net::{net_monitor, Vpn},
    reactive_gtk::{
        Align, AsStr, Box, Button, Component, Context, Label, Node, Orientation, Overlay, Scale,
        Separator, Surface,
    },
    shell_bar::MenuType,
    utils::poll,
};

pub fn settings(ctx: Context, menu: Mutable<Option<(ApplicationWindow, Node, MenuType)>>) -> Node {
    let battery = Mutable::new(get_battery_capacity());

    let battery1 = battery.clone();
    let battery2 = battery.clone();
    poll(
        move || {
            battery1.replace(get_battery_capacity());
        },
        Duration::from_secs(60),
    );

    let active_connection = Mutable::new(None);
    let vpn_list: Mutable<Vec<Vpn>> = Mutable::new(Vec::with_capacity(0));
    net_monitor(active_connection.clone(), vpn_list.clone());

    let sinks: Mutable<Vec<Sink>> = Mutable::new(Vec::with_capacity(0));
    let sinks1 = sinks.clone();
    let sinks2 = sinks.clone();
    let sources: Mutable<Vec<Source>> = Mutable::new(Vec::with_capacity(0));
    audio_subscribe(sinks.clone(), sources.clone());

    Box::default()
        .class_signal(menu.signal_ref(|m| {
            if m.as_ref()
                .map(|(_, _, menu_type)| *menu_type == MenuType::Settings)
                .unwrap_or_default()
            {
                vec!["bg", "pl-2", "pr-4", "rounded-r-m", "interactive", "active"]
            } else {
                vec!["bg", "pl-2", "pr-4", "rounded-r-m", "interactive"]
            }
        }))
        .on_click(move || {
            menu.replace_with(|m| {
                if let Some((win, _, _)) = m {
                    win.close();
                    None
                } else {
                    let node = ctx.open_surface(
                        Surface::layer(false, (true, true, true, true), None),
                        settings_menu(menu.clone(), battery.clone(), sinks2.clone()),
                    );
                    Some((node.0, node.1, MenuType::Settings))
                }
            });
        })
        .spacing(4)
        .children(vec![
            Label::default()
                .text_signal(active_connection.signal_ref(|c| {
                    c.as_ref()
                        .map(|c| c.to_icon().to_string())
                        .unwrap_or_default()
                }))
                .into(),
            Label::default()
                .text("󰖂")
                .visible_signal(
                    vpn_list.signal_ref(|vpn_list| vpn_list.iter().any(|vpn| vpn.active)),
                )
                .into(),
            Label::default()
                .text_signal(sinks1.signal_ref(|s| {
                    s.iter()
                        .find_map(|s| {
                            if s.active {
                                Some(s.to_icon().to_string())
                            } else {
                                None
                            }
                        })
                        .unwrap_or_default()
                }))
                .into(),
            Label::default()
                .text_signal(sources.signal_ref(|s| {
                    s.iter()
                        .find_map(|s| {
                            if s.active {
                                Some(s.to_icon().to_string())
                            } else {
                                None
                            }
                        })
                        .unwrap_or_default()
                }))
                .visible_signal(sources.signal_ref(|s| s.iter().any(|s| s.active)))
                .into(),
            battery_indicator(battery2),
        ])
        .into()
}

pub fn settings_menu(
    menu: Mutable<Option<(ApplicationWindow, Node, MenuType)>>,
    battery: Mutable<Option<BatteryData>>,
    sinks: Mutable<Vec<Sink>>,
) -> impl FnOnce(Context) -> Node {
    move |ctx| {
        let sub_menu: Mutable<Option<SubMenu>> = Mutable::new(None);
        let window = ctx.window.clone();
        let menu1 = menu.clone();
        let sub_menu1 = sub_menu.clone();
        let sub_menu2 = sub_menu.clone();

        let value = Mutable::<f64>::new(34.);

        let sinks1 = sinks.clone();
        let sinks2 = sinks.clone();

        Overlay::default()
            .children(vec![
                Box::default()
                    .hexpand(true)
                    .vexpand(true)
                    .on_click(move || {
                        ctx.window.hide();
                        menu.replace(None);
                    })
                    .into(),
                Box::default()
                    .class_signal(sub_menu.signal_ref(|m| {
                        if m.is_some() {
                            vec!["m-1", "p-5", "rounded-m", "bg", "border", "disabled-bg"]
                        } else {
                            vec!["m-1", "p-5", "rounded-m", "bg", "border"]
                        }
                    }))
                    .hexpand(false)
                    .vexpand(false)
                    .size((400, 400))
                    .halign(Align::End)
                    .valign(Align::Start)
                    .orientation(Orientation::Vertical)
                    .spacing(8)
                    .children(vec![
                        Box::default()
                            .orientation(Orientation::Horizontal)
                            .spacing(8)
                            .children(vec![
                                setting_button(
                                    sub_menu.clone(),
                                    Box::default()
                                        .class(&["rounded-l", "ph-3", "bg-dark-4"])
                                        .children(vec![battery_indicator(battery)])
                                        .into(),
                                ),
                                Box::default()
                                    .orientation(Orientation::Horizontal)
                                    .hexpand(true)
                                    .halign(Align::End)
                                    .spacing(8)
                                    .children(vec![
                                        setting_button(
                                            sub_menu.clone(),
                                            Button::default()
                                                .class(&["rounded-l", "ph-3"])
                                                .child(Label::default().text(""))
                                                .on_click(move || {
                                                    window.hide();
                                                    menu1.replace(None);
                                                    launcher::lock();
                                                })
                                                .into(),
                                        ),
                                        setting_button(
                                            sub_menu.clone(),
                                            Button::default()
                                                .class(&["rounded-l", "ph-3"])
                                                .child(Label::default().text("󰐥"))
                                                .on_click(move || {
                                                    sub_menu1.replace_with(|m| {
                                                        if m.map(|m| m != SubMenu::Power)
                                                            .unwrap_or(true)
                                                        {
                                                            Some(SubMenu::Power)
                                                        } else {
                                                            None
                                                        }
                                                    });
                                                })
                                                .into(),
                                        ),
                                    ])
                                    .into(),
                            ])
                            .into(),
                        menu_card(
                            sub_menu
                                .signal_ref(|m| m.map(|m| m == SubMenu::Power).unwrap_or_default()),
                            "󰐥",
                            "Power Off",
                            vec![
                                menu_card_item("", "Suspend", || {
                                    launcher::suspend();
                                }),
                                menu_card_item("", "Reboot", || {
                                    launcher::reboot();
                                }),
                                menu_card_item("", "Power Off", || {
                                    launcher::poweroff();
                                }),
                                Separator::default().class(&["mv-2"]).into(),
                                menu_card_item("", "Log Out", || {
                                    launcher::logout();
                                }),
                            ],
                        ),
                        setting_slider(
                            sub_menu.clone(),
                            sinks.signal_ref(|s| {
                                s.iter()
                                    .find_map(|s| {
                                        if s.active {
                                            Some(s.to_type_icon().to_string())
                                        } else {
                                            None
                                        }
                                    })
                                    .unwrap_or_default()
                            }),
                            (0., 100.),
                            value.signal(),
                            Some(|| {
                                toggle_volume(sinks.clone());
                            }),
                            move |v| set_volume(sinks1.clone(), v.round() as u32),
                            Some(move || {
                                sub_menu2.replace_with(|m| {
                                    if m.map(|m| m != SubMenu::Audio).unwrap_or(true) {
                                        Some(SubMenu::Audio)
                                    } else {
                                        None
                                    }
                                });
                            }),
                        ),
                        menu_card(
                            sub_menu
                                .signal_ref(|m| m.map(|m| m == SubMenu::Audio).unwrap_or_default()),
                            "󰕾",
                            "Sound Output",
                            vec![], // sinks.signal_ref(move |s| {
                                    //     let sinks3 = sinks2.clone();
                                    //     s.into_iter()
                                    //         .map(move |s| {
                                    //             let index = s.index;
                                    //             let name = s.name.clone();
                                    //             menu_card_item("", "", move || {
                                    //                 set_sink(sinks3.clone(), index, &name);
                                    //             })
                                    //         })
                                    //         .collect()
                                    // }),
                        ),
                    ])
                    .into(),
            ])
            .into()
    }
}

#[derive(Clone, Copy, PartialEq)]
enum SubMenu {
    Power,
    Audio,
}

fn battery_indicator(battery: Mutable<Option<BatteryData>>) -> Node {
    Box::default()
        .visible_signal(battery.signal_ref(|b| b.is_some()))
        .spacing(4)
        .children(vec![
            Label::default()
                .class_signal(battery.signal_ref(|b| {
                    b.as_ref()
                        .map(|b| vec![b.to_class().to_owned()])
                        .unwrap_or_default()
                }))
                .text_signal(battery.signal_ref(|b| {
                    b.as_ref()
                        .map(|b| b.to_icon().to_string())
                        .unwrap_or_default()
                }))
                .into(),
            Label::default()
                .text_signal(battery.signal_ref(|b| {
                    b.as_ref()
                        .map(|b| format!("{}%", b.capacity))
                        .unwrap_or_default()
                }))
                .into(),
        ])
        .into()
}

fn setting_button(sub_menu: Mutable<Option<SubMenu>>, child: Node) -> Node {
    let is_open = sub_menu.signal_ref(|m| m.is_some());
    let is_open1 = sub_menu.signal_ref(|m| m.is_some());

    Overlay::default()
        .class_signal(is_open.map(|v| if v { vec!["disabled"] } else { vec![""] }))
        .children(vec![
            child,
            Box::default()
                .visible_signal(is_open1)
                .on_click(move || {
                    sub_menu.replace(None);
                })
                .hexpand(true)
                .vexpand(true)
                .into(),
        ])
        .into()
}

fn setting_slider(
    sub_menu: Mutable<Option<SubMenu>>,
    icon: impl Signal<Item = impl AsStr + Clone> + 'static,
    range: (f64, f64),
    value: impl Signal<Item = f64> + 'static,
    on_toggle: Option<impl Fn()>,
    on_change: impl Fn(f64) + 'static,
    on_open_details: Option<impl Fn() + 'static>,
) -> Node {
    let is_open = sub_menu.signal_ref(|m| m.is_some());
    let is_open1 = sub_menu.signal_ref(|m| m.is_some());

    let icon = icon.broadcast();

    Overlay::default()
        .class_signal(is_open.map(|v| if v { vec!["disabled"] } else { vec![""] }))
        .children(vec![
            Box::default()
                .orientation(Orientation::Horizontal)
                .spacing(8)
                .children(vec![
                    Button::default()
                        .class(&["rounded-full"])
                        .child(Label::default().text_signal(icon.signal_cloned()))
                        .visible(on_toggle.is_some())
                        .into(),
                    Label::default()
                        .text_signal(icon.signal_cloned())
                        .visible(on_toggle.is_none())
                        .into(),
                    Scale::default()
                        .hexpand(true)
                        .range(range)
                        .value_signal(value)
                        .on_change(on_change)
                        .round_digits(0)
                        .into(),
                    Button::default()
                        .visible(on_open_details.is_some())
                        .class(&["rounded-full"])
                        .on_click(move || {
                            if let Some(on_open_details) = &on_open_details {
                                on_open_details();
                            }
                        })
                        .child(Label::default().text("󰁔"))
                        .into(),
                ])
                .into(),
            Box::default()
                .visible_signal(is_open1)
                .on_click(move || {
                    sub_menu.replace(None);
                })
                .hexpand(true)
                .vexpand(true)
                .into(),
        ])
        .into()
}

fn menu_card_item(icon: &str, content: &str, on_click: impl Fn() + 'static) -> Node {
    Button::default()
        .class(&["transparent", "rounded-s", "ph-6", "pv-2"])
        .on_click(on_click)
        .child(Box::default().spacing(8).children(vec![
            Label::default().text(icon).into(),
            Label::default().text(content).into(),
        ]))
        .into()
}

fn menu_card(
    visible: impl Signal<Item = bool> + 'static,
    icon: &str,
    title: &str,
    children: Vec<Node>,
) -> Node {
    Box::default()
        .visible_signal(visible)
        .class(&["bg", "p-5", "rounded-m"])
        .hexpand(true)
        .orientation(Orientation::Vertical)
        .children(vec![
            Box::default()
                .class(&["mb-2"])
                .spacing(12)
                .children(vec![
                    Box::default()
                        .class(&["rounded-full", "bg-light"])
                        .homogeneous(true)
                        .size((35, 35))
                        .children(vec![Label::default().text(icon).into()])
                        .into(),
                    Label::default()
                        .class(&["text-l", "text-bold"])
                        .text(title)
                        .into(),
                ])
                .into(),
            Box::default()
                .orientation(Orientation::Vertical)
                .children(children)
                .into(),
        ])
        .into()
}

fn menu_card_dynamic(
    visible: impl Signal<Item = bool> + 'static,
    icon: &str,
    title: &str,
    children: impl Signal<Item = Vec<Node>> + 'static,
) -> Node {
    Box::default()
        .visible_signal(visible)
        .class(&["bg", "p-5", "rounded-m"])
        .hexpand(true)
        .orientation(Orientation::Vertical)
        .children(vec![
            Box::default()
                .class(&["mb-2"])
                .spacing(12)
                .children(vec![
                    Box::default()
                        .class(&["rounded-full", "bg-light"])
                        .homogeneous(true)
                        .size((35, 35))
                        .children(vec![Label::default().text(icon).into()])
                        .into(),
                    Label::default()
                        .class(&["text-l", "text-bold"])
                        .text(title)
                        .into(),
                ])
                .into(),
            Box::default()
                .orientation(Orientation::Vertical)
                .children_signal(children)
                .into(),
        ])
        .into()
}
