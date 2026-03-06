use guido::prelude::*;

use crate::components::{IconKind, StaticIcon, icon, quick_setting, selectable_item};
use crate::services::bluetooth::{
    BluetoothCmd, BluetoothDataSignals, BluetoothState,
};
use crate::theme::ThemeColors;

/// Bluetooth quick setting tile
pub fn bt_quick_setting(
    data: BluetoothDataSignals,
    svc: Service<BluetoothCmd>,
    on_submenu: impl Fn() + 'static,
    expanded: impl Fn() -> bool + 'static,
) -> impl Widget {
    let state = data.state;
    let devices = data.devices;
    let svc_toggle = svc.clone();

    quick_setting()
        .kind(move || {
            let connected = devices.with(|d| d.iter().any(|d| d.connected));
            if connected {
                StaticIcon::BluetoothConnected
            } else {
                StaticIcon::Bluetooth
            }
        })
        .title(move || "Bluetooth".to_string())
        .subtitle(move || {
            let connected_count = devices.with(|d| d.iter().filter(|d| d.connected).count());
            if connected_count > 0 {
                format!("{connected_count} connected")
            } else if state.get() == BluetoothState::Active {
                "On".to_string()
            } else {
                "Off".to_string()
            }
        })
        .active(move || state.get() == BluetoothState::Active)
        .on_toggle(move || svc_toggle.send(BluetoothCmd::Toggle(state.get())))
        .on_submenu(on_submenu)
        .expanded(expanded)
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
        .layout(Flex::column().spacing(4))
        .child(
            container()
                .width(fill())
                .layout(
                    Flex::row()
                        .main_alignment(MainAlignment::SpaceBetween)
                        .cross_alignment(CrossAlignment::Center),
                )
                .child(text("Bluetooth Devices").color(theme.text).font_size(14))
                .child({
                    let svc_scan = svc.clone();
                    let hovered = create_signal(false);
                    container()
                        .padding([4, 8])
                        .corner_radius(6)
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
                            icon().kind(move || -> IconKind {
                                if discovering.get() {
                                    StaticIcon::Close
                                } else {
                                    StaticIcon::Refresh
                                }
                                .into()
                            })
                            .color(theme.text)
                            .font_size(12),
                        )
                }),
        )
        .child(move || {
            let device_list = devices.with(|d| d.clone());
            let mut col = container()
                .width(fill())
                .height(at_most(250))
                .scrollable(ScrollAxis::Vertical)
                .layout(Flex::column().spacing(2));

            if device_list.is_empty() {
                return Some(col.child(
                    container()
                        .padding(8)
                        .child(text("No devices found").color(theme.text).font_size(12)),
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
                col = col.child(
                    selectable_item()
                        .kind(StaticIcon::BluetoothConnected)
                        .label(label)
                        .selected(true)
                        .on_click(move || {
                            svc.send(BluetoothCmd::DisconnectDevice(path.clone()));
                        }),
                );
            }

            for device in available {
                let name = device.name.clone();
                let path = device.path.clone();
                let svc = svc.clone();
                col = col.child(
                    selectable_item()
                        .kind(StaticIcon::Bluetooth)
                        .label(name)
                        .selected(false)
                        .on_click(move || {
                            svc.send(BluetoothCmd::ConnectDevice(path.clone()));
                        }),
                );
            }

            if !discovered.is_empty() {
                col = col.child(
                    container()
                        .padding([4, 8])
                        .child(text("Available").color(theme.primary).font_size(11)),
                );
                for device in discovered {
                    let name = device.name.clone();
                    let path = device.path.clone();
                    let svc = svc.clone();
                    col = col.child(
                        selectable_item()
                            .kind(StaticIcon::Bluetooth)
                            .label(name)
                            .selected(false)
                            .on_click(move || {
                                svc.send(BluetoothCmd::PairDevice(path.clone()));
                            }),
                    );
                }
            }
            Some(col)
        })
}
