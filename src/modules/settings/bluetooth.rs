use guido::prelude::*;

use crate::components::{
    ButtonHierarchy, ButtonKind, ButtonSize, IconKind, StaticIcon, buttons::icon_button,
    quick_setting, selectable_item,
};
use crate::services::bluetooth::{BluetoothCommand, BluetoothService, BluetoothState};
use crate::services::compat::ServiceSignal;
use crate::theme::ThemeColors;

/// Bluetooth quick setting tile
pub fn bt_quick_setting(
    data: ServiceSignal<BluetoothService>,
    svc: Service<BluetoothCommand>,
    on_submenu: impl Fn() + 'static,
    expanded: impl Fn() -> bool + 'static,
) -> impl Widget {
    let svc_toggle = svc.clone();

    quick_setting()
        .kind(move || {
            let connected =
                data.with(|s| s.as_ref().is_some_and(|x| x.devices.iter().any(|d| d.connected)));
            if connected {
                StaticIcon::BluetoothConnected
            } else {
                StaticIcon::Bluetooth
            }
        })
        .title(move || "Bluetooth".to_string())
        .subtitle(move || {
            let connected: Vec<_> = data.with(|s| {
                s.as_ref()
                    .map(|x| {
                        x.devices
                            .iter()
                            .filter(|d| d.connected)
                            .map(|d| d.name.clone())
                            .collect()
                    })
                    .unwrap_or_default()
            });
            match connected.len() {
                0 => String::new(),
                1 => connected[0].clone(),
                n => format!("{n} devices"),
            }
        })
        .active(move || {
            data.with(|s| s.as_ref().map(|x| x.state.clone())) == Some(BluetoothState::Active)
        })
        .on_toggle(move || svc_toggle.send(BluetoothCommand::Toggle))
        .on_submenu(on_submenu)
        .expanded(expanded)
}

/// Bluetooth submenu: device list
pub fn bt_submenu(
    data: ServiceSignal<BluetoothService>,
    svc: Service<BluetoothCommand>,
) -> impl Widget {
    let theme = expect_context::<ThemeColors>();
    let discovering = move || data.with(|s| s.as_ref().is_some_and(|x| x.discovering));

    container()
        .width(fill())
        .layout(Flex::column().spacing(4))
        // Header: title + scanning status + scan button
        .child(
            container()
                .width(fill())
                .layout(
                    Flex::row()
                        .main_alignment(MainAlignment::SpaceBetween)
                        .cross_alignment(CrossAlignment::Center),
                )
                .child(
                    container()
                        .layout(
                            Flex::row()
                                .spacing(8)
                                .cross_alignment(CrossAlignment::Center),
                        )
                        .child(text("Bluetooth Devices").color(theme.text).font_size(14))
                        .child(move || {
                            if discovering() {
                                Some(
                                    text("Scanning...")
                                        .color(Color::rgba(1.0, 1.0, 1.0, 0.5))
                                        .font_size(11),
                                )
                            } else {
                                None
                            }
                        }),
                )
                .child({
                    let svc_scan = svc.clone();
                    icon_button()
                        .icon(move || -> IconKind {
                            if discovering() {
                                StaticIcon::Close
                            } else {
                                StaticIcon::Refresh
                            }
                            .into()
                        })
                        .size(ButtonSize::Small)
                        .kind(ButtonKind::Solid)
                        .on_click(move || {
                            // Upstream discovery auto-stops after 15s; while
                            // scanning the button is inert
                            if !discovering() {
                                svc_scan.send(BluetoothCommand::StartDiscovery);
                            }
                        })
                }),
        )
        // Device list
        .child(move || {
            let device_list =
                data.with(|s| s.as_ref().map(|x| x.devices.clone()).unwrap_or_default());
            let mut col = container()
                .width(fill())
                .height(at_most(250))
                .scrollable(ScrollAxis::Vertical)
                .layout(Flex::column().spacing(2));

            if device_list.is_empty() {
                return Some(
                    col.child(
                        container()
                            .padding(8)
                            .child(text("No devices found").color(theme.text).font_size(12)),
                    ),
                );
            }

            // Sort into categories
            let (mut connected, rest): (Vec<_>, Vec<_>) =
                device_list.into_iter().partition(|d| d.connected);
            let (mut paired, mut discovered): (Vec<_>, Vec<_>) =
                rest.into_iter().partition(|d| d.paired);

            connected.sort_by(|a, b| a.name.cmp(&b.name));
            paired.sort_by(|a, b| a.name.cmp(&b.name));
            discovered.sort_by(|a, b| a.name.cmp(&b.name));

            // Connected devices: green text, battery info, remove button
            for device in connected {
                let label = device
                    .battery
                    .map(|b| format!("{} ({b}%)", device.name))
                    .unwrap_or_else(|| device.name.clone());
                let path = device.path.clone();
                let remove_path = device.path.clone();
                let svc_disconnect = svc.clone();
                let svc_remove = svc.clone();
                col = col.child(
                    selectable_item()
                        .kind(StaticIcon::BluetoothConnected)
                        .label(label)
                        .selected(true)
                        .on_click(move || {
                            svc_disconnect.send(BluetoothCommand::DisconnectDevice(path.clone()));
                        })
                        .trailing(remove_button(svc_remove, remove_path)),
                );
            }

            // Paired but not connected
            for device in paired {
                let name = device.name.clone();
                let path = device.path.clone();
                let remove_path = device.path.clone();
                let svc_connect = svc.clone();
                let svc_remove = svc.clone();
                col = col.child(
                    selectable_item()
                        .kind(StaticIcon::Bluetooth)
                        .label(name)
                        .selected(false)
                        .on_click(move || {
                            svc_connect.send(BluetoothCommand::ConnectDevice(path.clone()));
                        })
                        .trailing(remove_button(svc_remove, remove_path)),
                );
            }

            // Discovered (unpaired) devices
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
                                svc.send(BluetoothCommand::PairDevice(path.clone()));
                            }),
                    );
                }
            }
            Some(col)
        })
}

fn remove_button(svc: Service<BluetoothCommand>, path: zbus::zvariant::OwnedObjectPath) -> impl Widget {
    icon_button()
        .icon(StaticIcon::Remove)
        .size(ButtonSize::Small)
        .kind(ButtonKind::Transparent)
        .hierarchy(ButtonHierarchy::Danger)
        .on_click(move || {
            svc.send(BluetoothCommand::RemoveDevice(path.clone()));
        })
}
