use guido::prelude::*;

use crate::components::{StaticIcon, icon, quick_setting};
use crate::services::bluetooth::{
    BluetoothCmd, BluetoothDataSignals, BluetoothState,
};
use crate::theme::ThemeColors;

/// Bar indicator: Bluetooth icon
pub fn bt_indicator(data: BluetoothDataSignals) -> impl Widget {
    let theme = expect_context::<ThemeColors>();
    let state = data.state;
    let devices = data.devices;

    container()
        .child(move || {
            match state.get() {
                BluetoothState::Unavailable => None,
                _ => {
                    let connected = devices.with(|d| d.iter().any(|d| d.connected));
                    Some(
                        icon(if connected {
                            StaticIcon::BluetoothConnected
                        } else {
                            StaticIcon::Bluetooth
                        })
                        .color(theme.text)
                        .font_size(14.0),
                    )
                }
            }
        })
}

/// Bluetooth quick setting tile
pub fn bt_quick_setting(
    data: BluetoothDataSignals,
    svc: Service<BluetoothCmd>,
    on_submenu: impl Fn() + 'static,
) -> impl Widget {
    let state = data.state;
    let devices = data.devices;
    let svc_toggle = svc.clone();

    quick_setting(
        move || {
            let connected = devices.with(|d| d.iter().any(|d| d.connected));
            if connected {
                StaticIcon::BluetoothConnected
            } else {
                StaticIcon::Bluetooth
            }
        },
        move || "Bluetooth".to_string(),
        move || {
            let connected_count = devices.with(|d| d.iter().filter(|d| d.connected).count());
            if connected_count > 0 {
                format!("{connected_count} connected")
            } else if state.get() == BluetoothState::Active {
                "On".to_string()
            } else {
                "Off".to_string()
            }
        },
        move || state.get() == BluetoothState::Active,
        move || svc_toggle.send(BluetoothCmd::Toggle),
        Some(on_submenu),
    )
}

/// Bluetooth submenu: device list
pub fn bt_submenu(
    data: BluetoothDataSignals,
    svc: Service<BluetoothCmd>,
) -> impl Widget {
    let theme = expect_context::<ThemeColors>();
    let devices = data.devices;
    let discovering = data.discovering;

    container()
        .width(fill())
        .layout(Flex::column().spacing(4.0))
        .child(
            container()
                .width(fill())
                .layout(
                    Flex::row()
                        .main_alignment(MainAlignment::SpaceBetween)
                        .cross_alignment(CrossAlignment::Center),
                )
                .child(text("Bluetooth Devices").color(theme.text).font_size(14.0))
                .child({
                    let svc_scan = svc.clone();
                    let hovered = create_signal(false);
                    container()
                        .padding([4.0, 8.0])
                        .corner_radius(6.0)
                        .on_hover(move |h| hovered.set(h))
                        .on_click(move || {
                            if discovering.get() {
                                svc_scan.send(BluetoothCmd::StopDiscovery);
                            } else {
                                svc_scan.send(BluetoothCmd::StartDiscovery);
                            }
                        })
                        .background(move || {
                            if hovered.get() {
                                Color::rgba(1.0, 1.0, 1.0, 0.1)
                            } else {
                                Color::TRANSPARENT
                            }
                        })
                        .child(
                            icon(move || {
                                if discovering.get() {
                                    StaticIcon::Close
                                } else {
                                    StaticIcon::Refresh
                                }
                            })
                            .color(theme.text)
                            .font_size(12.0),
                        )
                }),
        )
        .child(move || {
            let device_list = devices.with(|d| d.clone());
            let mut col = container()
                .width(fill())
                .height(at_most(250.0))
                .scrollable(ScrollAxis::Vertical)
                .layout(Flex::column().spacing(2.0));

            if device_list.is_empty() {
                return Some(col.child(
                    container()
                        .padding(8.0)
                        .child(text("No devices found").color(theme.text).font_size(12.0)),
                ));
            }

            // Connected devices first
            let mut connected: Vec<_> = device_list.iter().filter(|d| d.connected).collect();
            let mut available: Vec<_> = device_list.iter().filter(|d| !d.connected && d.paired).collect();
            let mut discovered: Vec<_> = device_list.iter().filter(|d| !d.connected && !d.paired).collect();

            connected.sort_by(|a, b| a.name.cmp(&b.name));
            available.sort_by(|a, b| a.name.cmp(&b.name));
            discovered.sort_by(|a, b| a.name.cmp(&b.name));

            for device in connected {
                let name = device.name.clone();
                let path = device.path.clone();
                let battery_str = device
                    .battery
                    .map(|b| format!(" ({b}%)"))
                    .unwrap_or_default();
                let label = format!("{name}{battery_str}");
                let svc = svc.clone();
                let hovered = create_signal(false);
                col = col.child(
                    container()
                        .width(fill())
                        .padding([6.0, 8.0])
                        .corner_radius(8.0)
                        .on_hover(move |h| hovered.set(h))
                        .on_click(move || {
                            svc.send(BluetoothCmd::DisconnectDevice(path.clone()));
                        })
                        .background(move || {
                            if hovered.get() {
                                Color::rgba(1.0, 1.0, 1.0, 0.1)
                            } else {
                                Color::rgba(1.0, 1.0, 1.0, 0.15)
                            }
                        })
                        .layout(
                            Flex::row()
                                .spacing(8.0)
                                .cross_alignment(CrossAlignment::Center),
                        )
                        .child(
                            icon(StaticIcon::BluetoothConnected)
                                .color(theme.text)
                                .font_size(14.0),
                        )
                        .child(text(label).color(theme.text).font_size(12.0)),
                );
            }

            for device in available {
                let name = device.name.clone();
                let path = device.path.clone();
                let svc = svc.clone();
                let hovered = create_signal(false);
                col = col.child(
                    container()
                        .width(fill())
                        .padding([6.0, 8.0])
                        .corner_radius(8.0)
                        .on_hover(move |h| hovered.set(h))
                        .on_click(move || {
                            svc.send(BluetoothCmd::ConnectDevice(path.clone()));
                        })
                        .background(move || {
                            if hovered.get() {
                                Color::rgba(1.0, 1.0, 1.0, 0.1)
                            } else {
                                Color::TRANSPARENT
                            }
                        })
                        .layout(
                            Flex::row()
                                .spacing(8.0)
                                .cross_alignment(CrossAlignment::Center),
                        )
                        .child(icon(StaticIcon::Bluetooth).color(theme.text).font_size(14.0))
                        .child(text(name).color(theme.text).font_size(12.0)),
                );
            }

            if !discovered.is_empty() {
                col = col.child(
                    container()
                        .padding([4.0, 8.0])
                        .child(text("Available").color(theme.primary).font_size(11.0)),
                );
                for device in discovered {
                    let name = device.name.clone();
                    let path = device.path.clone();
                    let svc = svc.clone();
                    let hovered = create_signal(false);
                    col = col.child(
                        container()
                            .width(fill())
                            .padding([6.0, 8.0])
                            .corner_radius(8.0)
                            .on_hover(move |h| hovered.set(h))
                            .on_click(move || {
                                svc.send(BluetoothCmd::PairDevice(path.clone()));
                            })
                            .background(move || {
                                if hovered.get() {
                                    Color::rgba(1.0, 1.0, 1.0, 0.1)
                                } else {
                                    Color::TRANSPARENT
                                }
                            })
                            .layout(
                                Flex::row()
                                    .spacing(8.0)
                                    .cross_alignment(CrossAlignment::Center),
                            )
                            .child(
                                icon(StaticIcon::Bluetooth).color(theme.text).font_size(14.0),
                            )
                            .child(text(name).color(theme.text).font_size(12.0)),
                    );
                }
            }
            Some(col)
        })
}
